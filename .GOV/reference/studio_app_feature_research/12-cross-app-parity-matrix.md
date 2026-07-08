---
file_id: studio-app-feature-research-parity-matrix
topic_id: SFR-PARITY-MATRIX
title: "Cross-App Studio Parity Matrix"
status: draft
summary: "Primitive-centered parity matrix mapping Photoshop, Affinity, and InDesign feature families/leaves to Handshake Studio implementation surfaces."
sources: 7
updated_at: "2026-07-05"
---

## [SFR-PARITY-MATRIX] Cross-App Studio Parity Matrix

### [SFR-PARITY-MATRIX.matrix] Primitive-Centered Parity

```yaml
parity_matrix:
  - parity_id: "parity.layer_graph.core"
    studio_primitive: "StudioLayerGraph"
    engine_module: "studio_layer_graph"
    photoshop_refs: ["photoshop.layer.stack", "photoshop.layer.adjustment", "photoshop.layer.fill", "photoshop.layer.smart_objects", "photoshop.layer.effects_styles"]
    affinity_refs: ["affinity_photo.live_filters_adjustments", "affinity_photo.saved_layer_states", "affinity_publisher.linked_file_layer_visibility_override", "affinity_desktop_delta.layer_records"]
    indesign_refs: ["indesign.layers", "indesign.object_styles", "indesign.links_panel"]
    parity_target: "One shared graph for raster, vector, text, layout, adjustment, mask, smart/linked, and generated layers."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-001", "SFR-GAP-003"]
    verification: ["layer_tree_snapshot", "undo_redo_roundtrip", "visual_hash_preserved_on_group"]
  - parity_id: "parity.raster.live_operations"
    studio_primitive: "StudioRasterPipeline"
    engine_module: "studio_raster"
    photoshop_refs: ["photoshop.filter.smart_filters", "photoshop.retouch.remove_tool", "photoshop.retouch.content_aware_fill", "photoshop.paint.brush_engine"]
    affinity_refs: ["affinity_photo.live_filters_adjustments", "affinity_photo.inpainting_patch_heal", "affinity_photo.liquify_persona", "affinity_photo.frequency_separation"]
    indesign_refs: ["indesign.express_edit"]
    parity_target: "Non-destructive raster operation stack with local filters and provider-isolated generated edits."
    implementation_posture: "mixed_local_and_provider"
    gaps_closed: ["SFR-GAP-003", "SFR-GAP-005"]
    verification: ["golden_render", "operation_receipt", "provider_denial_no_mutation"]
  - parity_id: "parity.selection_mask"
    studio_primitive: "StudioSelectionSet + StudioMaskGraph"
    engine_module: "studio_selection + studio_mask"
    photoshop_refs: ["photoshop.selection.manual_tools", "photoshop.selection.object_subject_people", "photoshop.mask.layer_mask", "photoshop.mask.vector_mask"]
    affinity_refs: ["affinity_photo.select_subject_ml", "affinity_photo.object_selection_tool_ml", "affinity_photo.compound_masks", "affinity_photo.live_masks_luminosity"]
    indesign_refs: ["indesign.frame_fitting", "indesign.text_wrap"]
    parity_target: "Selections can become masks, paths, layout clipping frames, or provider input regions."
    implementation_posture: "mixed_local_and_provider"
    gaps_closed: ["SFR-GAP-003", "SFR-GAP-005"]
    verification: ["mask_coverage", "selection_iou_fixture", "composite_preview_hash"]
  - parity_id: "parity.color_pipeline"
    studio_primitive: "StudioColorPipeline"
    engine_module: "studio_color"
    photoshop_refs: ["photoshop.color.profiles_modes", "photoshop.color.adjustments", "photoshop.color.ocio", "photoshop.camera_raw.basic_adjustments"]
    affinity_refs: ["affinity_photo.hdr_32bit_ocio", "affinity_photo.non_destructive_raw_develop"]
    indesign_refs: ["indesign.swatches_color", "indesign.separations_inks_overprint", "indesign.print_pdf_export"]
    parity_target: "Document color profile, swatches, adjustment recipes, raw development, and prepress output color checks."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-003"]
    verification: ["profile_trace", "gamut_warning", "export_color_fixture"]
  - parity_id: "parity.vector_paths"
    studio_primitive: "StudioVectorPathGraph"
    engine_module: "studio_vector"
    photoshop_refs: ["photoshop.vector.shapes", "photoshop.vector.pen_paths", "photoshop.vector.content_aware_tracing"]
    affinity_refs: ["affinity_designer.shape_builder", "affinity_designer.vector_warp", "affinity_designer.knife_scissor", "affinity_designer.vector_flood_fill"]
    indesign_refs: ["indesign.object_transform", "indesign.qr_codes"]
    parity_target: "One path graph for shape layers, vector clipping, layout objects, QR/vector output, booleans, warps, and export."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-003"]
    verification: ["path_topology", "svg_pdf_export", "boolean_golden"]
  - parity_id: "parity.typography_story"
    studio_primitive: "StudioTextRunAndStory"
    engine_module: "studio_typography"
    photoshop_refs: ["photoshop.typography.text_layers", "photoshop.typography.fonts_opentype", "photoshop.typography.text_on_path", "photoshop.typography.international_text"]
    affinity_refs: ["affinity_designer.text_on_path", "affinity_publisher.footnotes_endnotes_sidenotes", "affinity_publisher.smart_master_pages_text_styles"]
    indesign_refs: ["indesign.text_frames", "indesign.threaded_text", "indesign.fonts_opentype", "indesign.cjk_text", "indesign.footnotes_endnotes"]
    parity_target: "Unified text layer/story model with font resolution, shaping, styles, overset detection, and layout reflow."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-003"]
    verification: ["font_resolution", "shaping_trace", "overset_text", "story_edit_roundtrip"]
  - parity_id: "parity.page_layout"
    studio_primitive: "StudioPageSpread"
    engine_module: "studio_layout"
    photoshop_refs: ["photoshop.layout.artboards_frames"]
    affinity_refs: ["affinity_publisher.books", "affinity_publisher.place_autoflow", "affinity_publisher.picture_frames_tables_data_merge", "affinity_publisher.smart_master_pages_text_styles"]
    indesign_refs: ["indesign.document_setup", "indesign.pages_spreads", "indesign.parent_pages", "indesign.book_files", "indesign.flex_layout"]
    parity_target: "Page/spread/frame graph for artboards, publication pages, parent pages, books, flexible layouts, and placed assets."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-003"]
    verification: ["page_preview", "layout_reflow_trace", "parent_page_override_test"]
  - parity_id: "parity.tables_data"
    studio_primitive: "StudioTableFrame"
    engine_module: "studio_tables"
    photoshop_refs: []
    affinity_refs: ["affinity_publisher.picture_frames_tables_data_merge"]
    indesign_refs: ["indesign.tables", "indesign.table_cell_styles", "indesign.data_merge"]
    parity_target: "Structured table frame and data merge workflow for layout documents."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-003"]
    verification: ["table_overflow", "style_resolution", "data_merge_fixture"]
  - parity_id: "parity.export_prepress"
    studio_primitive: "StudioExportRecipe + StudioPreflightProfile"
    engine_module: "studio_export + studio_prepress"
    photoshop_refs: ["photoshop.export.export_as_quick_export", "photoshop.export.layers_artboards", "photoshop.export.content_credentials"]
    affinity_refs: ["affinity_designer.export_persona_slices", "affinity_photo.jpeg_xl_import_export"]
    indesign_refs: ["indesign.print_pdf_export", "indesign.preflight", "indesign.package_output", "indesign.accessible_pdf", "indesign.epub_export"]
    parity_target: "Export recipes with deterministic artifacts, preflight gates, metadata policy, and package manifests."
    implementation_posture: "mixed_local_and_shim"
    gaps_closed: ["SFR-GAP-003", "SFR-GAP-005"]
    verification: ["export_manifest", "pdf_preflight", "content_credentials_roundtrip", "package_manifest"]
  - parity_id: "parity.automation_agents"
    studio_primitive: "StudioActionGraph"
    engine_module: "studio_automation"
    photoshop_refs: ["photoshop.automation.actions", "photoshop.automation.batch_droplets", "photoshop.automation.uxp"]
    affinity_refs: ["affinity_photo.macros_batch_jobs"]
    indesign_refs: ["indesign.scripting_panels", "indesign.uxp_dom", "indesign.event_scripting", "indesign.server_automation"]
    parity_target: "Typed action graphs, dry runs, rollback plans, batch receipts, and model-visible operation traces."
    implementation_posture: "local_primitive"
    gaps_closed: ["SFR-GAP-003"]
    verification: ["dry_run_diff", "batch_receipts", "rollback_test", "capability_denial"]
  - parity_id: "parity.ai_provider_tools"
    studio_primitive: "StudioModelToolContract"
    engine_module: "studio_model_tools"
    photoshop_refs: ["photoshop.ai.generate_image", "photoshop.ai.generative_fill", "photoshop.ai.harmonize", "photoshop.ai.generative_upscale"]
    affinity_refs: ["affinity_photo.select_subject_ml", "affinity_photo.object_selection_tool_ml"]
    indesign_refs: ["indesign.ai_rewrite", "indesign.ai_alt_text", "indesign.ai_text_to_image", "indesign.ai_generative_expand"]
    parity_target: "Provider/local model tools behind typed request, receipt, fallback, and policy boundaries."
    implementation_posture: "provider_adapter"
    gaps_closed: ["SFR-GAP-005"]
    verification: ["provider_mock", "policy_denial", "offline_fallback", "receipt_redaction"]
```

### [SFR-PARITY-MATRIX.workflow] Parallel-Agent Workflow

```yaml
parallel_workflow:
  work_unit: "One parity_id is a natural work packet seed."
  assignment_rule: "Do not split a single parity_id across agents until its command contracts and validation fixtures are named."
  required_inputs: ["vendor_refs", "studio_primitive", "provider_posture", "verification"]
  required_outputs: ["command_contracts", "test_fixtures", "diagnostics", "updated parity status"]
  status_enum: ["research", "contracted", "implemented", "verified", "deferred", "omitted"]
```

### [SFR-PARITY-MATRIX.roi] High-ROI Additions

```yaml
high_roi_additions:
  - item: "Use parity_id as work-packet seed IDs."
    why_high_roi: "Turns research rows into parallel-agent implementation lanes without inventing new names."
    gap_closed: "Improves operator and LLM parallel workflow."
    reuse: "Existing stable IDs in this file and source feature maps."
    validation: "Every generated work packet links to one parity_id."
  - item: "Track implementation_posture at parity row level."
    why_high_roi: "Keeps provider-backed and local-native work from being mixed."
    gap_closed: "Reduces rework and scope confusion."
    reuse: "Provider posture map."
    validation: "No parity row remains posture-empty."
  - item: "Require verification per parity row before coding."
    why_high_roi: "Prevents broad feature claims from becoming untestable build scope."
    gap_closed: "Closes command-contract and primitive-build gaps."
    reuse: "Existing Handshake EventLedger, Flight Recorder, and diagnostics patterns."
    validation: "Each parity row has at least one deterministic test or explicit provider mock."
```

### [SFR-PARITY-MATRIX.sources] Sources

```yaml
sources:
  - { id: PM-S01, path: "01-photoshop-feature-map.md", note: "Photoshop category feature IDs." }
  - { id: PM-S02, path: "02-affinity-suite-feature-map.md", note: "Affinity category feature IDs." }
  - { id: PM-S03, path: "03-indesign-feature-map.md", note: "InDesign category feature IDs." }
  - { id: PM-S04, path: "05-studio-primitive-map.md", note: "Studio primitive names and engine modules." }
  - { id: PM-S05, path: "09-affinity-desktop-delta.md", note: "Affinity desktop delta rows." }
  - { id: PM-S06, path: "10-studio-command-contracts.md", note: "Command contract schema and seed contracts." }
  - { id: PM-S07, path: "11-provider-posture-map.md", note: "Provider posture classifications." }
```
