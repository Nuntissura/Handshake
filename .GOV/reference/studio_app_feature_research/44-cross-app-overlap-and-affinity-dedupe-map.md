---
file_id: cross-app-overlap-and-affinity-dedupe-map
file_kind: source_distilled_overlap_map
topic_id: SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE
title: Cross-App Overlap and Affinity Dedupe Map
status: draft
updated_at: '2026-07-05'
feature_row_count: 2730
primitive_domain_count: 21
affinity_domain_count: 10
affinity_relation_overlay_count: 1032
affinity_exact_name_overlap_count: 6
---

## [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE] Cross-App Overlap and Affinity Dedupe Map

<topic id="overlap-policy" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Policy for source-app overlap, Affinity dedupe, and Studio implementation grouping.">

### [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE.policy] Overlap Policy

```yaml
policy:
  goal: Prevent confusing overlap while preserving every source-observable feature/tool record for Studio rebuild planning.
  core_rule: Shared capability across source apps maps to one Handshake-native Studio primitive, not duplicate Adobe/Affinity/Figma implementations.
  source_variant_rule: Each source app retains its source_distilled_feature_id, source refs, provider posture, compatibility posture, and manual
    topic candidate.
  affinity_rule: Affinity rows are never renamed as Adobe rows. Shared behavior is grouped by Studio primitive; Affinity-specific workflow variants
    remain explicit.
  vendor_name_rule: Vendor product names appear only in source/provenance/compatibility references.
  file_format_rule: Compatibility targets remain explicit import/export fixtures; Studio does not invent a replacement interchange format for
    parity scope.
taxonomy_enums:
  source_family:
  - studio
  - adobe
  - affinity
  - figma
  relation_class:
  - shared_studio_primitive
  - adobe_source_row
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  - uniqueness_not_proven
  evidence_basis:
  - explicit_row_provenance
  - explicit_parity_matrix
  - manual_surface_grouping
  - exact_normalized_name_match
  - domain_ledger_statement
  - inferred_semantic_overlap
  - not_proven
row_coverage:
  photoshop:
    source_row_file: 39-photoshop-source-distilled-feature-rows.md
    source_domain_file: 34-photoshop-source-distilled-domain-ledger.md
    feature_row_count: 441
    domain_count: 15
    source_app_counts:
      Photoshop: 441
  indesign:
    source_row_file: 40-indesign-source-distilled-feature-rows.md
    source_domain_file: 35-indesign-source-distilled-domain-ledger.md
    feature_row_count: 542
    domain_count: 10
    source_app_counts:
      InDesign: 542
  illustrator:
    source_row_file: 41-illustrator-source-distilled-feature-rows.md
    source_domain_file: 36-illustrator-source-distilled-domain-ledger.md
    feature_row_count: 515
    domain_count: 11
    source_app_counts:
      Illustrator desktop: 515
  affinity:
    source_row_file: 42-affinity-source-distilled-feature-rows.md
    source_domain_file: 37-affinity-source-distilled-domain-ledger.md
    feature_row_count: 1032
    domain_count: 10
    source_app_counts:
      Affinity Photo 2 desktop: 368
      Affinity Designer 2 desktop: 324
      Affinity Publisher 2 desktop: 340
  figma:
    source_row_file: 43-figma-source-distilled-feature-rows.md
    source_domain_file: 38-figma-source-distilled-domain-ledger.md
    feature_row_count: 200
    domain_count: 10
    source_app_counts:
      Figma Developer Platform: 3
      Figma Design: 183
      Figma Make: 2
      FigJam: 4
      Figma Motion: 1
      Figma Slides: 1
      Figma Sites: 1
      Figma Buzz: 1
      Build with Figma: 1
      Figma AI: 1
      Figma Draw: 1
      Figma Community: 1
total_feature_row_count: 2730
affinity_exact_name_overlap_count: 6
```

</topic>

<topic id="primitive-overlap-matrix" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Generated primitive-domain overlap matrix across source app families.">

### [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE.matrix] Primitive Overlap Matrix

```yaml
primitive_overlap_matrix:
- studio_primitive_id: studio.primitive.ai.v1
  primitive_domain: ai
  overlap_class: adobe_figma_shared_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  - figma
  row_counts:
    photoshop: 18
    indesign: 5
    illustrator: 93
    affinity: 0
    figma: 3
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.brand-assets.v1
  primitive_domain: brand_assets
  overlap_class: figma_source_only_current_rows
  source_apps_present:
  - figma
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 0
    affinity: 0
    figma: 1
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.collaboration.v1
  primitive_domain: collaboration
  overlap_class: adobe_source_only_current_rows
  source_apps_present:
  - illustrator
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 2
    affinity: 0
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.color.v1
  primitive_domain: color
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  - affinity
  row_counts:
    photoshop: 22
    indesign: 58
    illustrator: 65
    affinity: 19
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.design-systems.v1
  primitive_domain: design_systems
  overlap_class: figma_source_only_current_rows
  source_apps_present:
  - figma
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 0
    affinity: 0
    figma: 165
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.dev-mode.v1
  primitive_domain: dev_mode
  overlap_class: figma_source_only_current_rows
  source_apps_present:
  - figma
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 0
    affinity: 0
    figma: 6
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.export.v1
  primitive_domain: export
  overlap_class: adobe_source_only_current_rows
  source_apps_present:
  - photoshop
  - indesign
  row_counts:
    photoshop: 61
    indesign: 78
    illustrator: 0
    affinity: 0
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.file-io.v1
  primitive_domain: file_io
  overlap_class: adobe_figma_shared_via_studio_primitive
  source_apps_present:
  - illustrator
  - figma
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 46
    affinity: 0
    figma: 22
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.interactive.v1
  primitive_domain: interactive
  overlap_class: adobe_source_only_current_rows
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  row_counts:
    photoshop: 69
    indesign: 147
    illustrator: 1
    affinity: 0
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.layer.v1
  primitive_domain: layer
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - affinity
  row_counts:
    photoshop: 85
    indesign: 16
    illustrator: 0
    affinity: 112
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.motion.v1
  primitive_domain: motion
  overlap_class: figma_source_only_current_rows
  source_apps_present:
  - figma
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 0
    affinity: 0
    figma: 1
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.page-layout.v1
  primitive_domain: page_layout
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  - affinity
  row_counts:
    photoshop: 14
    indesign: 68
    illustrator: 28
    affinity: 549
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.raster.v1
  primitive_domain: raster
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - affinity
  row_counts:
    photoshop: 76
    indesign: 12
    illustrator: 0
    affinity: 87
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.raw.v1
  primitive_domain: raw
  overlap_class: affinity_unique_or_non_adobe_shared_candidate
  source_apps_present:
  - affinity
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 0
    affinity: 24
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.selection.v1
  primitive_domain: selection
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  - affinity
  row_counts:
    photoshop: 49
    indesign: 3
    illustrator: 19
    affinity: 41
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.style-system.v1
  primitive_domain: style_system
  overlap_class: adobe_source_only_current_rows
  source_apps_present:
  - illustrator
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 17
    affinity: 0
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.tables.v1
  primitive_domain: tables
  overlap_class: adobe_source_only_current_rows
  source_apps_present:
  - indesign
  row_counts:
    photoshop: 0
    indesign: 10
    illustrator: 0
    affinity: 0
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.typography.v1
  primitive_domain: typography
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  - affinity
  row_counts:
    photoshop: 24
    indesign: 117
    illustrator: 57
    affinity: 110
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.vector.v1
  primitive_domain: vector
  overlap_class: affinity_shared_with_adobe_via_studio_primitive
  source_apps_present:
  - photoshop
  - indesign
  - illustrator
  - affinity
  - figma
  row_counts:
    photoshop: 23
    indesign: 28
    illustrator: 177
    affinity: 90
    figma: 1
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.whiteboard.v1
  primitive_domain: whiteboard
  overlap_class: figma_source_only_current_rows
  source_apps_present:
  - figma
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 0
    affinity: 0
    figma: 1
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
- studio_primitive_id: studio.primitive.workspace.v1
  primitive_domain: workspace
  overlap_class: adobe_source_only_current_rows
  source_apps_present:
  - illustrator
  row_counts:
    photoshop: 0
    indesign: 0
    illustrator: 10
    affinity: 0
    figma: 0
  implementation_rule: implement_once_in_studio_primitive_with_source_specific_behavior_variants
affinity_dedupe_domains:
- affinity_domain_id: aff.domain.personas_and_unified_workspaces
  name: Personas, unified workspace, StudioLink, panels, and shared app shell
  dedupe_status: affinity_distinct_workflow_candidate
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - workspace
  - raster
  - vector
  - page_layout
  - asset_pipeline
  manual_topic_candidate: studio.manual.workspace.persona-style-modules
- affinity_domain_id: aff.domain.photo_imaging
  name: Photo raster editing, raw development, selections, masks, adjustments, live filters, and retouch
  dedupe_status: shared_studio_primitive_with_affinity_variant
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - raster
  - camera_raw
  - selection
  - mask
  - color
  - layer
  - brush_engine
  manual_topic_candidate: studio.manual.photo.affinity-class-imaging
- affinity_domain_id: aff.domain.vector_design
  name: Designer vector tools, shapes, curves, pixel persona, constraints, symbols, and export
  dedupe_status: affinity_distinct_workflow_candidate
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - vector
  - geometry
  - boolean_ops
  - brush_engine
  - design_systems
  - export
  manual_topic_candidate: studio.manual.vector.affinity-class-design
- affinity_domain_id: aff.domain.publishing_layout
  name: Publisher pages, spreads, masters, frames, preflight, package, and PDF
  dedupe_status: shared_studio_primitive_with_affinity_variant
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - page_layout
  - master_pages
  - typography
  - tables
  - prepress
  - export
  manual_topic_candidate: studio.manual.layout.affinity-class-publishing
- affinity_domain_id: aff.domain.typography_and_text
  name: Typography, text frames, styles, glyphs, OpenType, text flow, and tables
  dedupe_status: shared_studio_primitive_with_affinity_variant
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - typography
  - text_engine
  - layout
  - style_system
  - tables
  manual_topic_candidate: studio.manual.typography.affinity-class-text
- affinity_domain_id: aff.domain.color_prepress_and_design_aids
  name: Color, swatches, gradients, effects, grids, snapping, resources, and prepress
  dedupe_status: shared_studio_primitive_with_affinity_variant
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - color
  - prepress
  - style_system
  - asset_pipeline
  - layout
  manual_topic_candidate: studio.manual.color.affinity-class-design-aids
- affinity_domain_id: aff.domain.tools_by_app
  name: Tool inventories by app family
  dedupe_status: affinity_unique_candidate_needs_source_page_confirmation
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - workspace
  - raster
  - vector
  - typography
  - page_layout
  - export
  manual_topic_candidate: studio.manual.tools.affinity-source-inventory
- affinity_domain_id: aff.domain.studio_panels
  name: Studio panels, inspectors, history, assets, resources, and diagnostics
  dedupe_status: shared_studio_primitive_with_affinity_variant
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - workspace
  - diagnostics
  - asset_pipeline
  - style_system
  - prepress
  manual_topic_candidate: studio.manual.panels.affinity-class-inspectors
- affinity_domain_id: aff.domain.commands_and_workflow_surfaces
  name: Commands, personas, macros, batch, export, resource management, and recovery
  dedupe_status: affinity_distinct_workflow_candidate
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - automation
  - command_contracts
  - batch
  - export
  - versioning
  manual_topic_candidate: studio.manual.automation.affinity-class-workflows
- affinity_domain_id: aff.domain.compatibility_and_formats
  name: Native documents, PSD/PDF/SVG/EPS/AI-compatible import, raster formats, and export
  dedupe_status: shared_studio_primitive_with_affinity_variant
  source_behavior_preservation_rule: retain Affinity source row and exact behavior notes; do not collapse it into an Adobe source label
  studio_primitive_domains:
  - file_io
  - export
  - pdf
  - svg
  - raster
  - vector
  - page_layout
  manual_topic_candidate: studio.manual.file-compatibility.affinity-class
affinity_exact_name_overlap_records:
- normalized_feature_name: about layers
  relation_class: affinity_exact_name_overlap_with_adobe
  equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_row_refs:
  - osd.affinity.affinity-photo.desktop.leaf.layers-aboutlayers.v1
  - osd.affinity.affinity-designer.desktop.leaf.layers-aboutlayers.v1
  - osd.affinity.affinity-publisher.desktop.leaf.layers-aboutlayers.v1
  adobe_row_refs:
  - osd.indesign.indesign.leaf.create-and-organize-pages.manage-layers.about-layers.v1
- normalized_feature_name: accessible pdfs
  relation_class: affinity_exact_name_overlap_with_adobe
  equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_row_refs:
  - osd.affinity.affinity-publisher.desktop.leaf.publishing-accessiblepdfs.v1
  adobe_row_refs:
  - osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.accessible-pdfs.v1
- normalized_feature_name: create new documents
  relation_class: affinity_exact_name_overlap_with_adobe
  equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_row_refs:
  - osd.affinity.affinity-photo.desktop.leaf.getstarted-newdocument.v1
  - osd.affinity.affinity-designer.desktop.leaf.getstarted-newdocument.v1
  - osd.affinity.affinity-publisher.desktop.leaf.getstarted-newdocument.v1
  adobe_row_refs:
  - osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.create-new-documents.v1
- normalized_feature_name: keyboard shortcuts
  relation_class: affinity_exact_name_overlap_with_adobe
  equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_row_refs:
  - osd.affinity.affinity-photo.desktop.leaf.workspace-customizingshortcuts.v1
  - osd.affinity.affinity-photo.desktop.leaf.workspace-shortcuts.v1
  - osd.affinity.affinity-designer.desktop.leaf.workspace-customizingshortcuts.v1
  - osd.affinity.affinity-designer.desktop.leaf.workspace-shortcuts.v1
  - osd.affinity.affinity-publisher.desktop.leaf.workspace-customizingshortcuts.v1
  - osd.affinity.affinity-publisher.desktop.leaf.workspace-shortcuts.v1
  adobe_row_refs:
  - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
- normalized_feature_name: open documents
  relation_class: affinity_exact_name_overlap_with_adobe
  equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_row_refs:
  - osd.affinity.affinity-publisher.desktop.leaf.getstarted-opendocument.v1
  adobe_row_refs:
  - osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.open-indesign-documents.v1
- normalized_feature_name: supported file formats
  relation_class: affinity_exact_name_overlap_with_adobe
  equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_row_refs:
  - osd.affinity.affinity-photo.desktop.leaf.appendix-fileformat.v1
  - osd.affinity.affinity-publisher.desktop.leaf.appendix-fileformat.v1
  adobe_row_refs:
  - osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html.v1
  - osd.illustrator.illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html.v1
affinity_relation_overlay:
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.addons-aboutaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About add-ons
  normalized_feature_name: about add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.addons-exportingaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Exporting add-ons
  normalized_feature_name: exporting add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.addons-importingaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Importing add-ons
  normalized_feature_name: importing add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.addons-linkingcontent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Linking custom content across apps
  normalized_feature_name: linking custom content across apps
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.adjustments-adjustment-applying.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Applying adjustments
  normalized_feature_name: applying adjustments
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.adjustments-clradjustments.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color adjustments
  normalized_feature_name: color adjustments
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.adjustments-export-3dlut.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Exporting custom adjustments as 3D LUTs
  normalized_feature_name: exporting custom adjustments as 3d luts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.adjustments-otheradjustments.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Other adjustments
  normalized_feature_name: other adjustments
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.adjustments-tonaladjustments.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tonal adjustments
  normalized_feature_name: tonal adjustments
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.appendix-contacting-us.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Contacting us
  normalized_feature_name: contacting us
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.appendix-copyrights.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Copyrights
  normalized_feature_name: copyrights
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.appendix-fileformat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Supported file formats
  normalized_feature_name: supported file formats
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html.v1
    - osd.illustrator.illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.appendix-glossary.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Glossary
  normalized_feature_name: glossary
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.assets-assets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using assets
  normalized_feature_name: using assets
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.astrophotography-astro-about.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About astrophotography stacking
  normalized_feature_name: about astrophotography stacking
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.astrophotography-astro-creating.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Creating an astrophotography stack
  normalized_feature_name: creating an astrophotography stack
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.astrophotography-astro-narrowband.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Compositing narrowband images
  normalized_feature_name: compositing narrowband images
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.astrophotography-astro-panelfiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Files panel
  normalized_feature_name: files panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.astrophotography-astro-panelrawoptions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: RAW Options panel
  normalized_feature_name: raw options panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.astrophotography-astro-panelstackingoptions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Stacking Options panel
  normalized_feature_name: stacking options panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.channels-channelsselectingediting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Selecting and editing channels
  normalized_feature_name: selecting and editing channels
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.channels-maskingchannels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Masking from channels
  normalized_feature_name: masking from channels
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.channels-sparechannels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Spare channels
  normalized_feature_name: spare channels
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.channels-usingchannels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using channels
  normalized_feature_name: using channels
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-aboutclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About color
  normalized_feature_name: about color
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-aboutclrspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color spaces
  normalized_feature_name: color spaces
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-clrchords.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color chords
  normalized_feature_name: color chords
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-clrmatting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Matting
  normalized_feature_name: matting
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-clrmodels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color models
  normalized_feature_name: color models
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-clrprofiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color management
  normalized_feature_name: color management
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-globalclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Global colors
  normalized_feature_name: global colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-gradienteditor.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Gradient and bitmap fills
  normalized_feature_name: gradient and bitmap fills
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-overprint.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Overprinting
  normalized_feature_name: overprinting
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-samplingclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Sampling (or picking) colors
  normalized_feature_name: sampling or picking colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-selectingclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Selecting colors
  normalized_feature_name: selecting colors
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.clr-spotclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Spot colors
  normalized_feature_name: spot colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-about-geometricshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About geometric shapes
  normalized_feature_name: about geometric shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-about-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About lines and shapes
  normalized_feature_name: about lines and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-arrowheads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Arrowheads
  normalized_feature_name: arrowheads
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-converttocurves.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Converting to curves
  normalized_feature_name: converting to curves
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-dot-dash-lines.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Dot/dash line styles
  normalized_feature_name: dot dash line styles
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-draw-geometricshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Draw and edit shapes
  normalized_feature_name: draw and edit shapes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-draw-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Draw lines and shapes
  normalized_feature_name: draw lines and shapes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-draw-qrcodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Draw QR codes
  normalized_feature_name: draw qr codes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-edit-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Edit vector lines and shapes
  normalized_feature_name: edit vector lines and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-join.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Joining shapes
  normalized_feature_name: joining shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-objectgrids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Quick Grids
  normalized_feature_name: quick grids
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-pressure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pressure sensitivity
  normalized_feature_name: pressure sensitivity
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-select-align-nodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Selecting and aligning nodes
  normalized_feature_name: selecting and aligning nodes
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Styles
  normalized_feature_name: styles
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.curvesshapes-transform-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Transforming curves and shapes
  normalized_feature_name: transforming curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-constructionsnapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Construction snapping for curves
  normalized_feature_name: construction snapping for curves
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-curvesnapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Curve snapping
  normalized_feature_name: curve snapping
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-dynamicguides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Dynamic guides
  normalized_feature_name: dynamic guides
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-grids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Grids
  normalized_feature_name: grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-grids-axonometric.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Isometric and axonometric grids
  normalized_feature_name: isometric and axonometric grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Ruler and column guides
  normalized_feature_name: ruler and column guides
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-margins.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Margins
  normalized_feature_name: margins
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-measuring.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Measuring
  normalized_feature_name: measuring
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-objectdefaults.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Vector content defaults
  normalized_feature_name: vector content defaults
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-pixelalign.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Force Pixel Alignment
  normalized_feature_name: force pixel alignment
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-rotatecanvas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rotate canvas
  normalized_feature_name: rotate canvas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-rulers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rulers
  normalized_feature_name: rulers
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-snapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Snapping
  normalized_feature_name: snapping
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-snapshot.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using snapshots
  normalized_feature_name: using snapshots
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-undo.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using undo, redo and history
  normalized_feature_name: using undo redo and history
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-usinghistogram.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using a histogram
  normalized_feature_name: using a histogram
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.designaids-usingvectorscope.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using Vectorscope
  normalized_feature_name: using vectorscope
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportoptionspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Export Options panel
  normalized_feature_name: export options panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportpersona.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Exporting using Export Persona
  normalized_feature_name: exporting using export persona
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  - export persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportpersona-layerspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layers panel
  normalized_feature_name: layers panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-exportsettings.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Export Settings
  normalized_feature_name: export settings
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.exportpersona-slicespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Slices panel
  normalized_feature_name: slices panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-applephotosextensions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Integrating Affinity Photo into Apple Photos
  normalized_feature_name: integrating affinity photo into apple photos
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-benchmark.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Benchmark
  normalized_feature_name: benchmark
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-hardwareacceleration.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Hardware acceleration
  normalized_feature_name: hardware acceleration
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-machinelearning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Affinity and Machine Learning (ML)
  normalized_feature_name: affinity and machine learning ml
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-pentablet.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using pen tablets
  normalized_feature_name: using pen tablets
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-regex.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using regular expressions in Affinity
  normalized_feature_name: using regular expressions in affinity
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-sidecar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using Sidecar
  normalized_feature_name: using sidecar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-surfacedial.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using Surface Dial
  normalized_feature_name: using surface dial
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-surfacepen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using Surface Pen
  normalized_feature_name: using surface pen
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-trackpads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using a trackpad
  normalized_feature_name: using a trackpad
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.extras-windowsphotosextensions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Integrating Affinity Photo into Windows Photos
  normalized_feature_name: integrating affinity photo into windows photos
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-blurfilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Blur filters
  normalized_feature_name: blur filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-clrfilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color filters
  normalized_feature_name: color filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-distortionfilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Distortion filters
  normalized_feature_name: distortion filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-edgedetectionfilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Edge detection filters
  normalized_feature_name: edge detection filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-filter-haze.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Haze Removal
  normalized_feature_name: haze removal
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-filter-shadows-highlights.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Shadows / Highlights
  normalized_feature_name: shadows highlights
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-filters-applying.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Applying filters
  normalized_feature_name: applying filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-imageblending.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Apply Image
  normalized_feature_name: apply image
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-lighting-effects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Lighting
  normalized_feature_name: lighting
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-noisefilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Noise filters
  normalized_feature_name: noise filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-plugins.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using plugins
  normalized_feature_name: using plugins
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.filters-sharpenfilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Sharpen filters
  normalized_feature_name: sharpen filters
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.focusmerging-focusmerge-sourcecloning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Focus merge source cloning
  normalized_feature_name: focus merge source cloning
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.focusmerging-focusmerging.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Focus merging images
  normalized_feature_name: focus merging images
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-close.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Close
  normalized_feature_name: close
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-documentunits.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Document units
  normalized_feature_name: document units
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-editinotheraffinityapps.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Editing files in other Affinity apps
  normalized_feature_name: editing files in other affinity apps
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-newdocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Create new documents
  normalized_feature_name: create new documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.create-new-documents.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-newfromclipboard.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: New from clipboard
  normalized_feature_name: new from clipboard
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-opendocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Open documents and images
  normalized_feature_name: open documents and images
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-openraw.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Opening a raw image
  normalized_feature_name: opening a raw image
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pan/Scroll the document view
  normalized_feature_name: pan scroll the document view
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-save.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Save
  normalized_feature_name: save
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-scanningimages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Scanning images
  normalized_feature_name: scanning images
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-templates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Document templates
  normalized_feature_name: document templates
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-view.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Viewing
  normalized_feature_name: viewing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.getstarted-zoom.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Zooming
  normalized_feature_name: zooming
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.hdr-hdr-editing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: 32-bit HDR editing
  normalized_feature_name: 32 bit hdr editing
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.hdr-hdr-merging.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Merging to 32-bit HDR
  normalized_feature_name: merging to 32 bit hdr
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.hdr-hdr-tonemapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tone mapping HDR images
  normalized_feature_name: tone mapping hdr images
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - tone mapping
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.hdr-ocio.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using OpenColorIO
  normalized_feature_name: using opencolorio
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.hdr-openexr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: 32-bit OpenEXR support
  normalized_feature_name: 32 bit openexr support
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.introduction-about-personas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About Personas
  normalized_feature_name: about personas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.introduction-about-photo.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: What is Affinity Photo?
  normalized_feature_name: what is affinity photo
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.introduction-keyfeatures.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: New features in V2.6
  normalized_feature_name: new features in v2 6
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-create-layerfx.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using layer effects
  normalized_feature_name: using layer effects
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-3d.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: 3D Effect
  normalized_feature_name: 3d effect
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-bevelemboss.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Bevel/Emboss
  normalized_feature_name: bevel emboss
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-clroverlay.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color Overlay
  normalized_feature_name: color overlay
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-gaussianblur.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Gaussian Blur
  normalized_feature_name: gaussian blur
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-gradientoverlay.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Gradient Overlay
  normalized_feature_name: gradient overlay
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-innerglow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Inner Glow
  normalized_feature_name: inner glow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-innershadow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Inner Shadow
  normalized_feature_name: inner shadow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-outerglow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Outer Glow
  normalized_feature_name: outer glow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-outershadow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Outer Shadow
  normalized_feature_name: outer shadow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layerfx-layerfx-outline.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Outline
  normalized_feature_name: outline
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-align.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Aligning
  normalized_feature_name: aligning
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-clipping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer clipping
  normalized_feature_name: layer clipping
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-copypasteoptions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Copying and pasting content
  normalized_feature_name: copying and pasting content
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-distribute.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Spacing
  normalized_feature_name: spacing
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-duplicate.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Duplicating
  normalized_feature_name: duplicating
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-fade.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Fade
  normalized_feature_name: fade
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-finding.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Finding
  normalized_feature_name: finding
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-flipping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Flipping
  normalized_feature_name: flipping
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-group.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Grouping
  normalized_feature_name: grouping
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-isolating.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Isolating
  normalized_feature_name: isolating
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-layerstates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer states
  normalized_feature_name: layer states
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-linking.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Linking
  normalized_feature_name: linking
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-locking.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Locking
  normalized_feature_name: locking
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-merge.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Merging and flattening
  normalized_feature_name: merging and flattening
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-order.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Ordering
  normalized_feature_name: ordering
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-rasterizing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rasterizing
  normalized_feature_name: rasterizing
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-rotateshear.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rotating and shearing
  normalized_feature_name: rotating and shearing
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-select.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Selecting
  normalized_feature_name: selecting
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-taglayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tagging layers
  normalized_feature_name: tagging layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-target.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Targeting
  normalized_feature_name: targeting
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layeroperations-view.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Viewing
  normalized_feature_name: viewing
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-aboutlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: About layers
  normalized_feature_name: about layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.manage-layers.about-layers.v1
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-adjustmentlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Adjustment layers
  normalized_feature_name: adjustment layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-compoundmasks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Compound layer masks
  normalized_feature_name: compound layer masks
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-createlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Creating layers
  normalized_feature_name: creating layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layerblendmodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer blending
  normalized_feature_name: layer blending
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layerblendranges.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer blend ranges
  normalized_feature_name: layer blend ranges
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layerdropzones.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer drop zones
  normalized_feature_name: layer drop zones
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layerfill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Fill layers
  normalized_feature_name: fill layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layerimage.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Image layers
  normalized_feature_name: image layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layermasks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer masks
  normalized_feature_name: layer masks
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layeropacity.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layer opacity
  normalized_feature_name: layer opacity
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-layerpattern.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pattern layers
  normalized_feature_name: pattern layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.layers-livefilters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using Live Filters
  normalized_feature_name: using live filters
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - live filter
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liquifypersona-liquify.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Warping using Liquify Persona
  normalized_feature_name: warping using liquify persona
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liquifypersona-liquify-masking.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Masking in Liquify Persona
  normalized_feature_name: masking in liquify persona
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liquifypersona-liquify-panelbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Brush panel
  normalized_feature_name: brush panel
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liquifypersona-liquify-panelmask.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Mask panel
  normalized_feature_name: mask panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liquifypersona-liquify-panelmesh.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Mesh panel
  normalized_feature_name: mesh panel
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.livemasks-livelayermasks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Live layer masks
  normalized_feature_name: live layer masks
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liveprojection-equirectangular.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Equirectangular projection
  normalized_feature_name: equirectangular projection
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.liveprojection-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Perspective projection
  normalized_feature_name: perspective projection
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.macros-batch-batchjobs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Batch jobs
  normalized_feature_name: batch jobs
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.macros-batch-macros.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Macros
  normalized_feature_name: macros
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.media-embeddingvslinking.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Embedding vs linking
  normalized_feature_name: embedding vs linking
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.media-linkedservices.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Linked Services
  normalized_feature_name: linked services
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.media-placeimages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Placing content
  normalized_feature_name: placing content
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.media-resourcemanager.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Resource Manager
  normalized_feature_name: resource manager
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.media-stockphotos.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using stock photos
  normalized_feature_name: using stock photos
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-erasing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Erasing
  normalized_feature_name: erasing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-painting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Painting brush strokes
  normalized_feature_name: painting brush strokes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-paintmixing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Mixing paint colors
  normalized_feature_name: mixing paint colors
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-pixel-custombrushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Creating custom brushes
  normalized_feature_name: creating custom brushes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-pixel-modify.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Modifying brushes
  normalized_feature_name: modifying brushes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-pixelbrushing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pixel-aligned painting
  normalized_feature_name: pixel aligned painting
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-replaceclrs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Replacing colors by brush
  normalized_feature_name: replacing colors by brush
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.painting-symmetrybrushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Symmetry and Mirror
  normalized_feature_name: symmetry and mirror
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-32bitpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: 32-bit Preview panel
  normalized_feature_name: 32 bit preview panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-adjustmentspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Adjustment panel
  normalized_feature_name: adjustment panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-assetspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Assets panel
  normalized_feature_name: assets panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-batchpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Batch panel
  normalized_feature_name: batch panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-brushespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Brushes panel
  normalized_feature_name: brushes panel
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-channelspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Channels panel
  normalized_feature_name: channels panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-characterpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Character panel
  normalized_feature_name: character panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-clrpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color panel
  normalized_feature_name: color panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-glyphpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Glyph Browser panel
  normalized_feature_name: glyph browser panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-histogrampanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Histogram panel
  normalized_feature_name: histogram panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-historypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: History panel
  normalized_feature_name: history panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-infopanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Info panel
  normalized_feature_name: info panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-layerfxpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Quick FX panel
  normalized_feature_name: quick fx panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-layerspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Layers panel
  normalized_feature_name: layers panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-librarypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Library panel
  normalized_feature_name: library panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-linkspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Links panel
  normalized_feature_name: links panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-macropanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Macro panel
  normalized_feature_name: macro panel
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-metadatapanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Metadata panel
  normalized_feature_name: metadata panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-navigatorpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Navigator panel
  normalized_feature_name: navigator panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-paragraphpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Paragraph panel
  normalized_feature_name: paragraph panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-scopepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Scope panel
  normalized_feature_name: scope panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-snapshotspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Snapshots panel
  normalized_feature_name: snapshots panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-sourcespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Sources panel
  normalized_feature_name: sources panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-statespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: States panel
  normalized_feature_name: states panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-stockpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Stock panel
  normalized_feature_name: stock panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-stylespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Styles panel
  normalized_feature_name: styles panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-swatchespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Swatches panel
  normalized_feature_name: swatches panel
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-textstylespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Text Styles panel
  normalized_feature_name: text styles panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-toolspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tools panel
  normalized_feature_name: tools panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-transformpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Transform panel
  normalized_feature_name: transform panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panels-typographypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Typography panel
  normalized_feature_name: typography panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panorama-panorama-editing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Editing Panoramas
  normalized_feature_name: editing panoramas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.panorama-panorama-stitching.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Stitching Panoramas
  normalized_feature_name: stitching panoramas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Developing raw images
  normalized_feature_name: developing raw images
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-panelbasic.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Basic panel
  normalized_feature_name: basic panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-paneldetails.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Details panel
  normalized_feature_name: details panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-panelfocus.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Focus panel
  normalized_feature_name: focus panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-panellens.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Lens panel
  normalized_feature_name: lens panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-panellocation.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Location panel
  normalized_feature_name: location panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-paneloverlays.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Overlays panel
  normalized_feature_name: overlays panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-panelsnapshots.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Snapshots panel
  normalized_feature_name: snapshots panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-raw-paneltones.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tones panel
  normalized_feature_name: tones panel
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.raw-usingoverlays.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using overlays
  normalized_feature_name: using overlays
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouch.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Retouching
  normalized_feature_name: retouching
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouch-frequencyseparation.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Frequency Separation
  normalized_feature_name: frequency separation
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouching-blemishes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Removing blemishes
  normalized_feature_name: removing blemishes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouching-cloninghealing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Cloning and healing
  normalized_feature_name: cloning and healing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouching-inpainting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Inpainting
  normalized_feature_name: inpainting
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouching-patching.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Patching
  normalized_feature_name: patching
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.retouching-retouching-redeye.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Removing Red Eye
  normalized_feature_name: removing red eye
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-editselectionaslayer.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Edit selection as layer using Quick Mask
  normalized_feature_name: edit selection as layer using quick mask
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-saveloadselections.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Saving and loading selections
  normalized_feature_name: saving and loading selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-brush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: By painting
  normalized_feature_name: by painting
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-create.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Overview
  normalized_feature_name: overview
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-flood.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: By flooding
  normalized_feature_name: by flooding
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-freehand.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: By drawing
  normalized_feature_name: by drawing
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-fromchannels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: From channels
  normalized_feature_name: from channels
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-fromlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: By layer content/luminosity
  normalized_feature_name: by layer content luminosity
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-fromshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: From shapes
  normalized_feature_name: from shapes
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-marquee.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using a marquee
  normalized_feature_name: using a marquee
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-modify.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Modifying pixel selections
  normalized_feature_name: modifying pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-outline.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Creating outline selections
  normalized_feature_name: creating outline selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-range.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: By range
  normalized_feature_name: by range
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-refine.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Refining pixel selection edges
  normalized_feature_name: refining pixel selection edges
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-sampled.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: From a sampled color
  normalized_feature_name: from a sampled color
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-selectsubject.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Select Subject (ML)
  normalized_feature_name: select subject ml
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.selections-selections-transform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Moving and transforming pixel selections
  normalized_feature_name: moving and transforming pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sharing-export.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Export
  normalized_feature_name: export
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sharing-print.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Print
  normalized_feature_name: print
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sharing-share.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Share
  normalized_feature_name: share
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-canvasrotateflip.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rotate/flip the canvas
  normalized_feature_name: rotate flip the canvas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-canvassize.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Changing canvas size
  normalized_feature_name: changing canvas size
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-cropping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Cropping and straightening
  normalized_feature_name: cropping and straightening
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-imagesize.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Changing image size
  normalized_feature_name: changing image size
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-meshwarping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Mesh warping
  normalized_feature_name: mesh warping
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Perspective
  normalized_feature_name: perspective
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-pixelart.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pixel Art resizing
  normalized_feature_name: pixel art resizing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.sizetransform-transform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Transforming
  normalized_feature_name: transforming
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.stacking-stacks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Image stacks
  normalized_feature_name: image stacks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.stacking-stacks-creative.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Creative effects using stacks
  normalized_feature_name: creative effects using stacks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.stacking-stacks-exposuremerge.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Exposure merging using stacks
  normalized_feature_name: exposure merging using stacks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.stacking-stacks-noisereduction.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Noise reduction using stacks
  normalized_feature_name: noise reduction using stacks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.stacking-stacks-objectremoval.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Object removal using stacks
  normalized_feature_name: object removal using stacks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-arttext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Artistic text
  normalized_feature_name: artistic text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-characters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Character formatting
  normalized_feature_name: character formatting
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-frametext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Frame text
  normalized_feature_name: frame text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-importtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Importing text
  normalized_feature_name: importing text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-opentype-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: OpenType font features
  normalized_feature_name: opentype font features
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-paragraphs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Paragraph formatting
  normalized_feature_name: paragraph formatting
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-pathtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Text on a path
  normalized_feature_name: text on a path
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-shapetext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Shape text
  normalized_feature_name: shape text
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-specialcharacters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Special characters and glyphs
  normalized_feature_name: special characters and glyphs
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-spelling.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Spelling
  normalized_feature_name: spelling
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-text-general.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Working with text
  normalized_feature_name: working with text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-textstyles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Using text styles
  normalized_feature_name: using text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-textstyles-create.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Creating and managing text styles
  normalized_feature_name: creating and managing text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-textstyles-remove.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Removing text styles
  normalized_feature_name: removing text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-textstyles-types.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Text style types
  normalized_feature_name: text style types
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.text-variablefonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Variable fonts
  normalized_feature_name: variable fonts
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-arrow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Arrow Tool
  normalized_feature_name: arrow tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-arttext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Artistic Text Tool
  normalized_feature_name: artistic text tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-backgrounderasebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Background Erase Brush Tool
  normalized_feature_name: background erase brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-badpixelmap.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Bad Pixel Map Tool
  normalized_feature_name: bad pixel map tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-blemishremoval.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Blemish Removal Tool
  normalized_feature_name: blemish removal tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-blurbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Blur Brush Tool
  normalized_feature_name: blur brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-burnbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Burn Brush Tool
  normalized_feature_name: burn brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-calloutellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Callout Ellipse Tool
  normalized_feature_name: callout ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-calloutroundedrectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Callout Rounded Rectangle Tool
  normalized_feature_name: callout rounded rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-cat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Cat Tool
  normalized_feature_name: cat tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-clonebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Clone Brush Tool
  normalized_feature_name: clone brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-cloud.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Cloud Tool
  normalized_feature_name: cloud tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-clrpicker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color Picker Tool
  normalized_feature_name: color picker tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-clrreplacementbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Color Replacement Brush Tool
  normalized_feature_name: color replacement brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-cog.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Cog Tool
  normalized_feature_name: cog tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-crescent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Crescent Tool
  normalized_feature_name: crescent tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-crop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Crop Tool
  normalized_feature_name: crop tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-diamond.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Diamond Tool
  normalized_feature_name: diamond tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-dnut.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Donut Tool
  normalized_feature_name: donut tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-dodgebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Dodge Brush Tool
  normalized_feature_name: dodge brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-doublestar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Double Star Tool
  normalized_feature_name: double star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-ellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Ellipse Tool
  normalized_feature_name: ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-erasebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Erase Brush Tool
  normalized_feature_name: erase brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-flooderase.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Flood Erase Tool
  normalized_feature_name: flood erase tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-floodfill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Flood Fill Tool
  normalized_feature_name: flood fill tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-floodselect.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Flood Select Tool
  normalized_feature_name: flood select tool
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-frametext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Frame Text Tool
  normalized_feature_name: frame text tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-gradient.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Gradient Tool
  normalized_feature_name: gradient tool
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-healingbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Healing Brush Tool
  normalized_feature_name: healing brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-heart.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Heart Tool
  normalized_feature_name: heart tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-inpaintingbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Inpainting Brush Tool
  normalized_feature_name: inpainting brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-liquify.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Liquify Tools
  normalized_feature_name: liquify tools
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-marquee.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Marquee Selection Tools
  normalized_feature_name: marquee selection tools
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-measure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Measure Tool
  normalized_feature_name: measure tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-medianbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Median Brush Tool
  normalized_feature_name: median brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-meshwarp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Mesh Warp Tool
  normalized_feature_name: mesh warp tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-move.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Move Tool
  normalized_feature_name: move tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-node.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Node Tool
  normalized_feature_name: node tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-objectselection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Object Selection Tool
  normalized_feature_name: object selection tool
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-paintbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Paint Brush Tool
  normalized_feature_name: paint brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-paintmixerbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Paint Mixer Brush
  normalized_feature_name: paint mixer brush
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: View Tool
  normalized_feature_name: view tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-patch.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Patch Tool
  normalized_feature_name: patch tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-pen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pen Tool
  normalized_feature_name: pen tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Perspective Tool
  normalized_feature_name: perspective tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-pie.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pie Tool
  normalized_feature_name: pie tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-pixel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Pixel Tool
  normalized_feature_name: pixel tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-polygon.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Polygon Tool
  normalized_feature_name: polygon tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-qrcode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: QR Code Tool
  normalized_feature_name: qr code tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-raw.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Raw Tools
  normalized_feature_name: raw tools
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-rectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rectangle Tool
  normalized_feature_name: rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-redeye.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Red Eye Removal Tool
  normalized_feature_name: red eye removal tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-roundedrectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Rounded Rectangle Tool
  normalized_feature_name: rounded rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-segment.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Segment Tool
  normalized_feature_name: segment tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-selectionbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Selection Brush Tool
  normalized_feature_name: selection brush tool
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-sharpenbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Sharpen Brush Tool
  normalized_feature_name: sharpen brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-slice.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Slice Tool
  normalized_feature_name: slice tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-smudgebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Smudge Brush Tool
  normalized_feature_name: smudge brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-spiral.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Spiral Tool
  normalized_feature_name: spiral tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-spongebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Sponge Brush Tool
  normalized_feature_name: sponge brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-squarestar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Square Star Tool
  normalized_feature_name: square star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-star.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Star Tool
  normalized_feature_name: star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-stylepicker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Style Picker Tool
  normalized_feature_name: style picker tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-tear.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tear Tool
  normalized_feature_name: tear tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-trapezoid.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Trapezoid Tool
  normalized_feature_name: trapezoid tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-triangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Triangle Tool
  normalized_feature_name: triangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-undobrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Undo Brush Tool
  normalized_feature_name: undo brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.tools-tools-zoom.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Zoom Tool
  normalized_feature_name: zoom tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-accessibility.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Accessibility
  normalized_feature_name: accessibility
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-contextbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Context toolbar
  normalized_feature_name: context toolbar
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-customizingshortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Keyboard shortcuts
  normalized_feature_name: keyboard shortcuts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-customizingtoolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Toolbar
  normalized_feature_name: toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-customizingtoolspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Tools panel
  normalized_feature_name: tools panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-customizingworkspace.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Workspace
  normalized_feature_name: workspace
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-expressions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Expressions for field input
  normalized_feature_name: expressions for field input
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-interface.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Interface Visual Reference
  normalized_feature_name: interface visual reference
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-personatoolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Persona Toolbar
  normalized_feature_name: persona toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Settings (Preferences)
  normalized_feature_name: settings preferences
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-shortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Keyboard shortcuts
  normalized_feature_name: keyboard shortcuts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-toolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Toolbar
  normalized_feature_name: toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-uiappearance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Changing the UI appearance
  normalized_feature_name: changing the ui appearance
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-photo.desktop.leaf.workspace-workspacemodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Photo 2 desktop
  feature_name: Application and document windows
  normalized_feature_name: application and document windows
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.addons-aboutaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About add-ons
  normalized_feature_name: about add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.addons-exportingaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Exporting add-ons
  normalized_feature_name: exporting add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.addons-importingaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Importing add-ons
  normalized_feature_name: importing add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.addons-linkingcontent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Linking custom content across apps
  normalized_feature_name: linking custom content across apps
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-3dlut.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: LUT adjustment
  normalized_feature_name: lut adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-applying.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Applying adjustments
  normalized_feature_name: applying adjustments
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-blackandwhite.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Black and White adjustment
  normalized_feature_name: black and white adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-brightnesscontrast.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Brightness / Contrast adjustment
  normalized_feature_name: brightness contrast adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-channelmixer.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Channel Mixer adjustment
  normalized_feature_name: channel mixer adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-clrbalance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color Balance adjustment
  normalized_feature_name: color balance adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-curves.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Curves adjustment
  normalized_feature_name: curves adjustment
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-exposure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Exposure adjustment
  normalized_feature_name: exposure adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-gradientmap.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Gradient Map adjustment
  normalized_feature_name: gradient map adjustment
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-hsl.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: HSL adjustment
  normalized_feature_name: hsl adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-invert.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Invert adjustment
  normalized_feature_name: invert adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-lensfilter.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Lens Filter adjustment
  normalized_feature_name: lens filter adjustment
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-levels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Levels adjustment
  normalized_feature_name: levels adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-ocio.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: OpenColorIO adjustment
  normalized_feature_name: opencolorio adjustment
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-posterize.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Posterize adjustment
  normalized_feature_name: posterize adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-reclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Recolor adjustment
  normalized_feature_name: recolor adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-selectiveclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selective Color adjustment
  normalized_feature_name: selective color adjustment
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-shadowshighlights.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Shadows / Highlights adjustment
  normalized_feature_name: shadows highlights adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-softproof.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Soft Proof adjustment
  normalized_feature_name: soft proof adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-splittoning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Split Toning adjustment
  normalized_feature_name: split toning adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-threshold.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Threshold adjustment
  normalized_feature_name: threshold adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-vibrance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Vibrance adjustment
  normalized_feature_name: vibrance adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.adjustments-adjustment-whitebalance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: White Balance adjustment
  normalized_feature_name: white balance adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.appendix-contacting-us.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Contacting us
  normalized_feature_name: contacting us
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.appendix-copyrights.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Copyrights
  normalized_feature_name: copyrights
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.appendix-fileformat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Import and export file formats
  normalized_feature_name: import and export file formats
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.appendix-glossary.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Glossary
  normalized_feature_name: glossary
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-about.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About artboards
  normalized_feature_name: about artboards
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-adddelete.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Adding and removing
  normalized_feature_name: adding and removing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-aligndistribute.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Aligning and distributing
  normalized_feature_name: aligning and distributing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-clr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color and opacity
  normalized_feature_name: color and opacity
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-designaids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Design aids
  normalized_feature_name: design aids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-export.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Exporting
  normalized_feature_name: exporting
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-moveresize.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selecting, moving and resizing
  normalized_feature_name: selecting moving and resizing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-objectcontrol.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Moving artboard content
  normalized_feature_name: moving artboard content
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-print.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Printing
  normalized_feature_name: printing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.artboards-artboards-renameview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Renaming and viewing
  normalized_feature_name: renaming and viewing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-aboutclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About color
  normalized_feature_name: about color
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-aboutclrspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About color spaces
  normalized_feature_name: about color spaces
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-clrchords.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color chords
  normalized_feature_name: color chords
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-clrmodels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color models
  normalized_feature_name: color models
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-clrprofiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color management
  normalized_feature_name: color management
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-globalclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Global colors
  normalized_feature_name: global colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-gradienteditor.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Gradient and bitmap fills
  normalized_feature_name: gradient and bitmap fills
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-ocio.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using OpenColorIO
  normalized_feature_name: using opencolorio
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-overprint.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Overprinting
  normalized_feature_name: overprinting
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-samplingclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Sampling colors
  normalized_feature_name: sampling colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-selectingclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selecting colors
  normalized_feature_name: selecting colors
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-spotclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Spot colors
  normalized_feature_name: spot colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-transparency.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Transparency
  normalized_feature_name: transparency
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.clr-transparencyeditor.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Transparency editing
  normalized_feature_name: transparency editing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-about-geometricshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About geometric shapes
  normalized_feature_name: about geometric shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-about-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About lines, curves and shapes
  normalized_feature_name: about lines curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-arrowheads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Arrowheads
  normalized_feature_name: arrowheads
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-contouringshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Contouring curves and shapes
  normalized_feature_name: contouring curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-corneringshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Cornering shapes
  normalized_feature_name: cornering shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-dot-dash-lines.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Dot/dash line styles
  normalized_feature_name: dot dash line styles
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-draw-geometricshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Draw and edit shapes
  normalized_feature_name: draw and edit shapes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-draw-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Draw curves and shapes
  normalized_feature_name: draw curves and shapes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-draw-pencillines.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Draw pencil lines
  normalized_feature_name: draw pencil lines
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-draw-qrcodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Draw QR codes
  normalized_feature_name: draw qr codes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-edit-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Edit curves and shapes
  normalized_feature_name: edit curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-edit-pressureprofiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Edit on-page pressure profiles
  normalized_feature_name: edit on page pressure profiles
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-expandstroke.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Expand stroke
  normalized_feature_name: expand stroke
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-fillmode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Fill modes
  normalized_feature_name: fill modes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-multistrokesandfills.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using multiple strokes and fills
  normalized_feature_name: using multiple strokes and fills
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-pressure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Pressure sensitivity
  normalized_feature_name: pressure sensitivity
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-select-align-nodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selecting and aligning nodes
  normalized_feature_name: selecting and aligning nodes
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-texture-line-style.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Texture line styles
  normalized_feature_name: texture line styles
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.curvesshapes-transform-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Transforming curves and shapes
  normalized_feature_name: transforming curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-cliptocanvas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Clip to Canvas
  normalized_feature_name: clip to canvas
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-constraints.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Constraints
  normalized_feature_name: constraints
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-constructionsnapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Construction snapping for curves
  normalized_feature_name: construction snapping for curves
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-curvesnapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Curve snapping
  normalized_feature_name: curve snapping
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-dynamicguides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Dynamic guides
  normalized_feature_name: dynamic guides
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-grids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Grids
  normalized_feature_name: grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-grids-axonometric.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Isometric and axonometric grids
  normalized_feature_name: isometric and axonometric grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Ruler and column guides
  normalized_feature_name: ruler and column guides
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-margins.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Margins
  normalized_feature_name: margins
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-measuring.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Measuring
  normalized_feature_name: measuring
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-pixelalign.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Force Pixel Alignment
  normalized_feature_name: force pixel alignment
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-rotatecanvas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Rotate document view
  normalized_feature_name: rotate document view
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-rulers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Rulers
  normalized_feature_name: rulers
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-snapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Snapping
  normalized_feature_name: snapping
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-snapshot.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using snapshots
  normalized_feature_name: using snapshots
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.designaids-undo.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using undo, redo and history
  normalized_feature_name: using undo redo and history
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportoptionspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Export Options panel
  normalized_feature_name: export options panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportpersona.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Exporting using Export Persona
  normalized_feature_name: exporting using export persona
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  - export persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportpersona-layerspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layers panel (Export Persona)
  normalized_feature_name: layers panel export persona
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  - export persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-exportsettings.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Export Settings
  normalized_feature_name: export settings
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.exportpersona-slicespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Slices panel
  normalized_feature_name: slices panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-hardwareacceleration.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Hardware acceleration
  normalized_feature_name: hardware acceleration
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-pentablet.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using pen tablets
  normalized_feature_name: using pen tablets
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-regex.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using regular expressions in Affinity
  normalized_feature_name: using regular expressions in affinity
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-sidecar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using Sidecar
  normalized_feature_name: using sidecar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-surfacedial.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using Surface Dial
  normalized_feature_name: using surface dial
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-surfacepen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using Surface Pen
  normalized_feature_name: using surface pen
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.extras-trackpads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using a trackpad
  normalized_feature_name: using a trackpad
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.filters-filter-meshwarp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Mesh Warp filter
  normalized_feature_name: mesh warp filter
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.filters-filter-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Perspective filter
  normalized_feature_name: perspective filter
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-aboutbitdepth.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About bit depth
  normalized_feature_name: about bit depth
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-close.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Close
  normalized_feature_name: close
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-documentsetup.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Document setup
  normalized_feature_name: document setup
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-documentunits.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Document units
  normalized_feature_name: document units
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-drawingscale.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Drawing scale
  normalized_feature_name: drawing scale
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-editinotheraffinityapps.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Editing files in other Affinity apps
  normalized_feature_name: editing files in other affinity apps
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-importadobe.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Importing Adobe documents
  normalized_feature_name: importing adobe documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-importcad.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Importing CAD documents
  normalized_feature_name: importing cad documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-importpdf.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Importing PDF documents
  normalized_feature_name: importing pdf documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-newdocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Create new documents
  normalized_feature_name: create new documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.create-new-documents.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-newfromclipboard.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: New from clipboard
  normalized_feature_name: new from clipboard
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-opendocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Open documents and images
  normalized_feature_name: open documents and images
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Pan/Scroll the document view
  normalized_feature_name: pan scroll the document view
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-save.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Save
  normalized_feature_name: save
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-templates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Document templates
  normalized_feature_name: document templates
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-view.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Viewing
  normalized_feature_name: viewing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.getstarted-zoom.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Zooming
  normalized_feature_name: zooming
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.introduction-about-designer.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Affinity Designer
  normalized_feature_name: affinity designer
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.introduction-about-personas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Personas
  normalized_feature_name: personas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.introduction-keyfeatures.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: New features in V2.6
  normalized_feature_name: new features in v2 6
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.introduction-switchingpersonas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Switching Personas
  normalized_feature_name: switching personas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-create-layerfx.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using layer effects
  normalized_feature_name: using layer effects
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-3d.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: 3D Effect
  normalized_feature_name: 3d effect
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-bevelemboss.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Bevel / Emboss
  normalized_feature_name: bevel emboss
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-clroverlay.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color Overlay
  normalized_feature_name: color overlay
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-gaussianblur.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Gaussian Blur
  normalized_feature_name: gaussian blur
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-gradientoverlay.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Gradient Overlay
  normalized_feature_name: gradient overlay
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-innerglow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Inner Glow
  normalized_feature_name: inner glow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-innershadow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Inner Shadow
  normalized_feature_name: inner shadow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-outerglow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Outer Glow
  normalized_feature_name: outer glow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-outershadow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Outer Shadow
  normalized_feature_name: outer shadow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layerfx-layerfx-outline.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Outline
  normalized_feature_name: outline
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-aboutlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About layers
  normalized_feature_name: about layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.manage-layers.about-layers.v1
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-adjustmentlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using adjustment layers
  normalized_feature_name: using adjustment layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-createlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Create layers
  normalized_feature_name: create layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layerblendmodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer blending
  normalized_feature_name: layer blending
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layerblendranges.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer blend ranges
  normalized_feature_name: layer blend ranges
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layerclip.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer clipping
  normalized_feature_name: layer clipping
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layercolours.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer colors
  normalized_feature_name: layer colors
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layerdropzones.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer drop zones
  normalized_feature_name: layer drop zones
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layerimage.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Image layers
  normalized_feature_name: image layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layermasks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer masking
  normalized_feature_name: layer masking
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-layeropacity.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer opacity
  normalized_feature_name: layer opacity
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-managelayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Arrange/manage layers
  normalized_feature_name: arrange manage layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-selecteditlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selecting and editing layers
  normalized_feature_name: selecting and editing layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.layers-taglayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Tagging layers
  normalized_feature_name: tagging layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.media-embeddingvslinking.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Embedding vs linking
  normalized_feature_name: embedding vs linking
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.media-linkedservices.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Linked Services
  normalized_feature_name: linked services
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.media-placeimages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Placing content
  normalized_feature_name: placing content
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.media-resourcemanager.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Resource Manager
  normalized_feature_name: resource manager
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.media-stockphotos.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using stock photos
  normalized_feature_name: using stock photos
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-align.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Aligning objects
  normalized_feature_name: aligning objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-compound.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating compounds with Boolean operations
  normalized_feature_name: creating compounds with boolean operations
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-converttocurves.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Convert objects to curves
  normalized_feature_name: convert objects to curves
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-copypasteoptions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Copying and pasting objects
  normalized_feature_name: copying and pasting objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-cutting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Cutting objects
  normalized_feature_name: cutting objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-distribute.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Distributing and spacing objects
  normalized_feature_name: distributing and spacing objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-duplicate.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Duplicating objects
  normalized_feature_name: duplicating objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-finding.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Finding objects
  normalized_feature_name: finding objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-floodingareas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Flooding areas
  normalized_feature_name: flooding areas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-group.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Grouping objects
  normalized_feature_name: grouping objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-isolating.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Isolating objects
  normalized_feature_name: isolating objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-join.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Joining objects with Boolean operations
  normalized_feature_name: joining objects with boolean operations
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-join-shapebuilder.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Adding objects by shape building
  normalized_feature_name: adding objects by shape building
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-layerstates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layer states
  normalized_feature_name: layer states
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-objectdefaults.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Object defaults
  normalized_feature_name: object defaults
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-objectgrids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Quick Grids
  normalized_feature_name: quick grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-order.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Ordering objects
  normalized_feature_name: ordering objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-rasterizing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Rasterizing
  normalized_feature_name: rasterizing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-rotateshear.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Rotating and shearing objects
  normalized_feature_name: rotating and shearing objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-select.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selecting objects
  normalized_feature_name: selecting objects
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-selectbyattribute.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selecting objects by attribute
  normalized_feature_name: selecting objects by attribute
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Styles
  normalized_feature_name: styles
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-target.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Targeting objects
  normalized_feature_name: targeting objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-transform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Transforming objects
  normalized_feature_name: transforming objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.objectcontrol-warp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Warping objects
  normalized_feature_name: warping objects
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-create-custombrushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating custom vector brushes
  normalized_feature_name: creating custom vector brushes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-erasing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Erasing
  normalized_feature_name: erasing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-modifystrokes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Modifying vector brush strokes
  normalized_feature_name: modifying vector brush strokes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-painting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Painting vector brush strokes
  normalized_feature_name: painting vector brush strokes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-pixel-custombrushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating custom pixel brushes
  normalized_feature_name: creating custom pixel brushes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-pixel-modify.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Modifying pixel brushes
  normalized_feature_name: modifying pixel brushes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-pixel-painting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Painting pixel brush strokes
  normalized_feature_name: painting pixel brush strokes
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-retouch.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Retouch
  normalized_feature_name: retouch
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.painting-symmetrybrushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Symmetry and Mirror
  normalized_feature_name: symmetry and mirror
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-32bitpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: 32-bit Preview panel
  normalized_feature_name: 32 bit preview panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-appearancepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Appearance panel
  normalized_feature_name: appearance panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-assetspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Assets panel
  normalized_feature_name: assets panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-brushespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Brushes panel
  normalized_feature_name: brushes panel
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-characterpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Character panel
  normalized_feature_name: character panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-clrpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color panel
  normalized_feature_name: color panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-constraintspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Constraints panel
  normalized_feature_name: constraints panel
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-glyphpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Glyph Browser panel
  normalized_feature_name: glyph browser panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-historypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: History panel
  normalized_feature_name: history panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-isometricpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Isometric panel
  normalized_feature_name: isometric panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-layerfxpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Quick FX panel
  normalized_feature_name: quick fx panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-layerspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Layers panel
  normalized_feature_name: layers panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-navigatorpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Navigator panel
  normalized_feature_name: navigator panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-paragraphpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Paragraph panel
  normalized_feature_name: paragraph panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-statespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: States panel
  normalized_feature_name: states panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-stockpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Stock panel
  normalized_feature_name: stock panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-strokepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Stroke panel
  normalized_feature_name: stroke panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-stylespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Styles panel
  normalized_feature_name: styles panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-swatchespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Swatches panel
  normalized_feature_name: swatches panel
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-symbolspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Symbols panel
  normalized_feature_name: symbols panel
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-textstylespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Text Styles panel
  normalized_feature_name: text styles panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-toolspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Tools panel
  normalized_feature_name: tools panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-transformpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Transform panel
  normalized_feature_name: transform panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.panels-typographypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Typography panel
  normalized_feature_name: typography panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-create.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating pixel selections
  normalized_feature_name: creating pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-flood.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Flooding pixel selections
  normalized_feature_name: flooding pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-modify.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Modifying pixel selections
  normalized_feature_name: modifying pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-outline.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating outline selections
  normalized_feature_name: creating outline selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-range.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Range pixel selections
  normalized_feature_name: range pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-refine.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Refining pixel selection edges
  normalized_feature_name: refining pixel selection edges
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.selections-selections-sampled.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Sampled color pixel selections
  normalized_feature_name: sampled color pixel selections
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-aboutpackaging.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: About packaging
  normalized_feature_name: about packaging
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-bleed.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Setting bleed
  normalized_feature_name: setting bleed
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-createsvg.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Working with SVGs
  normalized_feature_name: working with svgs
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-creatingpackages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating packages
  normalized_feature_name: creating packages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-export.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Export
  normalized_feature_name: export
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-openingpackages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Opening packages
  normalized_feature_name: opening packages
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-print.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Print
  normalized_feature_name: print
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-resavingpackages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Resaving modified packages
  normalized_feature_name: resaving modified packages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.sharesaveprint-share.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Share
  normalized_feature_name: share
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.symbolsassets-assets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using assets
  normalized_feature_name: using assets
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.symbolsassets-symbols.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Symbols
  normalized_feature_name: symbols
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-arttext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Artistic text
  normalized_feature_name: artistic text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-characters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Character formatting
  normalized_feature_name: character formatting
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-frametext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Frame text
  normalized_feature_name: frame text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-importtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Importing text
  normalized_feature_name: importing text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-opentype-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: OpenType font features
  normalized_feature_name: opentype font features
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-paragraphs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Paragraph formatting
  normalized_feature_name: paragraph formatting
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-pathtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Text on a path
  normalized_feature_name: text on a path
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-shapetext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Shape text
  normalized_feature_name: shape text
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-specialcharacters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Special characters and glyphs
  normalized_feature_name: special characters and glyphs
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-spelling.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Spelling
  normalized_feature_name: spelling
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-text-general.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Working with text
  normalized_feature_name: working with text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-textstyles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Using text styles
  normalized_feature_name: using text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-textstyles-create.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Creating and managing text styles
  normalized_feature_name: creating and managing text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-textstyles-remove.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Removing text styles
  normalized_feature_name: removing text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-textstyles-types.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Text style types
  normalized_feature_name: text style types
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.text-variablefonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Variable fonts
  normalized_feature_name: variable fonts
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-area.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Area Tool
  normalized_feature_name: area tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-arrow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Arrow Tool
  normalized_feature_name: arrow tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-artboard.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Artboard Tool
  normalized_feature_name: artboard tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-arttext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Artistic Text Tool
  normalized_feature_name: artistic text tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-blurbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Blur Brush Tool
  normalized_feature_name: blur brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-brush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Vector Brush Tool
  normalized_feature_name: vector brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-burnbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Burn Brush Tool
  normalized_feature_name: burn brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-calloutellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Callout Ellipse Tool
  normalized_feature_name: callout ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-calloutroundedrectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Callout Rounded Rectangle Tool
  normalized_feature_name: callout rounded rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-cat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Cat Tool
  normalized_feature_name: cat tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-cloud.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Cloud Tool
  normalized_feature_name: cloud tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-clrpicker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Color Picker Tool
  normalized_feature_name: color picker tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-cog.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Cog Tool
  normalized_feature_name: cog tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-contour.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Contour Tool
  normalized_feature_name: contour tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-corner.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Corner Tool
  normalized_feature_name: corner tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-crescent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Crescent Tool
  normalized_feature_name: crescent tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-crop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Vector Crop Tool
  normalized_feature_name: vector crop tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-diamond.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Diamond Tool
  normalized_feature_name: diamond tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-dnut.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Donut Tool
  normalized_feature_name: donut tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-dodgebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Dodge Brush Tool
  normalized_feature_name: dodge brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-doublestar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Double Star Tool
  normalized_feature_name: double star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-ellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Ellipse Tool
  normalized_feature_name: ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-erasebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Erase Brush Tool
  normalized_feature_name: erase brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-floodfill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Flood Fill Tool
  normalized_feature_name: flood fill tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-floodselect.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Flood Select Tool
  normalized_feature_name: flood select tool
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-frametext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Frame Text Tool
  normalized_feature_name: frame text tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-gradient.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Gradient Tool
  normalized_feature_name: gradient tool
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-heart.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Heart Tool
  normalized_feature_name: heart tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-knife.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Knife Tool
  normalized_feature_name: knife tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-linewidth.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Stroke Width Tool
  normalized_feature_name: stroke width tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-marquee.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Marquee Selection Tools
  normalized_feature_name: marquee selection tools
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-measure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Measure Tool
  normalized_feature_name: measure tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-move.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Move Tool
  normalized_feature_name: move tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-node.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Node Tool
  normalized_feature_name: node tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-paintbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Paint Brush Tool
  normalized_feature_name: paint brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: View Tool
  normalized_feature_name: view tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-pen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Pen Tool
  normalized_feature_name: pen tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-pencil.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Pencil Tool
  normalized_feature_name: pencil tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-pie.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Pie Tool
  normalized_feature_name: pie tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-pixel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Pixel Tool
  normalized_feature_name: pixel tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-placeimage.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Place Tool
  normalized_feature_name: place tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-pointtransform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Point Transform Tool
  normalized_feature_name: point transform tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-polygon.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Polygon Tool
  normalized_feature_name: polygon tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-qrcode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: QR Code Tool
  normalized_feature_name: qr code tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-rectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Rectangle Tool
  normalized_feature_name: rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-roundedrectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Rounded Rectangle Tool
  normalized_feature_name: rounded rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-segment.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Segment Tool
  normalized_feature_name: segment tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-selectionbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Selection Brush Tool
  normalized_feature_name: selection brush tool
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-shapebuilder.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Shape Builder Tool
  normalized_feature_name: shape builder tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-sharpenbrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Sharpen Brush Tool
  normalized_feature_name: sharpen brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-slice.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Slice Tool
  normalized_feature_name: slice tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-smudgebrush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Smudge Brush Tool
  normalized_feature_name: smudge brush tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-spiral.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Spiral Tool
  normalized_feature_name: spiral tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-squarestar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Square Star Tool
  normalized_feature_name: square star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-star.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Star Tool
  normalized_feature_name: star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-stylepicker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Style Picker Tool
  normalized_feature_name: style picker tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-tear.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Tear Tool
  normalized_feature_name: tear tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-transparency.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Transparency Tool
  normalized_feature_name: transparency tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-trapezoid.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Trapezoid Tool
  normalized_feature_name: trapezoid tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-triangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Triangle Tool
  normalized_feature_name: triangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-vectorfloodfill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Vector Flood Fill Tool
  normalized_feature_name: vector flood fill tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.tools-tools-zoom.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Zoom Tool
  normalized_feature_name: zoom tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-accessibility.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Accessibility
  normalized_feature_name: accessibility
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-contextbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Context toolbar
  normalized_feature_name: context toolbar
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-customizingshortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Keyboard shortcuts
  normalized_feature_name: keyboard shortcuts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-customizingtoolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Toolbar
  normalized_feature_name: toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-customizingtoolspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Tools panel
  normalized_feature_name: tools panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-customizingworkspace.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Workspace
  normalized_feature_name: workspace
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-expressions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Expressions for field input
  normalized_feature_name: expressions for field input
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-fieldinput.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Field input
  normalized_feature_name: field input
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-interface.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Interface Visual Reference
  normalized_feature_name: interface visual reference
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-personatoolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Persona Toolbar
  normalized_feature_name: persona toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Settings (Preferences)
  normalized_feature_name: settings preferences
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-shortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Keyboard shortcuts
  normalized_feature_name: keyboard shortcuts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-toolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Toolbar
  normalized_feature_name: toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-uiappearance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Changing the UI appearance
  normalized_feature_name: changing the ui appearance
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-designer.desktop.leaf.workspace-workspacemodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Designer 2 desktop
  feature_name: Application and document windows
  normalized_feature_name: application and document windows
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.addons-aboutaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About add-ons
  normalized_feature_name: about add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.addons-exportingaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Exporting add-ons
  normalized_feature_name: exporting add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.addons-importingaddons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Importing add-ons
  normalized_feature_name: importing add ons
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.addons-linkingcontent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Linking custom content across apps
  normalized_feature_name: linking custom content across apps
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-3dlut.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: LUT adjustment
  normalized_feature_name: lut adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-applying.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Applying adjustments
  normalized_feature_name: applying adjustments
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-blackandwhite.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Black and White adjustment
  normalized_feature_name: black and white adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-brightnesscontrast.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Brightness and Contrast adjustment
  normalized_feature_name: brightness and contrast adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-channelmixer.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Channel Mixer adjustment
  normalized_feature_name: channel mixer adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-clrbalance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color Balance adjustment
  normalized_feature_name: color balance adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-curves.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Curves adjustment
  normalized_feature_name: curves adjustment
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-exposure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Exposure adjustment
  normalized_feature_name: exposure adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-gradientmap.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Gradient Map adjustment
  normalized_feature_name: gradient map adjustment
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-hsl.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: HSL adjustment
  normalized_feature_name: hsl adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-invert.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Invert adjustment
  normalized_feature_name: invert adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-lensfilter.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Lens Filter adjustment
  normalized_feature_name: lens filter adjustment
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-levels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Levels adjustment
  normalized_feature_name: levels adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-ocio.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: OpenColorIO adjustment
  normalized_feature_name: opencolorio adjustment
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-posterize.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Posterize adjustment
  normalized_feature_name: posterize adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-reclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Recolor adjustment
  normalized_feature_name: recolor adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-selectiveclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Selective Color adjustment
  normalized_feature_name: selective color adjustment
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-shadowshighlights.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Shadows/Highlights adjustment
  normalized_feature_name: shadows highlights adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-softproof.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Soft Proof adjustment
  normalized_feature_name: soft proof adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-splittoning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Split Toning adjustment
  normalized_feature_name: split toning adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-threshold.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Threshold adjustment
  normalized_feature_name: threshold adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-vibrance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Vibrance adjustment
  normalized_feature_name: vibrance adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.adjustments-adjustment-whitebalance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: White Balance adjustment
  normalized_feature_name: white balance adjustment
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-aboutbooks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About books
  normalized_feature_name: about books
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-aboutcrossrefs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About cross-references
  normalized_feature_name: about cross references
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-aboutnotes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About notes
  normalized_feature_name: about notes
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-anchors.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Anchors
  normalized_feature_name: anchors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-creatingbooks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating books
  normalized_feature_name: creating books
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-datamerge.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Data merge
  normalized_feature_name: data merge
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-fields.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Fields
  normalized_feature_name: fields
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-formattingcrossrefs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Formatting cross-references
  normalized_feature_name: formatting cross references
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-hyperlinkingnotes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Hyperlinking notes
  normalized_feature_name: hyperlinking notes
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-hyperlinks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Hyperlinks
  normalized_feature_name: hyperlinks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-index.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Index
  normalized_feature_name: index
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-insertingnotes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Inserting notes
  normalized_feature_name: inserting notes
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-mergedocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Merge documents
  normalized_feature_name: merge documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-outputtingbooks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Outputting books
  normalized_feature_name: outputting books
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-pdfbookmarks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: PDF bookmarks
  normalized_feature_name: pdf bookmarks
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-settingcrossrefstarget.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Setting a cross-reference's target
  normalized_feature_name: setting a cross reference s target
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-settingcrossrefstext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Setting a cross-reference's text
  normalized_feature_name: setting a cross reference s text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-stylingnotes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Styling notes
  normalized_feature_name: styling notes
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-syncingchapters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Synchronizing chapters
  normalized_feature_name: synchronizing chapters
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-toc.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Table of contents
  normalized_feature_name: table of contents
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-updatingcrossrefs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Updating cross-references
  normalized_feature_name: updating cross references
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.advanced-updatingnumbers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Updating page, list and note numbers
  normalized_feature_name: updating page list and note numbers
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.appendix-contacting-us.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Contacting us
  normalized_feature_name: contacting us
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.appendix-copyrights.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Copyrights
  normalized_feature_name: copyrights
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.appendix-fileformat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Supported file formats
  normalized_feature_name: supported file formats
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html.v1
    - osd.illustrator.illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.appendix-glossary.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Glossary
  normalized_feature_name: glossary
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.assets-assets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using assets
  normalized_feature_name: using assets
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-aboutclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About color
  normalized_feature_name: about color
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-aboutclrspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color spaces
  normalized_feature_name: color spaces
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-clrchords.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color chords
  normalized_feature_name: color chords
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-clrmodels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color models
  normalized_feature_name: color models
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-clrprofiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color management
  normalized_feature_name: color management
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-globalclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Global colors
  normalized_feature_name: global colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-gradienteditor.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Gradient and bitmap fills
  normalized_feature_name: gradient and bitmap fills
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-ocio.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using OpenColorIO
  normalized_feature_name: using opencolorio
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-overprint.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Overprinting
  normalized_feature_name: overprinting
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-samplingclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Sampling colors
  normalized_feature_name: sampling colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-selectingclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Selecting colors
  normalized_feature_name: selecting colors
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-spotclr.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Spot colors
  normalized_feature_name: spot colors
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-transparency.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Transparency
  normalized_feature_name: transparency
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.clr-transparencyeditor.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Transparency editing
  normalized_feature_name: transparency editing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-about-geometricshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About geometric shapes
  normalized_feature_name: about geometric shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-about-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About lines, curves and shapes
  normalized_feature_name: about lines curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-arrowheads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Arrowheads
  normalized_feature_name: arrowheads
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-dot-dash-lines.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Dot/dash line styles
  normalized_feature_name: dot dash line styles
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-draw-geometricshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Draw and edit shapes
  normalized_feature_name: draw and edit shapes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-draw-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Draw curves and shapes
  normalized_feature_name: draw curves and shapes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-draw-qrcodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Draw QR codes
  normalized_feature_name: draw qr codes
  studio_surface: StudioRawDevelopRecipe
  primitive_domain: raw
  relation_class:
  - affinity_source_row
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps: []
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-edit-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Edit curves and shapes
  normalized_feature_name: edit curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-pressure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pressure sensitivity
  normalized_feature_name: pressure sensitivity
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-select-align-nodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Selecting and aligning nodes
  normalized_feature_name: selecting and aligning nodes
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.curvesshapes-transform-linesandshapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Transforming curves and shapes
  normalized_feature_name: transforming curves and shapes
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-baselinegrids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Baseline Grids
  normalized_feature_name: baseline grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-cliptocanvas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Clip to Canvas
  normalized_feature_name: clip to canvas
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-constraints.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Constraints
  normalized_feature_name: constraints
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-curvesnapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Curve snapping
  normalized_feature_name: curve snapping
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-dynamicguides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Dynamic guides
  normalized_feature_name: dynamic guides
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-grids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Grids
  normalized_feature_name: grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Ruler and column guides
  normalized_feature_name: ruler and column guides
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-margins.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Margins
  normalized_feature_name: margins
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-measuring.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Measuring
  normalized_feature_name: measuring
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-previewmode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Preview mode
  normalized_feature_name: preview mode
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-rotatecanvas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Rotate document view
  normalized_feature_name: rotate document view
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-rulers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Rulers
  normalized_feature_name: rulers
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-snapping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Snapping
  normalized_feature_name: snapping
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.designaids-undo.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using undo, redo and history
  normalized_feature_name: using undo redo and history
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-hardwareacceleration.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Hardware acceleration
  normalized_feature_name: hardware acceleration
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-machinelearning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Affinity and Machine Learning (ML)
  normalized_feature_name: affinity and machine learning ml
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-pentablet.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using pen tablets
  normalized_feature_name: using pen tablets
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-regex.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using regular expressions in Affinity
  normalized_feature_name: using regular expressions in affinity
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-sidecar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using Sidecar
  normalized_feature_name: using sidecar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-surfacedial.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using Surface Dial
  normalized_feature_name: using surface dial
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-surfacepen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using Surface Pen
  normalized_feature_name: using surface pen
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.extras-trackpads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using a trackpad
  normalized_feature_name: using a trackpad
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-close.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Close
  normalized_feature_name: close
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-documentunits.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Document units
  normalized_feature_name: document units
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-editinotheraffinityapps.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Editing files in other Affinity apps
  normalized_feature_name: editing files in other affinity apps
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-importadobe.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Importing other Adobe documents
  normalized_feature_name: importing other adobe documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-importcad.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Importing CAD documents
  normalized_feature_name: importing cad documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-importindesign.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Importing InDesign documents
  normalized_feature_name: importing indesign documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-importpdf.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Importing PDF documents
  normalized_feature_name: importing pdf documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-newdocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Create new documents
  normalized_feature_name: create new documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.create-new-documents.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-opendocument.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Open documents
  normalized_feature_name: open documents
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.create-documents.open-indesign-documents.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pan/Scroll the document view
  normalized_feature_name: pan scroll the document view
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-save.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Save
  normalized_feature_name: save
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-templates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Document templates
  normalized_feature_name: document templates
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-view.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Viewing
  normalized_feature_name: viewing
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.getstarted-zoom.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Zooming
  normalized_feature_name: zooming
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.introduction-about-personas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About Personas
  normalized_feature_name: about personas
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.introduction-about-publisher.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: What is Affinity Publisher?
  normalized_feature_name: what is affinity publisher
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.introduction-designerpersona.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Designer Persona
  normalized_feature_name: designer persona
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.introduction-keyfeatures.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Features
  normalized_feature_name: features
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.introduction-photopersona.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Photo Persona
  normalized_feature_name: photo persona
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-create-layerfx.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using layer effects
  normalized_feature_name: using layer effects
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-3d.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: 3D Effect
  normalized_feature_name: 3d effect
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-bevelemboss.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Bevel/Emboss
  normalized_feature_name: bevel emboss
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-clroverlay.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color Overlay
  normalized_feature_name: color overlay
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-gaussianblur.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Gaussian Blur
  normalized_feature_name: gaussian blur
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-gradientoverlay.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Gradient Overlay
  normalized_feature_name: gradient overlay
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-innerglow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Inner Glow
  normalized_feature_name: inner glow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-innershadow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Inner Shadow
  normalized_feature_name: inner shadow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-outerglow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Outer Glow
  normalized_feature_name: outer glow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-outershadow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Outer Shadow
  normalized_feature_name: outer shadow
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layerfx-layerfx-outline.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Outline
  normalized_feature_name: outline
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-aboutlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About layers
  normalized_feature_name: about layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.create-and-organize-pages.manage-layers.about-layers.v1
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-adjustmentlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using adjustment layers
  normalized_feature_name: using adjustment layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-createlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating layers
  normalized_feature_name: creating layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layerblendmodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer blending
  normalized_feature_name: layer blending
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layerblendranges.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer blend ranges
  normalized_feature_name: layer blend ranges
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layerclip.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer clipping
  normalized_feature_name: layer clipping
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layercolours.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer colors
  normalized_feature_name: layer colors
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layerdropzones.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer drop zones
  normalized_feature_name: layer drop zones
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layerimage.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Image layers
  normalized_feature_name: image layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layermasks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer masking
  normalized_feature_name: layer masking
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-layeropacity.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer opacity
  normalized_feature_name: layer opacity
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-managelayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Arrange/manage layers
  normalized_feature_name: arrange manage layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-selecteditlayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Selecting and editing layers
  normalized_feature_name: selecting and editing layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.layers-taglayers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Tagging layers
  normalized_feature_name: tagging layers
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-embeddingvslinking.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Embedding vs linking
  normalized_feature_name: embedding vs linking
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-linkedservices.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Linked Services
  normalized_feature_name: linked services
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-pictureframes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Picture frames
  normalized_feature_name: picture frames
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-placeimages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Placing content
  normalized_feature_name: placing content
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-placeimagesautoflow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Autoflowing images and documents
  normalized_feature_name: autoflowing images and documents
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-placeimagesweb.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Placing images from the Web
  normalized_feature_name: placing images from the web
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-resourcemanager.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Resource Manager
  normalized_feature_name: resource manager
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.media-stockphotos.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using stock photos
  normalized_feature_name: using stock photos
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-align.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Aligning objects
  normalized_feature_name: aligning objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-compound.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating compounds
  normalized_feature_name: creating compounds
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-converttocurves.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Convert objects to curves
  normalized_feature_name: convert objects to curves
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-copypasteoptions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Copying and pasting objects
  normalized_feature_name: copying and pasting objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-distribute.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Distributing and spacing objects
  normalized_feature_name: distributing and spacing objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-duplicate.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Duplicating objects
  normalized_feature_name: duplicating objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-finding.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Finding objects
  normalized_feature_name: finding objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-group.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Grouping objects
  normalized_feature_name: grouping objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-isolating.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Isolating objects
  normalized_feature_name: isolating objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-join.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Joining objects
  normalized_feature_name: joining objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-layerstates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layer states
  normalized_feature_name: layer states
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-objectdefaults.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Object defaults
  normalized_feature_name: object defaults
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-objectgrids.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Quick Grids
  normalized_feature_name: quick grids
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-order.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Ordering objects
  normalized_feature_name: ordering objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-pinning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pinning objects
  normalized_feature_name: pinning objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-rasterizing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Rasterizing
  normalized_feature_name: rasterizing
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-rotateshear.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Rotating and shearing objects
  normalized_feature_name: rotating and shearing objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-select.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Selecting objects
  normalized_feature_name: selecting objects
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-selectbyattribute.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Selecting objects by attribute
  normalized_feature_name: selecting objects by attribute
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Styles
  normalized_feature_name: styles
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-target.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Targeting objects
  normalized_feature_name: targeting objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.objectcontrol-transform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Transforming objects
  normalized_feature_name: transforming objects
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-aboutpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About pages and spreads
  normalized_feature_name: about pages and spreads
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-adddeletepages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Add and remove pages
  normalized_feature_name: add and remove pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-addingsections.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Adding sections
  normalized_feature_name: adding sections
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-applymasterpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Applying master pages
  normalized_feature_name: applying master pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-arrangepages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Arrange pages
  normalized_feature_name: arrange pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-copyduplicatepages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Copy and duplicate pages
  normalized_feature_name: copy and duplicate pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-createmasterpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating master pages
  normalized_feature_name: creating master pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-detachlinkmasterpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Detaching and linking master pages
  normalized_feature_name: detaching and linking master pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-documentsetup.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Document setup
  normalized_feature_name: document setup
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-editmasterpagecontent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Editing master page content
  normalized_feature_name: editing master page content
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-editmasterpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Editing master pages
  normalized_feature_name: editing master pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-ghostpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About ghost pages
  normalized_feature_name: about ghost pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-masterpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About master pages
  normalized_feature_name: about master pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-migratemasterpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Migrating edited master page content
  normalized_feature_name: migrating edited master page content
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-multipagespreads.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Multiple-page spreads
  normalized_feature_name: multiple page spreads
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-navigatingpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Navigating pages
  normalized_feature_name: navigating pages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-numberingpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Page headers and footers
  normalized_feature_name: page headers and footers
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.pages-selectingviewingpages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Select and view pages
  normalized_feature_name: select and view pages
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-anchorspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Anchors panel
  normalized_feature_name: anchors panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-assetspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Assets panel
  normalized_feature_name: assets panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-bookspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Books panel
  normalized_feature_name: books panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-characterpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Character panel
  normalized_feature_name: character panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-clrpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color panel
  normalized_feature_name: color panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-constraintspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Constraints panel
  normalized_feature_name: constraints panel
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-crossrefspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Cross-References panel
  normalized_feature_name: cross references panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-fieldspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Fields panel
  normalized_feature_name: fields panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-findreplacepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Find and Replace panel
  normalized_feature_name: find and replace panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-glyphpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Glyph Browser panel
  normalized_feature_name: glyph browser panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-historypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: History panel
  normalized_feature_name: history panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-hyperlinkspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Hyperlinks panel
  normalized_feature_name: hyperlinks panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-indexpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Index panel
  normalized_feature_name: index panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-layerfxpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Quick FX panel
  normalized_feature_name: quick fx panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-layerspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Layers panel
  normalized_feature_name: layers panel
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-navigatorpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Navigator panel
  normalized_feature_name: navigator panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-notespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Notes panel
  normalized_feature_name: notes panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-pagespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pages panel
  normalized_feature_name: pages panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-paragraphpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Paragraph panel
  normalized_feature_name: paragraph panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-pinningpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pinning panel
  normalized_feature_name: pinning panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-preflightpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Preflight panel
  normalized_feature_name: preflight panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-readingorderpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Reading Order panel
  normalized_feature_name: reading order panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-statespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: States panel
  normalized_feature_name: states panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-stockpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Stock panel
  normalized_feature_name: stock panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-strokepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Stroke panel
  normalized_feature_name: stroke panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-stylespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Styles panel
  normalized_feature_name: styles panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-swatchespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Swatches panel
  normalized_feature_name: swatches panel
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-tableformatspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Table Formats panel
  normalized_feature_name: table formats panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-tablepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Table panel
  normalized_feature_name: table panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-tagspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Tags panel
  normalized_feature_name: tags panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-textframepanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text Frame panel
  normalized_feature_name: text frame panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-textstylespanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text Styles panel
  normalized_feature_name: text styles panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-tocpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Table of Contents panel
  normalized_feature_name: table of contents panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-transformpanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Transform panel
  normalized_feature_name: transform panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.panels-typographypanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Typography panel
  normalized_feature_name: typography panel
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-aboutpackaging.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About packaging
  normalized_feature_name: about packaging
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-accessiblepdfs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Accessible PDFs
  normalized_feature_name: accessible pdfs
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.accessible-pdfs.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-bleed.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Setting bleed
  normalized_feature_name: setting bleed
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-creatingpackages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating packages
  normalized_feature_name: creating packages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-export.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Export as graphic
  normalized_feature_name: export as graphic
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-exportsettings.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Export Settings
  normalized_feature_name: export settings
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-openingpackages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Opening packages
  normalized_feature_name: opening packages
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-preflight.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Preflight
  normalized_feature_name: preflight
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-print.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Print
  normalized_feature_name: print
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-publishpdffiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Publishing PDF files
  normalized_feature_name: publishing pdf files
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-resavingpackages.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Resaving modified packages
  normalized_feature_name: resaving modified packages
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.publishing-share.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Share
  normalized_feature_name: share
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tables-createcustomtables.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating table formats
  normalized_feature_name: creating table formats
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tables-createtables.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating tables
  normalized_feature_name: creating tables
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tables-edittables.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Editing tables
  normalized_feature_name: editing tables
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tables-sorttables.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Sorting tables
  normalized_feature_name: sorting tables
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-arttext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Artistic text
  normalized_feature_name: artistic text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-autocorrect.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Auto-Correct/Check Spelling While Typing
  normalized_feature_name: auto correct check spelling while typing
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-capitalisation.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Capitalization
  normalized_feature_name: capitalization
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-characters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Character formatting
  normalized_feature_name: character formatting
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-documentstats.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Document statistics
  normalized_feature_name: document statistics
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-dropcaps.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Drop caps
  normalized_feature_name: drop caps
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-fillertext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Filler text
  normalized_feature_name: filler text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-find-and-replace.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Find and replace
  normalized_feature_name: find and replace
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-fittingframetext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Fitting text to frames
  normalized_feature_name: fitting text to frames
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-flowingtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Flowing text through frames
  normalized_feature_name: flowing text through frames
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-fontmanager.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Font Manager
  normalized_feature_name: font manager
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-framesetup.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text frame setup
  normalized_feature_name: text frame setup
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-frametext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Frame text
  normalized_feature_name: frame text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-hyphenation.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Hyphenation
  normalized_feature_name: hyphenation
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-importtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Importing text
  normalized_feature_name: importing text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-indents.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Indents
  normalized_feature_name: indents
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-initialwords.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Initial words
  normalized_feature_name: initial words
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-leadingspacing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Leading and inter-paragraph spacing
  normalized_feature_name: leading and inter paragraph spacing
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-linkingtextframes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Linking text frames
  normalized_feature_name: linking text frames
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-opentype-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: OpenType font features
  normalized_feature_name: opentype font features
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-opticalalignment.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Optical alignment
  normalized_feature_name: optical alignment
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-paragraphs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Paragraph formatting
  normalized_feature_name: paragraph formatting
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-pathtext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text on a path
  normalized_feature_name: text on a path
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-shapetext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Shape text
  normalized_feature_name: shape text
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-specialcharacters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Special characters and glyphs
  normalized_feature_name: special characters and glyphs
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-spelling.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Spelling
  normalized_feature_name: spelling
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-superscriptssubscripts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: About superscripts and subscripts
  normalized_feature_name: about superscripts and subscripts
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-tabs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Tabs
  normalized_feature_name: tabs
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-text-bulletsandnumbering.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Bullets and numbering
  normalized_feature_name: bullets and numbering
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-text-general.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Working with text
  normalized_feature_name: working with text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-text-multilevellists.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using multi-level lists
  normalized_feature_name: using multi level lists
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-textediting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Editing text
  normalized_feature_name: editing text
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-textmarks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text marks
  normalized_feature_name: text marks
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-textstyles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Using text styles
  normalized_feature_name: using text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-textstyles-create.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Creating and managing text styles
  normalized_feature_name: creating and managing text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-textstyles-remove.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Removing text styles
  normalized_feature_name: removing text styles
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-textstyles-types.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text style types
  normalized_feature_name: text style types
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-trackingkerning.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Tracking and kerning
  normalized_feature_name: tracking and kerning
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-variablefonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Variable fonts
  normalized_feature_name: variable fonts
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.text-wraptext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Text wrapping
  normalized_feature_name: text wrapping
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-arrow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Arrow Tool
  normalized_feature_name: arrow tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-arttext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Artistic Text Tool
  normalized_feature_name: artistic text tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-calloutellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Callout Ellipse Tool
  normalized_feature_name: callout ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-calloutroundedrectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Callout Rounded Rectangle Tool
  normalized_feature_name: callout rounded rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-cat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Cat Tool
  normalized_feature_name: cat tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-cloud.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Cloud Tool
  normalized_feature_name: cloud tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-clrpicker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Color Picker Tool
  normalized_feature_name: color picker tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-cog.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Cog Tool
  normalized_feature_name: cog tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-crescent.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Crescent Tool
  normalized_feature_name: crescent tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-crop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Vector Crop Tool
  normalized_feature_name: vector crop tool
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-datamergenode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Data Merge Layout Tool
  normalized_feature_name: data merge layout tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-diamond.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Diamond Tool
  normalized_feature_name: diamond tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-dnut.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Donut Tool
  normalized_feature_name: donut tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-doublestar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Double Star Tool
  normalized_feature_name: double star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-ellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Ellipse Tool
  normalized_feature_name: ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-frametext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Frame Text Tool
  normalized_feature_name: frame text tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-gradient.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Gradient Tool
  normalized_feature_name: gradient tool
  studio_surface: StudioColorPipeline
  primitive_domain: color
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-heart.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Heart Tool
  normalized_feature_name: heart tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-measure.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Measure Tool
  normalized_feature_name: measure tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-move.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Move Tool
  normalized_feature_name: move tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-node.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Node Tool
  normalized_feature_name: node tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: View Tool
  normalized_feature_name: view tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-pen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pen Tool
  normalized_feature_name: pen tool
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-pictureframeellipse.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Picture Frame Ellipse Tool
  normalized_feature_name: picture frame ellipse tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-pictureframerectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Picture Frame Rectangle Tool
  normalized_feature_name: picture frame rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-pie.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Pie Tool
  normalized_feature_name: pie tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-placeimage.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Place Tool
  normalized_feature_name: place tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-polygon.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Polygon Tool
  normalized_feature_name: polygon tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-qrcode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: QR Code Tool
  normalized_feature_name: qr code tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-rectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Rectangle Tool
  normalized_feature_name: rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-roundedrectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Rounded Rectangle Tool
  normalized_feature_name: rounded rectangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-segment.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Segment Tool
  normalized_feature_name: segment tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-spiral.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Spiral Tool
  normalized_feature_name: spiral tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-squarestar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Square Star Tool
  normalized_feature_name: square star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-star.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Star Tool
  normalized_feature_name: star tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-stylepicker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Style Picker Tool
  normalized_feature_name: style picker tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-tabletext.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Table Tool
  normalized_feature_name: table tool
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-tear.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Tear Tool
  normalized_feature_name: tear tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-transparency.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Transparency Tool
  normalized_feature_name: transparency tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-trapezoid.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Trapezoid Tool
  normalized_feature_name: trapezoid tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-triangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Triangle Tool
  normalized_feature_name: triangle tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.tools-tools-zoom.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Zoom Tool
  normalized_feature_name: zoom tool
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-accessibility.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Accessibility
  normalized_feature_name: accessibility
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-contextbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Context toolbar
  normalized_feature_name: context toolbar
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-customizingshortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Keyboard shortcuts
  normalized_feature_name: keyboard shortcuts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-customizingtoolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Toolbar
  normalized_feature_name: toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-customizingtoolspanel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Tools panel
  normalized_feature_name: tools panel
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-customizingworkspace.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Workspace
  normalized_feature_name: workspace
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-expressions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Expressions for field input
  normalized_feature_name: expressions for field input
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-interface.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Interface Visual Reference
  normalized_feature_name: interface visual reference
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-personatoolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Persona Toolbar
  normalized_feature_name: persona toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_distinctive_candidate
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers:
  - persona
  uniqueness_claim_status: distinctive_candidate_needs_source_page_confirmation
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Settings (Preferences)
  normalized_feature_name: settings preferences
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-shortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Keyboard shortcuts
  normalized_feature_name: keyboard shortcuts
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_exact_name_overlap_with_adobe
  - affinity_shared_primitive_overlap_with_adobe
  adobe_overlap:
    exact_normalized_name_matches:
    - osd.indesign.indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts.v1
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: exact_name_only_not_behavioral_equivalence
  affinity_distinctive_markers: []
  uniqueness_claim_status: not_claimed
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-toolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Toolbar
  normalized_feature_name: toolbar
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-uiappearance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Changing the UI appearance
  normalized_feature_name: changing the ui appearance
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
- affinity_row_id: osd.affinity.affinity-publisher.desktop.leaf.workspace-workspacemodes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  affinity_source_app: Affinity Publisher 2 desktop
  feature_name: Application and document windows
  normalized_feature_name: application and document windows
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  relation_class:
  - affinity_source_row
  - affinity_shared_primitive_overlap_with_adobe
  - affinity_current_corpus_name_absent_from_adobe
  adobe_overlap:
    exact_normalized_name_matches: []
    shared_primitive_adobe_apps:
    - illustrator
    - indesign
    - photoshop
    equivalence_claim: none
  affinity_distinctive_markers: []
  uniqueness_claim_status: current_corpus_name_absent_from_adobe
  verification_needed:
  - direct_source_page_comparison_before_claiming_unique_behavior
  - command_contract_mapping_before_implementation
```

</topic>

<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for the generated overlap and Affinity dedupe map.">

### [SFR-CROSS-APP-OVERLAP-AFFINITY-DEDUPE.sources] Sources

```yaml
sources:
- id: DEDUPE-S01
  path: 33-online-source-distilled-feature-ledger.md
  note: Source-distilled merge contract.
- id: DEDUPE-R01
  path: 39-photoshop-source-distilled-feature-rows.md
  note: photoshop source-distilled feature rows.
- id: DEDUPE-R02
  path: 40-indesign-source-distilled-feature-rows.md
  note: indesign source-distilled feature rows.
- id: DEDUPE-R03
  path: 41-illustrator-source-distilled-feature-rows.md
  note: illustrator source-distilled feature rows.
- id: DEDUPE-R04
  path: 42-affinity-source-distilled-feature-rows.md
  note: affinity source-distilled feature rows.
- id: DEDUPE-R05
  path: 43-figma-source-distilled-feature-rows.md
  note: figma source-distilled feature rows.
- id: DEDUPE-D01
  path: 34-photoshop-source-distilled-domain-ledger.md
  note: photoshop source-distilled domain ledger.
- id: DEDUPE-D02
  path: 35-indesign-source-distilled-domain-ledger.md
  note: indesign source-distilled domain ledger.
- id: DEDUPE-D03
  path: 36-illustrator-source-distilled-domain-ledger.md
  note: illustrator source-distilled domain ledger.
- id: DEDUPE-D04
  path: 37-affinity-source-distilled-domain-ledger.md
  note: affinity source-distilled domain ledger.
- id: DEDUPE-D05
  path: 38-figma-source-distilled-domain-ledger.md
  note: figma source-distilled domain ledger.
```

</topic>
