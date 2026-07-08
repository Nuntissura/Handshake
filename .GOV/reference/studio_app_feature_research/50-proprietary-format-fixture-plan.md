---
file_id: 50-proprietary-format-fixture-plan
file_kind: proprietary_format_fixture_plan
topic_id: SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN
title: Proprietary Format Fixture Plan
status: draft
summary: Generated fixture and receipt plan for native, proprietary, local-copy, and
  round-trip source creative formats that Studio must preserve without inventing a
  replacement interchange format.
updated_at: '2026-07-05'
format_fixture_plan_count: 15
source_format_family_count: 38
---

## [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN] Proprietary Format Fixture Plan

<topic id="format-fixture-summary" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage summary for proprietary/native format fixture planning.">

### [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN.summary] Fixture Plan Summary

```yaml
format_fixture_plan_summary:
  format_fixture_plan_count: 15
  source_format_family_count: 38
  source_compatibility_record_count: 410
  target_selection_rule: format families with compatibility_posture native_round_trip_target
  format_fixture_targets_by_app:
    affinity: 5
    figma: 6
    illustrator: 3
    indesign: 2
    photoshop: 2
  schema_public_status_counts:
    documented_interchange_xml_with_source_behavior_fixtures: 1
    partly_documented_large_native_document: 1
    partly_documented_native_document: 1
    vendor_private_local_copy_document: 6
    vendor_private_native_document: 4
    vendor_private_or_pdf_compatible_native_document: 1
    vendor_private_template_document: 1
  support_direction_counts:
    edit_preserve: 15
    export: 15
    import: 15
    round_trip: 15
  policy:
    compatibility_rule: Preserve compatibility with source creative formats through fixture-driven import, export, edit-preserve, and round-trip contracts.
    no_new_interchange_rule: Do not invent a replacement interchange format for Studio parity scope.
    private_schema_rule: Undocumented vendor-private structures are handled through fixtures, preservation blobs, explicit unsupported-feature receipts, and lossy-conversion diagnostics.
    claim_rule: A format is not parity-complete until representative fixtures pass and unsupported features are documented in receipts and the Studio UserManual.
```

</topic>

<topic id="format-fixture-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable fixture plan rows for native and proprietary format targets.">

### [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN.rows] Fixture Plan Rows

```yaml
format_fixture_plan_rows:
- fixture_plan_id: format-fixture.format-afdesign.v1
  format_id: format.afdesign
  format_labels:
  - AFDESIGN
  source_apps_present:
  - affinity
  schema_public_status: vendor_private_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    affinity:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - vector
  - layer
  - color
  - typography
  - export
  rust_implementation_lanes:
  - studio_vector
  - studio_layer
  - studio_color
  - studio_typography
  - studio_export
  fixture_families:
  - affinity_live_adjustment_stack
  - components_instances_variables
  - empty_minimal_document
  - gradients_patterns_appearances
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - paths_shapes_booleans_symbols
  - studiolink_persona_state
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / AFDESIGN
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.affinity.aff-domain-compatibility_and_formats.v1
  - compat.native.affinity.format-afdesign.v1
- fixture_plan_id: format-fixture.format-afphoto.v1
  format_id: format.afphoto
  format_labels:
  - AFPHOTO
  source_apps_present:
  - affinity
  schema_public_status: vendor_private_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    affinity:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - raster
  - layer
  - mask
  - color
  - raw
  - export
  rust_implementation_lanes:
  - studio_raster
  - studio_layer
  - studio_mask
  - studio_color
  - studio_raw
  - studio_export
  fixture_families:
  - affinity_live_adjustment_stack
  - bit_depth_hdr_and_transparency
  - empty_minimal_document
  - layers_masks_adjustments
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - smart_or_live_filters
  - studiolink_persona_state
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / AFPHOTO
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.affinity.aff-domain-compatibility_and_formats.v1
  - compat.native.affinity.format-afphoto.v1
- fixture_plan_id: format-fixture.format-afpub.v1
  format_id: format.afpub
  format_labels:
  - AFPUB
  source_apps_present:
  - affinity
  schema_public_status: vendor_private_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    affinity:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - page_layout
  - typography
  - tables
  - prepress
  - export
  rust_implementation_lanes:
  - studio_page_layout
  - studio_typography
  - studio_tables
  - studio_prepress
  - studio_export
  fixture_families:
  - affinity_live_adjustment_stack
  - empty_minimal_document
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - pages_spreads_masters
  - preflight_bleed_package_pdf
  - studiolink_persona_state
  - text_font_and_missing_font_cases
  - threaded_text_tables_footnotes
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / AFPUB
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.affinity.aff-domain-compatibility_and_formats.v1
  - compat.native.affinity.format-afpub.v1
- fixture_plan_id: format-fixture.format-ai.v1
  format_id: format.ai
  format_labels:
  - AI
  source_apps_present:
  - affinity
  - illustrator
  schema_public_status: vendor_private_or_pdf_compatible_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    affinity:
    - fixture_required
    illustrator:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - vector
  - typography
  - color
  - layer
  - export
  rust_implementation_lanes:
  - studio_vector
  - studio_typography
  - studio_color
  - studio_layer
  - studio_export
  fixture_families:
  - affinity_live_adjustment_stack
  - components_instances_variables
  - effects_appearance_expansion
  - empty_minimal_document
  - gradients_patterns_appearances
  - legacy_ai_version_fixture
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - paths_shapes_booleans_symbols
  - pdf_compatible_ai_toggle
  - studiolink_persona_state
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / AI
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.affinity.aff-domain-compatibility_and_formats.v1
  - compat.domain.illustrator.ail-domain-file_io_export_prepress.v1
  - compat.native.illustrator.format-ai.v1
- fixture_plan_id: format-fixture.format-ait.v1
  format_id: format.ait
  format_labels:
  - AIT
  source_apps_present:
  - illustrator
  schema_public_status: vendor_private_template_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    illustrator:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - vector
  - typography
  - style_system
  - export
  rust_implementation_lanes:
  - studio_vector
  - studio_typography
  - studio_style_system
  - studio_export
  fixture_families:
  - components_instances_variables
  - effects_appearance_expansion
  - empty_minimal_document
  - gradients_patterns_appearances
  - legacy_ai_version_fixture
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - paths_shapes_booleans_symbols
  - pdf_compatible_ai_toggle
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / AIT
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.illustrator.ail-domain-file_io_export_prepress.v1
  - compat.native.illustrator.format-ait.v1
- fixture_plan_id: format-fixture.format-buzz.v1
  format_id: format.buzz
  format_labels:
  - BUZZ local copy
  source_apps_present:
  - figma
  schema_public_status: vendor_private_local_copy_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - brand_assets
  - design_systems
  - export
  rust_implementation_lanes:
  - studio_brand_assets
  - studio_design_systems
  - studio_export
  fixture_families:
  - components_instances_variables
  - empty_minimal_document
  - gradients_patterns_appearances
  - library_component_detachment
  - linked_and_embedded_assets
  - local_copy_version_skew
  - metadata_and_color_profile
  - multiplayer_history_loss_probe
  - paths_shapes_booleans_symbols
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / BUZZ local copy
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.figma.fig-domain-buzz.v1
  - compat.domain.figma.fig-domain-collaboration_import_export_local_copies.v1
  - compat.native.figma.format-buzz.v1
- fixture_plan_id: format-fixture.format-deck.v1
  format_id: format.deck
  format_labels:
  - DECK local copy
  source_apps_present:
  - figma
  schema_public_status: vendor_private_local_copy_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - presentation
  - typography
  - interactive
  - export
  rust_implementation_lanes:
  - studio_presentation
  - studio_typography
  - studio_interactive
  - studio_export
  fixture_families:
  - comments_history_and_collaboration_artifacts
  - empty_minimal_document
  - export_publish_state
  - interactive_nodes_or_frames
  - library_component_detachment
  - linked_and_embedded_assets
  - local_copy_version_skew
  - metadata_and_color_profile
  - multiplayer_history_loss_probe
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / DECK local copy
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.figma.fig-domain-collaboration_import_export_local_copies.v1
  - compat.domain.figma.fig-domain-slides.v1
  - compat.native.figma.format-deck.v1
- fixture_plan_id: format-fixture.format-fig.v1
  format_id: format.fig
  format_labels:
  - FIG local copy
  source_apps_present:
  - figma
  schema_public_status: vendor_private_local_copy_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - design_systems
  - vector
  - page_layout
  - prototype
  - export
  rust_implementation_lanes:
  - studio_design_systems
  - studio_vector
  - studio_page_layout
  - studio_prototype
  - studio_export
  fixture_families:
  - components_instances_variables
  - empty_minimal_document
  - gradients_patterns_appearances
  - library_component_detachment
  - linked_and_embedded_assets
  - local_copy_version_skew
  - metadata_and_color_profile
  - multiplayer_history_loss_probe
  - pages_spreads_masters
  - paths_shapes_booleans_symbols
  - preflight_bleed_package_pdf
  - text_font_and_missing_font_cases
  - threaded_text_tables_footnotes
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / FIG local copy
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.figma.fig-domain-collaboration_import_export_local_copies.v1
  - compat.native.figma.format-fig.v1
- fixture_plan_id: format-fixture.format-idml.v1
  format_id: format.idml
  format_labels:
  - IDML
  source_apps_present:
  - indesign
  schema_public_status: documented_interchange_xml_with_source_behavior_fixtures
  compatibility_posture: native_round_trip_target
  support_by_app:
    indesign:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - page_layout
  - typography
  - tables
  - prepress
  - export
  rust_implementation_lanes:
  - studio_page_layout
  - studio_typography
  - studio_tables
  - studio_prepress
  - studio_export
  fixture_families:
  - book_and_package_dependency_fixture
  - empty_minimal_document
  - idml_vs_indd_comparison
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - pages_spreads_masters
  - preflight_bleed_package_pdf
  - text_font_and_missing_font_cases
  - threaded_text_tables_footnotes
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / IDML
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.indesign.idd-domain-import_export_publish_print.v1
  - compat.native.indesign.format-idml.v1
- fixture_plan_id: format-fixture.format-indd.v1
  format_id: format.indd
  format_labels:
  - INDD
  source_apps_present:
  - indesign
  schema_public_status: vendor_private_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    indesign:
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - page_layout
  - typography
  - tables
  - prepress
  - export
  rust_implementation_lanes:
  - studio_page_layout
  - studio_typography
  - studio_tables
  - studio_prepress
  - studio_export
  fixture_families:
  - book_and_package_dependency_fixture
  - empty_minimal_document
  - idml_vs_indd_comparison
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - pages_spreads_masters
  - preflight_bleed_package_pdf
  - text_font_and_missing_font_cases
  - threaded_text_tables_footnotes
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / INDD
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.native.indesign.format-indd.v1
- fixture_plan_id: format-fixture.format-jam.v1
  format_id: format.jam
  format_labels:
  - JAM local copy
  source_apps_present:
  - figma
  schema_public_status: vendor_private_local_copy_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - whiteboard
  - collaboration
  - file_io
  - export
  rust_implementation_lanes:
  - studio_whiteboard
  - studio_collaboration
  - studio_file_io
  - studio_export
  fixture_families:
  - comments_history_and_collaboration_artifacts
  - empty_minimal_document
  - export_publish_state
  - interactive_nodes_or_frames
  - library_component_detachment
  - linked_and_embedded_assets
  - local_copy_version_skew
  - metadata_and_color_profile
  - multiplayer_history_loss_probe
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / JAM local copy
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.figma.fig-domain-collaboration_import_export_local_copies.v1
  - compat.native.figma.format-jam.v1
- fixture_plan_id: format-fixture.format-make.v1
  format_id: format.make
  format_labels:
  - MAKE local copy
  source_apps_present:
  - figma
  schema_public_status: vendor_private_local_copy_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    figma:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - ai
  - web
  - dev_mode
  - export
  rust_implementation_lanes:
  - studio_ai
  - studio_web
  - studio_dev_mode
  - studio_export
  fixture_families:
  - code_api_or_dev_handoff_artifacts
  - comments_history_and_collaboration_artifacts
  - empty_minimal_document
  - export_publish_state
  - generated_or_provider_backed_nodes
  - interactive_nodes_or_frames
  - library_component_detachment
  - linked_and_embedded_assets
  - local_copy_version_skew
  - metadata_and_color_profile
  - multiplayer_history_loss_probe
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / MAKE local copy
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.figma.fig-domain-make.v1
  - compat.native.figma.format-make.v1
- fixture_plan_id: format-fixture.format-psb.v1
  format_id: format.psb
  format_labels:
  - PSB
  source_apps_present:
  - photoshop
  schema_public_status: partly_documented_large_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    photoshop:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - raster
  - layer
  - mask
  - color
  - export
  rust_implementation_lanes:
  - studio_raster
  - studio_layer
  - studio_mask
  - studio_color
  - studio_export
  fixture_families:
  - bit_depth_hdr_and_transparency
  - empty_minimal_document
  - layers_masks_adjustments
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - psd_psb_size_boundary
  - smart_object_round_trip_fixture
  - smart_or_live_filters
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / PSB
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.photoshop.psd-domain-document_file_io.v1
  - compat.native.photoshop.format-psb.v1
- fixture_plan_id: format-fixture.format-psd.v1
  format_id: format.psd
  format_labels:
  - PSD
  source_apps_present:
  - affinity
  - illustrator
  - photoshop
  schema_public_status: partly_documented_native_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    affinity:
    - fixture_required
    illustrator:
    - fixture_required
    photoshop:
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - raster
  - layer
  - mask
  - typography
  - color
  - export
  rust_implementation_lanes:
  - studio_raster
  - studio_layer
  - studio_mask
  - studio_typography
  - studio_color
  - studio_export
  fixture_families:
  - affinity_live_adjustment_stack
  - bit_depth_hdr_and_transparency
  - effects_appearance_expansion
  - empty_minimal_document
  - layers_masks_adjustments
  - legacy_ai_version_fixture
  - linked_and_embedded_assets
  - metadata_and_color_profile
  - pdf_compatible_ai_toggle
  - psd_psb_size_boundary
  - smart_object_round_trip_fixture
  - smart_or_live_filters
  - studiolink_persona_state
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / PSD
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.affinity.aff-domain-compatibility_and_formats.v1
  - compat.domain.illustrator.ail-domain-file_io_export_prepress.v1
  - compat.domain.photoshop.psd-domain-document_file_io.v1
  - compat.native.photoshop.format-psd.v1
- fixture_plan_id: format-fixture.format-site.v1
  format_id: format.site
  format_labels:
  - SITE local copy
  source_apps_present:
  - figma
  schema_public_status: vendor_private_local_copy_document
  compatibility_posture: native_round_trip_target
  support_by_app:
    figma:
    - export
    - fixture_required
    - round_trip
  required_support_directions:
  - import
  - edit_preserve
  - export
  - round_trip
  studio_primitive_domains:
  - web
  - interactive
  - design_systems
  - export
  rust_implementation_lanes:
  - studio_web
  - studio_interactive
  - studio_design_systems
  - studio_export
  fixture_families:
  - comments_history_and_collaboration_artifacts
  - components_instances_variables
  - empty_minimal_document
  - export_publish_state
  - gradients_patterns_appearances
  - interactive_nodes_or_frames
  - library_component_detachment
  - linked_and_embedded_assets
  - local_copy_version_skew
  - metadata_and_color_profile
  - multiplayer_history_loss_probe
  - paths_shapes_booleans_symbols
  - text_font_and_missing_font_cases
  - unsupported_feature_probe
  minimum_fixture_count_rule: at_least_one_fixture_per_fixture_family_per_supported_app_and_direction
  round_trip_assertions:
  - open_without_crash
  - preserve_document_graph_or_emit_explicit_unsupported_feature_receipt
  - preserve_visible_render_for_supported_features
  - preserve linked or embedded asset references where supported
  - preserve color profile and units where supported
  - export_or_save_with_deterministic_receipt
  - reopen_exported_output_and_compare_supported_state
  unsupported_feature_policy:
  - do_not_silently_drop_source_data
  - emit unsupported_feature_receipt with source path, feature kind, affected object ids, fallback, and recovery advice
  - keep original source blob or substructure when preservation is possible
  - mark lossy conversion in the command receipt and internal Studio UserManual topic
  receipt_required_fields:
  - format_id
  - source_app_key
  - fixture_id
  - operation_direction
  - parser_version
  - writer_version
  - preserved_features
  - converted_features
  - unsupported_features
  - dropped_features
  - render_comparison_result
  - round_trip_result
  - recovery_steps
  manual_topic_candidate: Studio / File Compatibility / SITE local copy
  implementation_readiness: needs_fixture_corpus_before_product_parity_claim
  compatibility_record_refs:
  - compat.domain.figma.fig-domain-collaboration_import_export_local_copies.v1
  - compat.domain.figma.fig-domain-sites.v1
  - compat.feature.figma.osd-figma-figma-sites-leaf-category-v1.v1
  - compat.native.figma.format-site.v1
```

</topic>

<topic id="format-fixture-sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for proprietary format fixture plan.">

### [SFR-PROPRIETARY-FORMAT-FIXTURE-PLAN.sources] Sources

```yaml
sources:
- id: FORMAT-FIXTURE-S01
  path: 46-file-format-compatibility-registry.md
  note: Source-distilled file-format compatibility registry.
- id: FORMAT-FIXTURE-S02
  path: 49-source-coverage-verification-matrix.md
  note: Coverage matrix proving source URL and local snapshot evidence for feature rows.
- id: FORMAT-FIXTURE-S03
  path: 47-studio-rust-implementation-backlog.md
  note: Implementation-facing primitive backlog.
- id: FORMAT-FIXTURE-S04
  path: _tools/generate-proprietary-format-fixture-plan.py
  note: Generator for this fixture plan.
```

</topic>
