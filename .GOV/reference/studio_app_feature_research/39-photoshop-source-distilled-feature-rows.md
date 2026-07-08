---
file_id: 39-photoshop-source-distilled-feature-rows
file_kind: source_distilled_feature_rows
topic_id: SFR-PHOTOSHOP-SOURCE-DISTILLED-FEATURE-ROWS
title: Photoshop Source Distilled Feature Rows
status: draft
updated_at: '2026-07-05'
app_key: photoshop
source_cards: 15-photoshop-feature-use-cards.md
source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
feature_row_count: 441
source_ref_count: 882
---

## [SFR-PHOTOSHOP-SOURCE-DISTILLED-FEATURE-ROWS] Photoshop Source Distilled Feature Rows

<topic id="feature-row-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and source policy for generated source-distilled feature rows.">

### [SFR-PHOTOSHOP-SOURCE-DISTILLED-FEATURE-ROWS.coverage] Feature Row Coverage

```yaml
coverage:
  app_key: photoshop
  source_cards: 15-photoshop-feature-use-cards.md
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_row_count: 441
  distillation_status: online_source_distilled_feature_rows
  installed_exports_role: optional_enrichment_only
  naming_rule: Vendor product names remain source/provenance and compatibility references only.
  manual_handoff_rule: Promote manual_topic_candidate into the internal Studio UserManual in the same change that implements
    the feature behavior.
```

</topic>

<topic id="source-distilled-feature-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable source-distilled feature rows.">

### [SFR-PHOTOSHOP-SOURCE-DISTILLED-FEATURE-ROWS.rows] Source Distilled Feature Rows

```yaml
source_distilled_feature_rows:
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.add-frames-to-animations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.add-frames-to-animations.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.add-frames-to-animations
  feature_name: Add frames to animations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add frames to animations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Add frames to animations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Add frames to animations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/add-frames-to-animations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.choose-a-frame-disposal-method.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.choose-a-frame-disposal-method.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.choose-a-frame-disposal-method
  feature_name: Choose a frame disposal method
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Choose a frame disposal method to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Choose a frame disposal method" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Choose a frame disposal method
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/choose-a-frame-disposal-method.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.copy-frames-with-layer-properties.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.copy-frames-with-layer-properties.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.copy-frames-with-layer-properties
  feature_name: Copy frames with layer properties
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Copy frames with layer properties to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Copy frames with layer properties" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Copy frames with layer properties
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/copy-frames-with-layer-properties.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.create-frame-based-animations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.create-frame-based-animations.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.create-frame-based-animations
  feature_name: Create frame-based animations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create frame-based animations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Create frame-based animations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Create frame-based animations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/create-frame-based-animations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.create-frames-using-tweening.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.create-frames-using-tweening.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.create-frames-using-tweening
  feature_name: Create frames using tweening
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create frames using tweening to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Create frames using tweening" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Create frames using tweening
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/create-frames-using-tweening.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.delete-animations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.delete-animations.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.delete-animations
  feature_name: Delete animations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete animations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Delete animations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Delete animations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/delete-animations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.edit-animation-frames.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.edit-animation-frames.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.edit-animation-frames
  feature_name: Edit animation frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit animation frames to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Edit animation frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Edit animation frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/edit-animation-frames.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.manage-layer-visibility-in-animation-frames.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.manage-layer-visibility-in-animation-frames.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.manage-layer-visibility-in-animation-frames
  feature_name: Manage layer visibility in animation frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage layer visibility in animation frames to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Manage layer visibility in animation frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Manage layer visibility in animation frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/manage-layer-visibility-in-animation-frames.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.select-animation-frames.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.select-animation-frames.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.select-animation-frames
  feature_name: Select animation frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select animation frames to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Select animation frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Select animation frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/select-animation-frames.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.specify-a-delay-time-in-frame-animations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.specify-a-delay-time-in-frame-animations.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.specify-a-delay-time-in-frame-animations
  feature_name: Specify a delay time in frame animations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Specify a delay time in frame animations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Specify a delay time in frame animations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Specify a delay time in frame animations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/specify-a-delay-time-in-frame-animations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.specify-looping-in-frame-animations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.specify-looping-in-frame-animations.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.specify-looping-in-frame-animations
  feature_name: Specify looping in frame animations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Specify looping in frame animations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Specify looping in frame animations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Specify looping in frame animations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/specify-looping-in-frame-animations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.create-animation-frames.unify-layer-properties-in-animation-frames.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.create-animation-frames.unify-layer-properties-in-animation-frames.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.create-animation-frames.unify-layer-properties-in-animation-frames
  feature_name: Unify layer properties in animation frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: create-animation-frames
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Unify layer properties in animation frames to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Unify layer properties in animation frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Unify layer properties in animation frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/create-animation-frames/unify-layer-properties-in-animation-frames.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.use-keyframes.create-timeline-animation-workflow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.use-keyframes.create-timeline-animation-workflow.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.use-keyframes.create-timeline-animation-workflow
  feature_name: Create a timeline animation workflow
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: use-keyframes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a timeline animation workflow to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Create a timeline animation workflow" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Create a timeline animation workflow
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/use-keyframes/create-timeline-animation-workflow.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.add-video-and-animation.use-keyframes.overview-of-animating-layer-properties.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.add-video-and-animation.use-keyframes.overview-of-animating-layer-properties.v0
  source_feature_id: photoshop.leaf.add-video-and-animation.use-keyframes.overview-of-animating-layer-properties
  feature_name: Overview of animating layer properties
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: add-video-and-animation
  source_subcategory: use-keyframes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of animating layer properties to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of animating layer properties" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Overview of animating layer properties
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/add-video-and-animation/use-keyframes/overview-of-animating-layer-properties.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.choose-a-cmyk-equivalent-for-a-non-printable-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.choose-a-cmyk-equivalent-for-a-non-printable-color.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.choose-a-cmyk-equivalent-for-a-non-printable-color
  feature_name: Choose a CMYK equivalent for a non-printable color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Choose a CMYK equivalent for a non-printable color to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Choose a CMYK equivalent for a non-printable color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Choose a CMYK equivalent for a non-printable color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/choose-a-cmyk-equivalent-for-a-non-printable-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.choose-a-color-while-painting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.choose-a-color-while-painting.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.choose-a-color-while-painting
  feature_name: Choose a color while painting
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Choose a color while painting to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Choose a color while painting" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Choose a color while painting
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/choose-a-color-while-painting.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.choose-a-spot-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.choose-a-spot-color.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.choose-a-spot-color
  feature_name: Choose a spot color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Choose a spot color to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Choose a spot color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Choose a spot color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/choose-a-spot-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.choose-colors-with-the-adobe-color-picker.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.choose-colors-with-the-adobe-color-picker.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.choose-colors-with-the-adobe-color-picker
  feature_name: Choose colors with the Adobe Color Picker
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Choose colors with the Adobe Color Picker to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Choose colors with the Adobe Color Picker" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Choose colors with the Adobe Color Picker
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/choose-colors-with-the-adobe-color-picker.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.choose-websafe-colors.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.choose-websafe-colors.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.choose-websafe-colors
  feature_name: Choose web-safe colors
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Choose web-safe colors to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Choose web-safe colors" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Choose web-safe colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/choose-websafe-colors.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.set-foreground-and-background-colors.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.set-foreground-and-background-colors.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.set-foreground-and-background-colors
  feature_name: Set foreground and background colors
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set foreground and background colors to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Set foreground and background colors" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Set foreground and background colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/set-foreground-and-background-colors.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.choose-colors.spot-color-libraries.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.choose-colors.spot-color-libraries.v0
  source_feature_id: photoshop.leaf.adjust-color.choose-colors.spot-color-libraries
  feature_name: Spot color libraries
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: choose-colors
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: optional_integration
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Spot color libraries to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Spot color libraries" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Spot color libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/choose-colors/spot-color-libraries.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-corrections.apply-a-hue-or-saturation-adjustment.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-corrections.apply-a-hue-or-saturation-adjustment.v0
  source_feature_id: photoshop.leaf.adjust-color.color-corrections.apply-a-hue-or-saturation-adjustment
  feature_name: Apply a Hue or Saturation adjustment
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-corrections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply a Hue or Saturation adjustment to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Apply a Hue or Saturation adjustment" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Apply a Hue or Saturation adjustment
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-corrections/apply-a-hue-or-saturation-adjustment.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-corrections.colorize-a-grayscale-image-or-create-a-monotone-effect.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-corrections.colorize-a-grayscale-image-or-create-a-monotone-effect.v0
  source_feature_id: photoshop.leaf.adjust-color.color-corrections.colorize-a-grayscale-image-or-create-a-monotone-effect
  feature_name: Colorize a grayscale image or create a monotone effect
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-corrections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Colorize a grayscale image or create a monotone effect to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Colorize a grayscale image or create a monotone effect" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Colorize a grayscale image or create a monotone effect
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-corrections/colorize-a-grayscale-image-or-create-a-monotone-effect.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-effects-techniques.apply-gradient-fill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-effects-techniques.apply-gradient-fill.v0
  source_feature_id: photoshop.leaf.adjust-color.color-effects-techniques.apply-gradient-fill
  feature_name: Apply a gradient fill
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-effects-techniques
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply a gradient fill to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Apply a gradient fill" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Apply a gradient fill
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-effects-techniques/apply-gradient-fill.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-effects-techniques.convert-a-color-image-to-black-and-white.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-effects-techniques.convert-a-color-image-to-black-and-white.v0
  source_feature_id: photoshop.leaf.adjust-color.color-effects-techniques.convert-a-color-image-to-black-and-white
  feature_name: Convert a color image to black and white
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-effects-techniques
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert a color image to black and white to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Convert a color image to black and white" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert a color image to black and white
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-effects-techniques/convert-a-color-image-to-black-and-white.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-effects-techniques.edit-a-gradient.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-effects-techniques.edit-a-gradient.v0
  source_feature_id: photoshop.leaf.adjust-color.color-effects-techniques.edit-a-gradient
  feature_name: Edit a gradient
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-effects-techniques
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit a gradient to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Edit a gradient" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Edit a gradient
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-effects-techniques/edit-a-gradient.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-modes.conversion-options-for-indexed-color-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-modes.conversion-options-for-indexed-color-images.v0
  source_feature_id: photoshop.leaf.adjust-color.color-modes.conversion-options-for-indexed-color-images
  feature_name: Conversion options for indexed-color images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-modes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Conversion options for indexed-color images to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Conversion options for indexed-color images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Conversion options for indexed-color images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-modes/conversion-options-for-indexed-color-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-modes.convert-a-bitmap-mode-image-to-grayscale-mode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-modes.convert-a-bitmap-mode-image-to-grayscale-mode.v0
  source_feature_id: photoshop.leaf.adjust-color.color-modes.convert-a-bitmap-mode-image-to-grayscale-mode
  feature_name: Convert a Bitmap mode image to Grayscale mode
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-modes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert a Bitmap mode image to Grayscale mode to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Convert a Bitmap mode image to Grayscale mode" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert a Bitmap mode image to Grayscale mode
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-modes/convert-a-bitmap-mode-image-to-grayscale-mode.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-modes.convert-a-color-photo-to-grayscale-mode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-modes.convert-a-color-photo-to-grayscale-mode.v0
  source_feature_id: photoshop.leaf.adjust-color.color-modes.convert-a-color-photo-to-grayscale-mode
  feature_name: Convert a color photo to Grayscale mode
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-modes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert a color photo to Grayscale mode to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Convert a color photo to Grayscale mode" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert a color photo to Grayscale mode
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-modes/convert-a-color-photo-to-grayscale-mode.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-modes.convert-a-grayscale-or-rgb-image-to-indexed-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-modes.convert-a-grayscale-or-rgb-image-to-indexed-color.v0
  source_feature_id: photoshop.leaf.adjust-color.color-modes.convert-a-grayscale-or-rgb-image-to-indexed-color
  feature_name: Convert a grayscale or RGB image to indexed color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-modes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert a grayscale or RGB image to indexed color to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Convert a grayscale or RGB image to indexed color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert a grayscale or RGB image to indexed color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-modes/convert-a-grayscale-or-rgb-image-to-indexed-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-modes.convert-an-image-to-another-color-mode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-modes.convert-an-image-to-another-color-mode.v0
  source_feature_id: photoshop.leaf.adjust-color.color-modes.convert-an-image-to-another-color-mode
  feature_name: Convert an image to another color mode
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-modes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert an image to another color mode to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Convert an image to another color mode" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert an image to another color mode
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-modes/convert-an-image-to-another-color-mode.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-modes.convert-an-image-to-bitmap-mode.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-modes.convert-an-image-to-bitmap-mode.v0
  source_feature_id: photoshop.leaf.adjust-color.color-modes.convert-an-image-to-bitmap-mode
  feature_name: Convert an image to Bitmap mode
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-modes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert an image to Bitmap mode to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Convert an image to Bitmap mode" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert an image to Bitmap mode
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-modes/convert-an-image-to-bitmap-mode.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-profiles.about-color-profiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-profiles.about-color-profiles.v0
  source_feature_id: photoshop.leaf.adjust-color.color-profiles.about-color-profiles
  feature_name: About color profiles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-profiles
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About color profiles to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "About color profiles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / About color profiles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-profiles/about-color-profiles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-profiles.change-color-profile-for-documents.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-profiles.change-color-profile-for-documents.v0
  source_feature_id: photoshop.leaf.adjust-color.color-profiles.change-color-profile-for-documents
  feature_name: Change color profile for documents
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-profiles
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change color profile for documents to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Change color profile for documents" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Change color profile for documents
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-profiles/change-color-profile-for-documents.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.color-profiles.embed-color-profiles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.color-profiles.embed-color-profiles.v0
  source_feature_id: photoshop.leaf.adjust-color.color-profiles.embed-color-profiles
  feature_name: Embed color profiles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: color-profiles
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Embed color profiles to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Embed color profiles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Embed color profiles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/color-profiles/embed-color-profiles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.selective-color-adjustments.match-color-between-two-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.selective-color-adjustments.match-color-between-two-images.v0
  source_feature_id: photoshop.leaf.adjust-color.selective-color-adjustments.match-color-between-two-images
  feature_name: Match color between two images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: selective-color-adjustments
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Match color between two images to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Match color between two images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Match color between two images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/selective-color-adjustments/match-color-between-two-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.selective-color-adjustments.match-color-in-different-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.selective-color-adjustments.match-color-in-different-images.v0
  source_feature_id: photoshop.leaf.adjust-color.selective-color-adjustments.match-color-in-different-images
  feature_name: Match color in different images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: selective-color-adjustments
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Match color in different images to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Match color in different images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Match color in different images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/selective-color-adjustments/match-color-in-different-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.selective-color-adjustments.match-color-of-two-layers-in-the-same-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.selective-color-adjustments.match-color-of-two-layers-in-the-same-image.v0
  source_feature_id: photoshop.leaf.adjust-color.selective-color-adjustments.match-color-of-two-layers-in-the-same-image
  feature_name: Match color of two layers in the same image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: selective-color-adjustments
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Match color of two layers in the same image to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Match color of two layers in the same image" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Match color of two layers in the same image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/selective-color-adjustments/match-color-of-two-layers-in-the-same-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.selective-color-adjustments.replace-object-colors-by-applying-a-hue-or-saturation-adjustment.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.selective-color-adjustments.replace-object-colors-by-applying-a-hue-or-saturation-adjustment.v0
  source_feature_id: photoshop.leaf.adjust-color.selective-color-adjustments.replace-object-colors-by-applying-a-hue-or-saturation-adjustment
  feature_name: Replace Object Colors by Applying a Hue or Saturation Adjustment
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: selective-color-adjustments
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace Object Colors by Applying a Hue or Saturation Adjustment to create, arrange, combine, or non-destructively control visual layer state
    imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Replace Object Colors by Applying a Hue or Saturation Adjustment" without needing hidden
    source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Replace Object Colors by Applying a Hue or Saturation Adjustment
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/selective-color-adjustments/replace-object-colors-by-applying-a-hue-or-saturation-adjustment.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.adjust-color.selective-color-adjustments.save-and-apply-settings-in-match-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.adjust-color.selective-color-adjustments.save-and-apply-settings-in-match-color.v0
  source_feature_id: photoshop.leaf.adjust-color.selective-color-adjustments.save-and-apply-settings-in-match-color
  feature_name: Save and apply settings in Match Color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: adjust-color
  source_subcategory: selective-color-adjustments
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save and apply settings in Match Color to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Save and apply settings in Match Color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Save and apply settings in Match Color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/adjust-color/selective-color-adjustments/save-and-apply-settings-in-match-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.app-integrations.access-adobe-express-templates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.app-integrations.access-adobe-express-templates.v0
  source_feature_id: photoshop.leaf.app-integrations.access-adobe-express-templates
  feature_name: Access Adobe Express Templates
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: app-integrations
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Access Adobe Express Templates to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Access Adobe Express Templates" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Access Adobe Express Templates
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/app-integrations/access-adobe-express-templates.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.app-integrations.open-photoshop-files-in-illustrator.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.app-integrations.open-photoshop-files-in-illustrator.v0
  source_feature_id: photoshop.leaf.app-integrations.open-photoshop-files-in-illustrator
  feature_name: Open Photoshop files in Illustrator
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: app-integrations
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Open Photoshop files in Illustrator to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Open Photoshop files in Illustrator" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Open Photoshop files in Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/app-integrations/open-photoshop-files-in-illustrator.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.app-integrations.refine-firefly-generated-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.app-integrations.refine-firefly-generated-images.v0
  source_feature_id: photoshop.leaf.app-integrations.refine-firefly-generated-images
  feature_name: Refine Firefly-generated images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: app-integrations
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Refine Firefly-generated images to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Refine Firefly-generated images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Refine Firefly-generated images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/app-integrations/refine-firefly-generated-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.app-integrations.transform-image-to-video.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.app-integrations.transform-image-to-video.v0
  source_feature_id: photoshop.leaf.app-integrations.transform-image-to-video
  feature_name: Transform image to video
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: app-integrations
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transform image to video to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Transform image to video" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Transform image to video
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/app-integrations/transform-image-to-video.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.create-a-new-preset-brush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.create-a-new-preset-brush.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.create-a-new-preset-brush
  feature_name: Create a new preset brush
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a new preset brush to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create a new preset brush" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create a new preset brush
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/create-a-new-preset-brush.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.create-brush-set-painting-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.create-brush-set-painting-options.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.create-brush-set-painting-options
  feature_name: Create a brush and set painting options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a brush and set painting options to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create a brush and set painting options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create a brush and set painting options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/create-brush-set-painting-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.create-brush-tip-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.create-brush-tip-image.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.create-brush-tip-image
  feature_name: Create a brush tip from an image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a brush tip from an image to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create a brush tip from an image" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create a brush tip from an image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/create-brush-tip-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.create-preset-brush-groups.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.create-preset-brush-groups.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.create-preset-brush-groups
  feature_name: Create preset brush groups
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create preset brush groups to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create preset brush groups" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create preset brush groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/create-preset-brush-groups.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.delete-preset-brushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.delete-preset-brushes.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.delete-preset-brushes
  feature_name: Delete preset brushes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete preset brushes to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Delete preset brushes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Delete preset brushes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/delete-preset-brushes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.display-brush-panel-brush-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.display-brush-panel-brush-options.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.display-brush-panel-brush-options
  feature_name: Display the Brush Settings panel and brush options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Display the Brush Settings panel and brush options to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Display the Brush Settings panel and brush options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Display the Brush Settings panel and brush options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/display-brush-panel-brush-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.get-started-with-brush-presets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.get-started-with-brush-presets.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.get-started-with-brush-presets
  feature_name: Get started with brush presets
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get started with brush presets to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Get started with brush presets" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Get started with brush presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/get-started-with-brush-presets.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.import-brushes-brush-packs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.import-brushes-brush-packs.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.import-brushes-brush-packs
  feature_name: Import brushes and brush packs
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import brushes and brush packs to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Import brushes and brush packs" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Import brushes and brush packs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/import-brushes-brush-packs.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.rename-preset-brushes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.rename-preset-brushes.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.rename-preset-brushes
  feature_name: Rename preset brushes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rename preset brushes to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Rename preset brushes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Rename preset brushes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/rename-preset-brushes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.brushes-presets.select-a-preset-brush.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.brushes-presets.select-a-preset-brush.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.brushes-presets.select-a-preset-brush
  feature_name: Select a preset brush
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: brushes-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select a preset brush to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Select a preset brush" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select a preset brush
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/brushes-presets/select-a-preset-brush.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.create-fill-with-patterns.create-a-new-pattern.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.create-fill-with-patterns.create-a-new-pattern.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.create-fill-with-patterns.create-a-new-pattern
  feature_name: Create a new pattern
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: create-fill-with-patterns
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a new pattern to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create a new pattern" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create a new pattern
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/create-fill-with-patterns/create-a-new-pattern.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.create-fill-with-patterns.pattern-preview-best-practices.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.create-fill-with-patterns.pattern-preview-best-practices.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.create-fill-with-patterns.pattern-preview-best-practices
  feature_name: Pattern Preview best practices
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: create-fill-with-patterns
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: optional_integration
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Pattern Preview best practices to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Pattern Preview best practices" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Pattern Preview best practices
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/create-fill-with-patterns/pattern-preview-best-practices.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.content-aware-fills.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.content-aware-fills.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.content-aware-fills
  feature_name: Use Content-Aware, Pattern, or History fills
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Content-Aware, Pattern, or History fills to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Use Content-Aware, Pattern, or History fills" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Use Content-Aware, Pattern, or History fills
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/content-aware-fills.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.create-a-new-layer-when-brushing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.create-a-new-layer-when-brushing.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.create-a-new-layer-when-brushing
  feature_name: Create a new layer when brushing
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a new layer when brushing to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create a new layer when brushing" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create a new layer when brushing
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/create-a-new-layer-when-brushing.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-paint-bucket-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-paint-bucket-tool.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-paint-bucket-tool
  feature_name: Fill adjacent areas with similar colors
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fill adjacent areas with similar colors to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Fill adjacent areas with similar colors" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Fill adjacent areas with similar colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/fill-paint-bucket-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-selection-layer-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-selection-layer-color.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-selection-layer-color
  feature_name: Fill a selection or layer with color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fill a selection or layer with color to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Fill a selection or layer with color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Fill a selection or layer with color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/fill-selection-layer-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-work-canvas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-work-canvas.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.fill-work-canvas
  feature_name: Fill the work canvas
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fill the work canvas to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Fill the work canvas" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Fill the work canvas
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/fill-work-canvas.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.painting-tools-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.painting-tools-overview.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.painting-tools-overview
  feature_name: Painting tools overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Painting tools overview to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Painting tools overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Painting tools overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/painting-tools-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.stroke-selection-layer-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.stroke-selection-layer-color.v0
  source_feature_id: photoshop.leaf.apply-painting-techniques.fill-objects-selections-layers.stroke-selection-layer-color
  feature_name: Stroke a selection or layer with color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: apply-painting-techniques
  source_subcategory: fill-objects-selections-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Stroke a selection or layer with color to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Stroke a selection or layer with color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Stroke a selection or layer with color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/apply-painting-techniques/fill-objects-selections-layers/stroke-selection-layer-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.automation-settings-and-presets.actions-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.automation-settings-and-presets.actions-overview.v0
  source_feature_id: photoshop.leaf.automate-tasks.automation-settings-and-presets.actions-overview
  feature_name: Overview of Actions
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: automation-settings-and-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of Actions to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of Actions" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Overview of Actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/automation-settings-and-presets/actions-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.automation-settings-and-presets.apply-actions-in-the-actions-panel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.automation-settings-and-presets.apply-actions-in-the-actions-panel.v0
  source_feature_id: photoshop.leaf.automate-tasks.automation-settings-and-presets.apply-actions-in-the-actions-panel
  feature_name: Apply actions in the Actions panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: automation-settings-and-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply actions in the Actions panel to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Apply actions in the Actions panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Apply actions in the Actions panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/automation-settings-and-presets/apply-actions-in-the-actions-panel.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.automation-settings-and-presets.use-the-actions-panel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.automation-settings-and-presets.use-the-actions-panel.v0
  source_feature_id: photoshop.leaf.automate-tasks.automation-settings-and-presets.use-the-actions-panel
  feature_name: Use the Actions panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: automation-settings-and-presets
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use the Actions panel to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use the Actions panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use the Actions panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/automation-settings-and-presets/use-the-actions-panel.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.add-commands-to-an-action.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.add-commands-to-an-action.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.add-commands-to-an-action
  feature_name: Add commands to an action
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add commands to an action to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Add commands to an action" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Add commands to an action
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/add-commands-to-an-action.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.change-settings-when-playing-an-action.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.change-settings-when-playing-an-action.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.change-settings-when-playing-an-action
  feature_name: Change settings when playing an action
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change settings when playing an action to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Change settings when playing an action" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Change settings when playing an action
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/change-settings-when-playing-an-action.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.exclude-commands-from-an-action.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.exclude-commands-from-an-action.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.exclude-commands-from-an-action
  feature_name: Exclude commands from an action
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Exclude commands from an action to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Exclude commands from an action" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Exclude commands from an action
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/exclude-commands-from-an-action.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.insert-a-non-recordable-menu-command.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.insert-a-non-recordable-menu-command.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.insert-a-non-recordable-menu-command
  feature_name: Insert a non-recordable menu command
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Insert a non-recordable menu command to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Insert a non-recordable menu command" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Insert a non-recordable menu command
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/insert-a-non-recordable-menu-command.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.insert-a-stop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.insert-a-stop.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.insert-a-stop
  feature_name: Insert a stop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Insert a stop to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Insert a stop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Insert a stop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/insert-a-stop.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.overwrite-a-single-command.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.overwrite-a-single-command.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.overwrite-a-single-command
  feature_name: Overwrite a single command
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overwrite a single command to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Overwrite a single command" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Overwrite a single command
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/overwrite-a-single-command.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.rearrange-commands-within-an-action.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.rearrange-commands-within-an-action.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.rearrange-commands-within-an-action
  feature_name: Rearrange commands within an action
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rearrange commands within an action to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Rearrange commands within an action" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Rearrange commands within an action
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/rearrange-commands-within-an-action.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.record-a-path.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.record-a-path.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.record-a-path
  feature_name: Record a path
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Record a path to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Record a path" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Record a path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/record-a-path.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.record-an-action.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.record-an-action.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.record-an-action
  feature_name: Record an action
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Record an action to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Record an action" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Record an action
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/record-an-action.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.create-record-actions.record-an-action-again.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.create-record-actions.record-an-action-again.v0
  source_feature_id: photoshop.leaf.automate-tasks.create-record-actions.record-an-action-again
  feature_name: Record an action again
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: create-record-actions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Record an action again to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Record an action again" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Record an action again
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/create-record-actions/record-an-action-again.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-and-droplet-processing-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-and-droplet-processing-options.v0
  source_feature_id: photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-and-droplet-processing-options
  feature_name: Batch and droplet processing options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: process-a-batch-of-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Batch and droplet processing options to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Batch and droplet processing options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Batch and droplet processing options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/process-a-batch-of-files/batch-and-droplet-processing-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-process-files.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-process-files.v0
  source_feature_id: photoshop.leaf.automate-tasks.process-a-batch-of-files.batch-process-files
  feature_name: Batch-process files
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: process-a-batch-of-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Batch-process files to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Batch-process files" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Batch-process files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/process-a-batch-of-files/batch-process-files.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.convert-files-with-the-image-processor.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.process-a-batch-of-files.convert-files-with-the-image-processor.v0
  source_feature_id: photoshop.leaf.automate-tasks.process-a-batch-of-files.convert-files-with-the-image-processor
  feature_name: Convert files with the Image Processor
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: process-a-batch-of-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert files with the Image Processor to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Convert files with the Image Processor" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Convert files with the Image Processor
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/process-a-batch-of-files/convert-files-with-the-image-processor.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.create-a-droplet-from-an-action.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.process-a-batch-of-files.create-a-droplet-from-an-action.v0
  source_feature_id: photoshop.leaf.automate-tasks.process-a-batch-of-files.create-a-droplet-from-an-action
  feature_name: Create a droplet from an action
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: process-a-batch-of-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a droplet from an action to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Create a droplet from an action" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Create a droplet from an action
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/process-a-batch-of-files/create-a-droplet-from-an-action.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.automate-tasks.process-a-batch-of-files.image-processor-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.automate-tasks.process-a-batch-of-files.image-processor-overview.v0
  source_feature_id: photoshop.leaf.automate-tasks.process-a-batch-of-files.image-processor-overview
  feature_name: Image Processor overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: automate-tasks
  source_subcategory: process-a-batch-of-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Image Processor overview to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Image Processor overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Image Processor overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/automate-tasks/process-a-batch-of-files/image-processor-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.add-layer-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.add-layer-styles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.add-layer-styles
  feature_name: Add layer styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add layer styles to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Add layer styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Add layer styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/add-layer-styles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.convert-layer-styles-to-image-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.convert-layer-styles-to-image-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.convert-layer-styles-to-image-layers
  feature_name: Convert layer styles to image layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert layer styles to image layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Convert layer styles to image layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Convert layer styles to image layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/convert-layer-styles-to-image-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.copy-and-paste-layer-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.copy-and-paste-layer-styles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.copy-and-paste-layer-styles
  feature_name: Copy and paste layer styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Copy and paste layer styles to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Copy and paste layer styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Copy and paste layer styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/copy-and-paste-layer-styles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.display-or-hide-layer-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.display-or-hide-layer-styles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.display-or-hide-layer-styles
  feature_name: Display or hide layer styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Display or hide layer styles to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Display or hide layer styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Display or hide layer styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/display-or-hide-layer-styles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.import-preset-style-libraries.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.import-preset-style-libraries.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.import-preset-style-libraries
  feature_name: Import preset style libraries
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: optional_integration
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import preset style libraries to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Import preset style libraries" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Import preset style libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/import-preset-style-libraries.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.layer-style-effects-and-options-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.layer-style-effects-and-options-overview.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.layer-style-effects-and-options-overview
  feature_name: Layer style effects and options overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Layer style effects and options overview to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Layer style effects and options overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Layer style effects and options overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/layer-style-effects-and-options-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.manage-contours.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.manage-contours.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.manage-contours
  feature_name: Manage contours
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage contours to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Manage contours" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Manage contours
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/manage-contours.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.manage-preset-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.manage-preset-styles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.manage-preset-styles
  feature_name: Manage preset styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage preset styles to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Manage preset styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Manage preset styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/manage-preset-styles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.remove-layer-effects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.remove-layer-effects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.remove-layer-effects
  feature_name: Remove layer effects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove layer effects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Remove layer effects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Remove layer effects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/remove-layer-effects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.scale-layer-effects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.scale-layer-effects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.scale-layer-effects
  feature_name: Scale layer effects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale layer effects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Scale layer effects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Scale layer effects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/scale-layer-effects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.set-a-global-lighting-angle-for-all-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.set-a-global-lighting-angle-for-all-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.set-a-global-lighting-angle-for-all-layers
  feature_name: Set a global lighting angle for all layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set a global lighting angle for all layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Set a global lighting angle for all layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Set a global lighting angle for all layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/set-a-global-lighting-angle-for-all-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.work-with-layer-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.work-with-layer-styles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.work-with-layer-styles
  feature_name: Work with layer styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with layer styles to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Work with layer styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Work with layer styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/work-with-layer-styles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.apply-layer-effects.work-with-preset-styles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.apply-layer-effects.work-with-preset-styles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.apply-layer-effects.work-with-preset-styles
  feature_name: Work with preset styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: apply-layer-effects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with preset styles to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Work with preset styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Work with preset styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/apply-layer-effects/work-with-preset-styles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjust-contrast-with-clarity-and-dehaze.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjust-contrast-with-clarity-and-dehaze.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjust-contrast-with-clarity-and-dehaze
  feature_name: Adjust contrast with Clarity and Dehaze
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust contrast with Clarity and Dehaze to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust contrast with Clarity and Dehaze" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Adjust contrast with Clarity and Dehaze
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/adjust-contrast-with-clarity-and-dehaze.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-and-fill-layers-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-and-fill-layers-overview.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-and-fill-layers-overview
  feature_name: Adjustment and fill layers overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjustment and fill layers overview to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Adjustment and fill layers overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Adjustment and fill layers overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/adjustment-and-fill-layers-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-layers-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-layers-options.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-layers-options
  feature_name: Adjustment layer options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjustment layer options to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Adjustment layer options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Adjustment layer options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/adjustment-layers-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-presets-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-presets-overview.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.adjustment-presets-overview
  feature_name: Adjustment presets overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjustment presets overview to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Adjustment presets overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Adjustment presets overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/adjustment-presets-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.change-adjustment-and-fill-layer-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.change-adjustment-and-fill-layer-options.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.change-adjustment-and-fill-layer-options
  feature_name: Change adjustment and fill layer options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change adjustment and fill layer options to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Change adjustment and fill layer options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Change adjustment and fill layer options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/change-adjustment-and-fill-layer-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.correct-color-balance-with-color-and-vibrance.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.correct-color-balance-with-color-and-vibrance.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.correct-color-balance-with-color-and-vibrance
  feature_name: Correct color balance with Color and vibrance
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Correct color balance with Color and vibrance to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Correct color balance with Color and vibrance" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Correct color balance with Color and vibrance
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/correct-color-balance-with-color-and-vibrance.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-adjustment-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-adjustment-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-adjustment-layers
  feature_name: Create adjustment layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create adjustment layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create adjustment layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create adjustment layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/create-adjustment-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-custom-presets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-custom-presets.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-custom-presets
  feature_name: Create custom presets
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create custom presets to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create custom presets" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create custom presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/create-custom-presets.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-fill-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-fill-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.create-fill-layers
  feature_name: Create fill layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create fill layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create fill layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create fill layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/create-fill-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.enhance-texture-with-grain.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.enhance-texture-with-grain.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.enhance-texture-with-grain
  feature_name: Adjust texture with Grain
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust texture with Grain to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust texture with Grain" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Adjust texture with Grain
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/enhance-texture-with-grain.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.merging-adjustment-or-fill-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.merging-adjustment-or-fill-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.merging-adjustment-or-fill-layers
  feature_name: Merging adjustment or fill layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Merging adjustment or fill layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Merging adjustment or fill layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Merging adjustment or fill layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/merging-adjustment-or-fill-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.use-layer-masks-to-target-adjustment-or-fill-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.use-layer-masks-to-target-adjustment-or-fill-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.use-layer-masks-to-target-adjustment-or-fill-layers
  feature_name: Use layer masks to target adjustment or fill layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use layer masks to target adjustment or fill layers to create, arrange, combine, or non-destructively control visual layer state imported from
    Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Use layer masks to target adjustment or fill layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Use layer masks to target adjustment or fill layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/use-layer-masks-to-target-adjustment-or-fill-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.work-with-adjustment-and-fill-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.work-with-adjustment-and-fill-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.color-adjustment-fill-layers.work-with-adjustment-and-fill-layers
  feature_name: Work with adjustment layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: color-adjustment-fill-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with adjustment layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Work with adjustment layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Work with adjustment layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/color-adjustment-fill-layers/work-with-adjustment-and-fill-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.create-layer-compositions.align-content-of-layers-and-groups.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.create-layer-compositions.align-content-of-layers-and-groups.v0
  source_feature_id: photoshop.leaf.create-manage-layers.create-layer-compositions.align-content-of-layers-and-groups
  feature_name: Align layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: create-layer-compositions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Align layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Align layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/create-layer-compositions/align-content-of-layers-and-groups.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.create-layer-compositions.align-image-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.create-layer-compositions.align-image-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.create-layer-compositions.align-image-layers
  feature_name: Auto-align image layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: create-layer-compositions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Auto-align image layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Auto-align image layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Auto-align image layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/create-layer-compositions/align-image-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.create-layer-compositions.distribute-layers-groups-evenly.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.create-layer-compositions.distribute-layers-groups-evenly.v0
  source_feature_id: photoshop.leaf.create-manage-layers.create-layer-compositions.distribute-layers-groups-evenly
  feature_name: Distribute layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: create-layer-compositions
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Distribute layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Distribute layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Distribute layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/create-layer-compositions/distribute-layers-groups-evenly.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.change-transparency-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.change-transparency-preferences.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.change-transparency-preferences
  feature_name: Change transparency preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change transparency preferences to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Change transparency preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Change transparency preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/change-transparency-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.convert-background-and-regular-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.convert-background-and-regular-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.convert-background-and-regular-layers
  feature_name: Convert background and regular layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert background and regular layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Convert background and regular layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Convert background and regular layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/convert-background-and-regular-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.create-document-from-layer-or-group.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.create-document-from-layer-or-group.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.create-document-from-layer-or-group
  feature_name: Create document from layer or group
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create document from layer or group to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create document from layer or group" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create document from layer or group
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/create-document-from-layer-or-group.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.duplicate-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.duplicate-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.duplicate-layers
  feature_name: Duplicate layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Duplicate layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Duplicate layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Duplicate layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/duplicate-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.layers-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.layers-overview.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.layers-overview
  feature_name: Layers overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Layers overview to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Layers overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Layers overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/layers-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.organize-layers-with-layer-groups.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.organize-layers-with-layer-groups.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.organize-layers-with-layer-groups
  feature_name: Organize layers with layer groups
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Organize layers with layer groups to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Organize layers with layer groups" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Organize layers with layer groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/organize-layers-with-layer-groups.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.sample-from-all-visible-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.sample-from-all-visible-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.sample-from-all-visible-layers
  feature_name: Sample from all visible layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Sample from all visible layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Sample from all visible layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Sample from all visible layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/sample-from-all-visible-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.video-layers-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.video-layers-overview.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.video-layers-overview
  feature_name: Video Layers overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Video Layers overview to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Video Layers overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Video Layers overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/video-layers-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.get-started-layers.work-with-the-layers-panel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.get-started-layers.work-with-the-layers-panel.v0
  source_feature_id: photoshop.leaf.create-manage-layers.get-started-layers.work-with-the-layers-panel
  feature_name: Work with the Layers panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: get-started-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with the Layers panel to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Work with the Layers panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Work with the Layers panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/get-started-layers/work-with-the-layers-panel.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.add-artboards-current-document.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.add-artboards-current-document.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.add-artboards-current-document
  feature_name: Add artboards to the current document
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add artboards to the current document to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Add artboards to the current document" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Add artboards to the current document
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/add-artboards-current-document.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.add-stroke-to-frame.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.add-stroke-to-frame.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.add-stroke-to-frame
  feature_name: Add a stroke to a frame
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add a stroke to a frame to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Add a stroke to a frame" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Add a stroke to a frame
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/add-stroke-to-frame.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.artboard-properties.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.artboard-properties.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.artboard-properties
  feature_name: Artboard properties and behaviors
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Artboard properties and behaviors to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Artboard properties and behaviors" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Artboard properties and behaviors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/artboard-properties.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.convert-to-frame.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.convert-to-frame.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.convert-to-frame
  feature_name: Convert shapes or text to frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert shapes or text to frames to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Convert shapes or text to frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Convert shapes or text to frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/convert-to-frame.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.create-artboard-documents.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.create-artboard-documents.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.create-artboard-documents
  feature_name: Create artboard documents
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create artboard documents to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create artboard documents" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create artboard documents
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/create-artboard-documents.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.draw-frames.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.draw-frames.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.draw-frames
  feature_name: Draw frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw frames to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Draw frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Draw frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/draw-frames.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.get-started-artboards.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.get-started-artboards.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.get-started-artboards
  feature_name: Get started with artboards
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get started with artboards to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Get started with artboards" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Get started with artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/get-started-artboards.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.place-image-frame.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.place-image-frame.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.place-image-frame
  feature_name: Place an image into a frame
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Place an image into a frame to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Place an image into a frame" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Place an image into a frame
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/place-image-frame.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.layout-design-tools.select-frame-content.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.layout-design-tools.select-frame-content.v0
  source_feature_id: photoshop.leaf.create-manage-layers.layout-design-tools.select-frame-content
  feature_name: Select a frame and its content
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: layout-design-tools
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select a frame and its content to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Select a frame and its content" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Select a frame and its content
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/layout-design-tools/select-frame-content.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.convert-embedded-smart-objects-to-linked.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.convert-embedded-smart-objects-to-linked.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.convert-embedded-smart-objects-to-linked
  feature_name: Convert embedded Smart Objects to linked
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert embedded Smart Objects to linked to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Convert embedded Smart Objects to linked" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Convert embedded Smart Objects to linked
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/convert-embedded-smart-objects-to-linked.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.convert-smart-objects-to-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.convert-smart-objects-to-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.convert-smart-objects-to-layers
  feature_name: Convert Smart Objects to layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert Smart Objects to layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Convert Smart Objects to layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Convert Smart Objects to layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/convert-smart-objects-to-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.create-embedded-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.create-embedded-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.create-embedded-smart-objects
  feature_name: Create embedded Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create embedded Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create embedded Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create embedded Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/create-embedded-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.create-linked-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.create-linked-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.create-linked-smart-objects
  feature_name: Create linked Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create linked Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create linked Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create linked Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/create-linked-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.duplicate-an-embedded-smart-object.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.duplicate-an-embedded-smart-object.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.duplicate-an-embedded-smart-object
  feature_name: Duplicate an embedded Smart Object
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Duplicate an embedded Smart Object to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Duplicate an embedded Smart Object" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Duplicate an embedded Smart Object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/duplicate-an-embedded-smart-object.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.edit-the-contents-of-a-smart-object.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.edit-the-contents-of-a-smart-object.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.edit-the-contents-of-a-smart-object
  feature_name: Edit the contents of a Smart Object
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit the contents of a Smart Object to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Edit the contents of a Smart Object" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Edit the contents of a Smart Object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/edit-the-contents-of-a-smart-object.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.embed-linked-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.embed-linked-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.embed-linked-smart-objects
  feature_name: Embed Linked Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Embed Linked Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Embed Linked Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Embed Linked Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/embed-linked-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.export-the-contents-of-an-embedded-smart-object.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.export-the-contents-of-an-embedded-smart-object.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.export-the-contents-of-an-embedded-smart-object
  feature_name: Export the contents of an embedded Smart Object
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export the contents of an embedded Smart Object to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Export the contents of an embedded Smart Object" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Export the contents of an embedded Smart Object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/export-the-contents-of-an-embedded-smart-object.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.filter-the-layers-panel-by-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.filter-the-layers-panel-by-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.filter-the-layers-panel-by-smart-objects
  feature_name: Filter the Layers panel by Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Filter the Layers panel by Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Filter the Layers panel by Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Filter the Layers panel by Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/filter-the-layers-panel-by-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.package-linked-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.package-linked-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.package-linked-smart-objects
  feature_name: Package and locate linked Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Package and locate linked Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Package and locate linked Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Package and locate linked Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/package-linked-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.rasterize-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.rasterize-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.rasterize-smart-objects
  feature_name: Rasterize Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rasterize Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Rasterize Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Rasterize Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/rasterize-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.replace-the-contents-of-a-smart-object.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.replace-the-contents-of-a-smart-object.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.replace-the-contents-of-a-smart-object
  feature_name: Replace the contents of a Smart Object
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace the contents of a Smart Object to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Replace the contents of a Smart Object" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Replace the contents of a Smart Object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/replace-the-contents-of-a-smart-object.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.reset-smart-object-transforms.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.reset-smart-object-transforms.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.reset-smart-object-transforms
  feature_name: Reset Smart Object transforms
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reset Smart Object transforms to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Reset Smart Object transforms" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Reset Smart Object transforms
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/reset-smart-object-transforms.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.smart-objects-overview-and-benefits.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.smart-objects-overview-and-benefits.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.smart-objects-overview-and-benefits
  feature_name: Smart Objects - overview and benefits
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Smart Objects - overview and benefits to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Smart Objects - overview and benefits" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Smart Objects - overview and benefits
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/smart-objects-overview-and-benefits.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.update-linked-smart-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.update-linked-smart-objects.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.update-linked-smart-objects
  feature_name: Update Linked Smart Objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Update Linked Smart Objects to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Update Linked Smart Objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Update Linked Smart Objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/update-linked-smart-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.smart-objects.view-linked-smart-object-properties.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.smart-objects.view-linked-smart-object-properties.v0
  source_feature_id: photoshop.leaf.create-manage-layers.smart-objects.view-linked-smart-object-properties
  feature_name: View Linked Smart Object properties
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: smart-objects
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View Linked Smart Object properties to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "View Linked Smart Object properties" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / View Linked Smart Object properties
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/smart-objects/view-linked-smart-object-properties.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.transform-manipulate-layers.clean-up-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.transform-manipulate-layers.clean-up-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.transform-manipulate-layers.clean-up-layers
  feature_name: Clean up image layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: transform-manipulate-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Clean up image layers to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Clean up image layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Clean up image layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/transform-manipulate-layers/clean-up-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.transform-manipulate-layers.display-layer-edges-and-handles.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.transform-manipulate-layers.display-layer-edges-and-handles.v0
  source_feature_id: photoshop.leaf.create-manage-layers.transform-manipulate-layers.display-layer-edges-and-handles
  feature_name: Display layer edges and handles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: transform-manipulate-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Display layer edges and handles to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Display layer edges and handles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Display layer edges and handles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/transform-manipulate-layers/display-layer-edges-and-handles.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.transform-manipulate-layers.group-and-ungroup-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.transform-manipulate-layers.group-and-ungroup-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.transform-manipulate-layers.group-and-ungroup-layers
  feature_name: Group and ungroup layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: transform-manipulate-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Group and ungroup layers to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Group and ungroup layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Group and ungroup layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/transform-manipulate-layers/group-and-ungroup-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.transform-manipulate-layers.link-and-unlink-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.transform-manipulate-layers.link-and-unlink-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.transform-manipulate-layers.link-and-unlink-layers
  feature_name: Link and unlink layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: transform-manipulate-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Link and unlink layers to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Link and unlink layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Link and unlink layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/transform-manipulate-layers/link-and-unlink-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-manage-layers.transform-manipulate-layers.select-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-manage-layers.transform-manipulate-layers.select-layers.v0
  source_feature_id: photoshop.leaf.create-manage-layers.transform-manipulate-layers.select-layers
  feature_name: Select layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-manage-layers
  source_subcategory: transform-manipulate-layers
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select layers to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Select layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Select layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-manage-layers/transform-manipulate-layers/select-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.blend-images.auto-blend-layers-command-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.blend-images.auto-blend-layers-command-overview.v0
  source_feature_id: photoshop.leaf.create-masks.blend-images.auto-blend-layers-command-overview
  feature_name: Auto-Blend Layers command overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: blend-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Auto-Blend Layers command overview to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Auto-Blend Layers command overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Auto-Blend Layers command overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/blend-images/auto-blend-layers-command-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.blend-images.create-a-composite-with-extended-depth-of-field.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.blend-images.create-a-composite-with-extended-depth-of-field.v0
  source_feature_id: photoshop.leaf.create-masks.blend-images.create-a-composite-with-extended-depth-of-field
  feature_name: Create a composite with extended depth of field
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: blend-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a composite with extended depth of field to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Create a composite with extended depth of field" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Create a composite with extended depth of field
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/blend-images/create-a-composite-with-extended-depth-of-field.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.layer-masks.add-layer-masks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.layer-masks.add-layer-masks.v0
  source_feature_id: photoshop.leaf.create-masks.layer-masks.add-layer-masks
  feature_name: Add layer masks
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: layer-masks
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add layer masks to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Add layer masks" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Add layer masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/add-layer-masks.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.layer-masks.apply-or-delete-layer-masks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.layer-masks.apply-or-delete-layer-masks.v0
  source_feature_id: photoshop.leaf.create-masks.layer-masks.apply-or-delete-layer-masks
  feature_name: Apply or delete layer masks
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: layer-masks
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply or delete layer masks to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Apply or delete layer masks" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Apply or delete layer masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/apply-or-delete-layer-masks.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.layer-masks.create-layer-masks-for-all-detected-objects-in-a-layer.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.layer-masks.create-layer-masks-for-all-detected-objects-in-a-layer.v0
  source_feature_id: photoshop.leaf.create-masks.layer-masks.create-layer-masks-for-all-detected-objects-in-a-layer
  feature_name: Create layer masks for all detected objects in a layer
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: layer-masks
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create layer masks for all detected objects in a layer to create, arrange, combine, or non-destructively control visual layer state imported from
    Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Create layer masks for all detected objects in a layer" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Create layer masks for all detected objects in a layer
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/create-layer-masks-for-all-detected-objects-in-a-layer.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.layer-masks.disable-or-enable-layer-masks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.layer-masks.disable-or-enable-layer-masks.v0
  source_feature_id: photoshop.leaf.create-masks.layer-masks.disable-or-enable-layer-masks
  feature_name: Disable or enable layer masks
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: layer-masks
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Disable or enable layer masks to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Disable or enable layer masks" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Disable or enable layer masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/disable-or-enable-layer-masks.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-masks.layer-masks.unlink-layers-and-masks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-masks.layer-masks.unlink-layers-and-masks.v0
  source_feature_id: photoshop.leaf.create-masks.layer-masks.unlink-layers-and-masks
  feature_name: Unlink layers and masks
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-masks
  source_subcategory: layer-masks
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Unlink layers and masks to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Unlink layers and masks" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Unlink layers and masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-masks/layer-masks/unlink-layers-and-masks.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.edit-images-with-generative-fill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.create-images.edit-images-with-generative-fill.v0
  source_feature_id: photoshop.leaf.create-open-import-images.create-images.edit-images-with-generative-fill
  feature_name: Edit images with Generative Fill
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: create-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit images with Generative Fill to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Edit images with Generative Fill" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Edit images with Generative Fill
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/edit-images-with-generative-fill.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.explore-beyond-the-canvas-with-generative-expand.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.create-images.explore-beyond-the-canvas-with-generative-expand.v0
  source_feature_id: photoshop.leaf.create-open-import-images.create-images.explore-beyond-the-canvas-with-generative-expand
  feature_name: Explore beyond the canvas with Generative Expand
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: create-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Explore beyond the canvas with Generative Expand to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Explore beyond the canvas with Generative Expand" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Explore beyond the canvas with Generative Expand
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/explore-beyond-the-canvas-with-generative-expand.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.generate-image-with-descriptive-text-prompts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.create-images.generate-image-with-descriptive-text-prompts.v0
  source_feature_id: photoshop.leaf.create-open-import-images.create-images.generate-image-with-descriptive-text-prompts
  feature_name: Generate an image with descriptive text prompts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: create-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate an image with descriptive text prompts to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Generate an image with descriptive text prompts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Generate an image with descriptive text prompts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/generate-image-with-descriptive-text-prompts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.generate-images-using-reference-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.create-images.generate-images-using-reference-image.v0
  source_feature_id: photoshop.leaf.create-open-import-images.create-images.generate-images-using-reference-image
  feature_name: Generate images guided by a reference image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: create-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate images guided by a reference image to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Generate images guided by a reference image" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate images guided by a reference image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/generate-images-using-reference-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.generate-sharper-variations-with-enhance-detail.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.create-images.generate-sharper-variations-with-enhance-detail.v0
  source_feature_id: photoshop.leaf.create-open-import-images.create-images.generate-sharper-variations-with-enhance-detail
  feature_name: Generate sharper variations with Enhance Detail
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: create-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate sharper variations with Enhance Detail to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Generate sharper variations with Enhance Detail" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Generate sharper variations with Enhance Detail
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/generate-sharper-variations-with-enhance-detail.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.create-images.use-reference-images-for-consistent-results.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.create-images.use-reference-images-for-consistent-results.v0
  source_feature_id: photoshop.leaf.create-open-import-images.create-images.use-reference-images-for-consistent-results
  feature_name: Use reference images for consistent results
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: create-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: local_primitive_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use reference images for consistent results to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Use reference images for consistent results" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Use reference images for consistent results
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/create-images/use-reference-images-for-consistent-results.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.create-open-import-images.import-files.browse-select-and-import-adobe-stock-assets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.create-open-import-images.import-files.browse-select-and-import-adobe-stock-assets.v0
  source_feature_id: photoshop.leaf.create-open-import-images.import-files.browse-select-and-import-adobe-stock-assets
  feature_name: Browse, select, and import Adobe Stock assets
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: create-open-import-images
  source_subcategory: import-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: optional_integration
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Browse, select, and import Adobe Stock assets to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Browse, select, and import Adobe Stock assets" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Browse, select, and import Adobe Stock assets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/create-open-import-images/import-files/browse-select-and-import-adobe-stock-assets.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.crop-straighten.apply-content-aware-fill-while-cropping-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.crop-straighten.apply-content-aware-fill-while-cropping-images.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.crop-straighten.apply-content-aware-fill-while-cropping-images
  feature_name: Apply Content-Aware Fill while cropping images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: crop-straighten
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply Content-Aware Fill while cropping images to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Apply Content-Aware Fill while cropping images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Apply Content-Aware Fill while cropping images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/crop-straighten/apply-content-aware-fill-while-cropping-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.crop-straighten.crop-photos.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.crop-straighten.crop-photos.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.crop-straighten.crop-photos
  feature_name: Crop photos
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: crop-straighten
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Crop photos to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Crop photos" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Crop photos
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/crop-straighten/crop-photos.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.crop-straighten.crop-tool-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.crop-straighten.crop-tool-options.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.crop-straighten.crop-tool-options
  feature_name: Crop tool options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: crop-straighten
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Crop tool options to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Crop tool options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Crop tool options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/crop-straighten/crop-tool-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.crop-straighten.resize-canvas-using-the-crop-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.crop-straighten.resize-canvas-using-the-crop-tool.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.crop-straighten.resize-canvas-using-the-crop-tool
  feature_name: Resize the canvas using the crop tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: crop-straighten
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resize the canvas using the crop tool to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Resize the canvas using the crop tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Resize the canvas using the crop tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/crop-straighten/resize-canvas-using-the-crop-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.crop-straighten.straighten-tilted-photos.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.crop-straighten.straighten-tilted-photos.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.crop-straighten.straighten-tilted-photos
  feature_name: Straighten tilted photos
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: crop-straighten
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Straighten tilted photos to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Straighten tilted photos" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Straighten tilted photos
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/crop-straighten/straighten-tilted-photos.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.crop-straighten.transform-perspective-while-cropping.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.crop-straighten.transform-perspective-while-cropping.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.crop-straighten.transform-perspective-while-cropping
  feature_name: Transform perspective while cropping
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: crop-straighten
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transform perspective while cropping to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Transform perspective while cropping" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Transform perspective while cropping
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/crop-straighten/transform-perspective-while-cropping.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-pixel-dimensions-of-an-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-pixel-dimensions-of-an-image.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-pixel-dimensions-of-an-image
  feature_name: Change the pixel dimensions of images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change the pixel dimensions of images to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Change the pixel dimensions of images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Change the pixel dimensions of images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/change-pixel-dimensions-of-an-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-print-dimensions-and-resolution.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-print-dimensions-and-resolution.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.change-print-dimensions-and-resolution
  feature_name: Change print dimensions and resolution
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change print dimensions and resolution to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Change print dimensions and resolution" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Change print dimensions and resolution
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/change-print-dimensions-and-resolution.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.file-size.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.file-size.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.file-size
  feature_name: File size
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use File size to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "File size" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / File size
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/file-size.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.manage-image-file-size.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.manage-image-file-size.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.manage-image-file-size
  feature_name: Manage image file size
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage image file size to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Manage image file size" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Manage image file size
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/manage-image-file-size.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.monitor-resolution-and-image-display-size.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.monitor-resolution-and-image-display-size.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.monitor-resolution-and-image-display-size
  feature_name: Monitor resolution and image display size
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Monitor resolution and image display size to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Monitor resolution and image display size" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Monitor resolution and image display size
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/monitor-resolution-and-image-display-size.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.preserve-visual-content-when-scaling-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.preserve-visual-content-when-scaling-images.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.preserve-visual-content-when-scaling-images
  feature_name: Preserve visual content when scaling images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Preserve visual content when scaling images to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Preserve visual content when scaling images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Preserve visual content when scaling images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/preserve-visual-content-when-scaling-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.printed-image-resolution.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.printed-image-resolution.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.printed-image-resolution
  feature_name: Printed image resolution
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Printed image resolution to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Printed image resolution" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Printed image resolution
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/printed-image-resolution.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.printer-resolution.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.printer-resolution.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.printer-resolution
  feature_name: Printer resolution
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Printer resolution to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Printer resolution" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Printer resolution
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/printer-resolution.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resample-option-in-image-size-dialog-box.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resample-option-in-image-size-dialog-box.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resample-option-in-image-size-dialog-box
  feature_name: Resample option in the Image Size dialog
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resample option in the Image Size dialog to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Resample option in the Image Size dialog" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Resample option in the Image Size dialog
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/resample-option-in-image-size-dialog-box.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resampling-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resampling-options.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resampling-options
  feature_name: Resampling options in Photoshop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resampling options in Photoshop to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Resampling options in Photoshop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Resampling options in Photoshop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/resampling-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resize-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resize-images.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resize-images
  feature_name: Resize images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resize images to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Resize images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Resize images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/resize-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resizing-parameters-in-photoshop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resizing-parameters-in-photoshop.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resizing-parameters-in-photoshop
  feature_name: Resizing parameters in Photoshop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resizing parameters in Photoshop to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Resizing parameters in Photoshop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Resizing parameters in Photoshop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/resizing-parameters-in-photoshop.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resolution-specs-for-printing-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resolution-specs-for-printing-images.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.resolution-specs-for-printing-images
  feature_name: Resolution specs for printing images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resolution specs for printing images to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Resolution specs for printing images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Resolution specs for printing images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/resolution-specs-for-printing-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.set-image-size-and-resolution.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.set-image-size-and-resolution.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.set-image-size-and-resolution
  feature_name: Set image size and resolution
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set image size and resolution to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Set image size and resolution" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Set image size and resolution
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/set-image-size-and-resolution.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.specify-content-to-protect-when-scaling.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.resize-adjust-resolution.specify-content-to-protect-when-scaling.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.resize-adjust-resolution.specify-content-to-protect-when-scaling
  feature_name: Specify content to protect when scaling
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: resize-adjust-resolution
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Specify content to protect when scaling to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Specify content to protect when scaling" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Specify content to protect when scaling
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/resize-adjust-resolution/specify-content-to-protect-when-scaling.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.adjust-scale-rotation-and-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.adjust-scale-rotation-and-perspective.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.adjust-scale-rotation-and-perspective
  feature_name: Adjust scale, rotation, and perspective
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust scale, rotation, and perspective to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust scale, rotation, and perspective" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Adjust scale, rotation, and perspective
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/adjust-scale-rotation-and-perspective.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.apply-transformations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.apply-transformations.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.apply-transformations
  feature_name: Apply transformations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply transformations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Apply transformations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Apply transformations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/apply-transformations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.duplicate-objects-as-you-transform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.duplicate-objects-as-you-transform.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.duplicate-objects-as-you-transform
  feature_name: Duplicate objects as you transform
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Duplicate objects as you transform to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Duplicate objects as you transform" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Duplicate objects as you transform
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/duplicate-objects-as-you-transform.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.move-reference-point-for-transformations.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.move-reference-point-for-transformations.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.move-reference-point-for-transformations
  feature_name: Move reference point for transformations
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move reference point for transformations to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Move reference point for transformations" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Move reference point for transformations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/move-reference-point-for-transformations.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.rotate-objects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.rotate-objects.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.rotate-objects
  feature_name: Rotate objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate objects to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Rotate objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Rotate objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/rotate-objects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.rotate-or-flip-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.rotate-or-flip-images.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.rotate-or-flip-images
  feature_name: Rotate or flip images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate or flip images to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Rotate or flip images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Rotate or flip images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/rotate-or-flip-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.transformation-options-in-adobe-photoshop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.transformation-options-in-adobe-photoshop.v0
  source_feature_id: photoshop.leaf.crop-resize-transform.transform-manipulate-reshape.transformation-options-in-adobe-photoshop
  feature_name: Transformation options in Photoshop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: crop-resize-transform
  source_subcategory: transform-manipulate-reshape
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transformation options in Photoshop to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Transformation options in Photoshop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Transformation options in Photoshop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/crop-resize-transform/transform-manipulate-reshape/transformation-options-in-adobe-photoshop.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.create-shapes.add-legacy-custom-shapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.create-shapes.add-legacy-custom-shapes.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.create-shapes.add-legacy-custom-shapes
  feature_name: Add legacy custom shapes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: create-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add legacy custom shapes to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Add legacy custom shapes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add legacy custom shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/create-shapes/add-legacy-custom-shapes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.create-shapes.create-shapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.create-shapes.create-shapes.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.create-shapes.create-shapes
  feature_name: Draw shapes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: create-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw shapes to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw shapes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/create-shapes/create-shapes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.create-shapes.draw-custom-shapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.create-shapes.draw-custom-shapes.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.create-shapes.draw-custom-shapes
  feature_name: Draw custom shapes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: create-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw custom shapes to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw custom shapes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw custom shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/create-shapes/draw-custom-shapes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.create-shapes.draw-star-shapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.create-shapes.draw-star-shapes.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.create-shapes.draw-star-shapes
  feature_name: Create star shapes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: create-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create star shapes to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Create star shapes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create star shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/create-shapes/draw-star-shapes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.create-shapes.drawing-tools-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.create-shapes.drawing-tools-overview.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.create-shapes.drawing-tools-overview
  feature_name: Drawing tools overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: create-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Drawing tools overview to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Drawing tools overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Drawing tools overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/create-shapes/drawing-tools-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.create-shapes.fill-and-stroke-shapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.create-shapes.fill-and-stroke-shapes.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.create-shapes.fill-and-stroke-shapes
  feature_name: Modify fill and stroke for shapes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: create-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify fill and stroke for shapes to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Modify fill and stroke for shapes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Modify fill and stroke for shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/create-shapes/fill-and-stroke-shapes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-a-circle-square-or-rectangle.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-a-circle-square-or-rectangle.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-a-circle-square-or-rectangle
  feature_name: Draw a circle, square, or rectangle
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw a circle, square, or rectangle to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw a circle, square, or rectangle" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw a circle, square, or rectangle
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/draw-a-circle-square-or-rectangle.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-an-arrow.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-an-arrow.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-an-arrow
  feature_name: Draw an arrow
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw an arrow to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw an arrow" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw an arrow
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/draw-an-arrow.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-curves-and-straight-segments-intuitively.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-curves-and-straight-segments-intuitively.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-curves-and-straight-segments-intuitively
  feature_name: Draw curves and straight segments
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw curves and straight segments to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw curves and straight segments" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw curves and straight segments
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/draw-curves-and-straight-segments-intuitively.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-lines-and-straight-line-segments.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-lines-and-straight-line-segments.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-lines-and-straight-line-segments
  feature_name: Draw lines and line segments
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw lines and line segments to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw lines and line segments" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw lines and line segments
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/draw-lines-and-straight-line-segments.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-paths-with-the-pen-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-paths-with-the-pen-tool.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.draw-paths-with-the-pen-tool
  feature_name: Draw paths with the Pen tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw paths with the Pen tool to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Draw paths with the Pen tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw paths with the Pen tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/draw-paths-with-the-pen-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.overview-of-pen-tool-settings.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.overview-of-pen-tool-settings.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.overview-of-pen-tool-settings
  feature_name: Overview of Pen tool settings
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of Pen tool settings to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of Pen tool settings" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Overview of Pen tool settings
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/overview-of-pen-tool-settings.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.shape-path-and-pixel-mode-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.shape-path-and-pixel-mode-options.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.shape-path-and-pixel-mode-options
  feature_name: Shape, path, and pixel mode options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Shape, path, and pixel mode options to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Shape, path, and pixel mode options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Shape, path, and pixel mode options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/shape-path-and-pixel-mode-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.draw-shapes-paths.draw-lines-curves.trace-images-easily-with-the-content-aware-tracing-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.draw-shapes-paths.draw-lines-curves.trace-images-easily-with-the-content-aware-tracing-tool.v0
  source_feature_id: photoshop.leaf.draw-shapes-paths.draw-lines-curves.trace-images-easily-with-the-content-aware-tracing-tool
  feature_name: Trace images with the Content-Aware Tracing tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: draw-shapes-paths
  source_subcategory: draw-lines-curves
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Trace images with the Content-Aware Tracing tool to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Trace images with the Content-Aware Tracing tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Trace images with the Content-Aware Tracing tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/draw-shapes-paths/draw-lines-curves/trace-images-easily-with-the-content-aware-tracing-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.distort-specific-image-areas-with-puppet-warp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.distort-specific-image-areas-with-puppet-warp.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.distort-specific-image-areas-with-puppet-warp
  feature_name: Distort specific image areas with Puppet Warp
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Distort specific image areas with Puppet Warp to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Distort specific image areas with Puppet Warp" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Distort specific image areas with Puppet Warp
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/distort-specific-image-areas-with-puppet-warp.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.enhance-images-with-generative-ai-filters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.enhance-images-with-generative-ai-filters.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.enhance-images-with-generative-ai-filters
  feature_name: Enhance images with generative AI filters
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Enhance images with generative AI filters to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Enhance images with generative AI filters" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Enhance images with generative AI filters
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/enhance-images-with-generative-ai-filters.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.freeze-or-thaw-areas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.freeze-or-thaw-areas.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.freeze-or-thaw-areas
  feature_name: Freeze or thaw areas
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Freeze or thaw areas to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Freeze or thaw areas" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Freeze or thaw areas
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/freeze-or-thaw-areas.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.get-precise-distortions-with-split-warp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.get-precise-distortions-with-split-warp.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.get-precise-distortions-with-split-warp
  feature_name: Get precise distortions with Split Warp
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get precise distortions with Split Warp to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Get precise distortions with Split Warp" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Get precise distortions with Split Warp
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/get-precise-distortions-with-split-warp.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.overview-of-distortion-tools.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.overview-of-distortion-tools.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.overview-of-distortion-tools
  feature_name: Overview of distortion tools
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of distortion tools to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of distortion tools" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Overview of distortion tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/overview-of-distortion-tools.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.overview-of-liquify-filter.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.overview-of-liquify-filter.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.overview-of-liquify-filter
  feature_name: Overview of Liquify filter
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of Liquify filter to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of Liquify filter" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Overview of Liquify filter
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/overview-of-liquify-filter.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.reconstruct-distortions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.reconstruct-distortions.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.reconstruct-distortions
  feature_name: Reconstruct distortions
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reconstruct distortions to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Reconstruct distortions" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Reconstruct distortions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/reconstruct-distortions.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.replace-the-sky-in-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.replace-the-sky-in-images.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.replace-the-sky-in-images
  feature_name: Replace the sky in images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace the sky in images to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Replace the sky in images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Replace the sky in images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/replace-the-sky-in-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.reshape-and-distort-images-with-transform-warp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.reshape-and-distort-images-with-transform-warp.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.reshape-and-distort-images-with-transform-warp
  feature_name: Reshape and distort images with Transform Warp
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reshape and distort images with Transform Warp to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Reshape and distort images with Transform Warp" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Reshape and distort images with Transform Warp
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/reshape-and-distort-images-with-transform-warp.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.select-and-manage-sky-presets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.select-and-manage-sky-presets.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.select-and-manage-sky-presets
  feature_name: Select and manage sky presets
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select and manage sky presets to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Select and manage sky presets" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select and manage sky presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/select-and-manage-sky-presets.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.use-liquify-to-distort-an-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.use-liquify-to-distort-an-image.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.use-liquify-to-distort-an-image
  feature_name: Use Liquify to distort an image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Liquify to distort an image to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Use Liquify to distort an image" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Use Liquify to distort an image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/use-liquify-to-distort-an-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.warp-a-layer-wth-cylindrical-transform.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.warp-a-layer-wth-cylindrical-transform.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.warp-a-layer-wth-cylindrical-transform
  feature_name: Warp a Layer with Cylindrical Transform
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Warp a Layer with Cylindrical Transform to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Warp a Layer with Cylindrical Transform" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Warp a Layer with Cylindrical Transform
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/warp-a-layer-wth-cylindrical-transform.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.work-with-backdrops.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.work-with-backdrops.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.work-with-backdrops
  feature_name: Work with backdrops
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with backdrops to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Work with backdrops" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Work with backdrops
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/work-with-backdrops.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.artistic-stylize-filters.work-with-meshes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.artistic-stylize-filters.work-with-meshes.v0
  source_feature_id: photoshop.leaf.effects-filters.artistic-stylize-filters.work-with-meshes
  feature_name: Work with meshes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: artistic-stylize-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with meshes to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Work with meshes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Work with meshes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/artistic-stylize-filters/work-with-meshes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.blur-sharpen-filters.blur-specific-areas-with-the-blur-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.blur-sharpen-filters.blur-specific-areas-with-the-blur-tool.v0
  source_feature_id: photoshop.leaf.effects-filters.blur-sharpen-filters.blur-specific-areas-with-the-blur-tool
  feature_name: Soften hard edges or reduce detail with the Blur tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: blur-sharpen-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Soften hard edges or reduce detail with the Blur tool to modify pixel content or raster-derived appearance through a Studio command that can be
    previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Soften hard edges or reduce detail with the Blur tool" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Soften hard edges or reduce detail with the Blur tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/blur-sharpen-filters/blur-specific-areas-with-the-blur-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.blur-sharpen-filters.create-depth-of-field-with-lens-blur.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.blur-sharpen-filters.create-depth-of-field-with-lens-blur.v0
  source_feature_id: photoshop.leaf.effects-filters.blur-sharpen-filters.create-depth-of-field-with-lens-blur
  feature_name: Create depth of field with lens blur
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: blur-sharpen-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create depth of field with lens blur to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create depth of field with lens blur" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create depth of field with lens blur
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/blur-sharpen-filters/create-depth-of-field-with-lens-blur.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.blur-sharpen-filters.enhance-edge-contrast-with-the-sharpen-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.blur-sharpen-filters.enhance-edge-contrast-with-the-sharpen-tool.v0
  source_feature_id: photoshop.leaf.effects-filters.blur-sharpen-filters.enhance-edge-contrast-with-the-sharpen-tool
  feature_name: Enhance edge contrast with the Sharpen tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: blur-sharpen-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Enhance edge contrast with the Sharpen tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Enhance edge contrast with the Sharpen tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Enhance edge contrast with the Sharpen tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/blur-sharpen-filters/enhance-edge-contrast-with-the-sharpen-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.blur-sharpen-filters.overview-of-adding-blur-to-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.blur-sharpen-filters.overview-of-adding-blur-to-images.v0
  source_feature_id: photoshop.leaf.effects-filters.blur-sharpen-filters.overview-of-adding-blur-to-images
  feature_name: Overview of adding blur to images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: blur-sharpen-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of adding blur to images to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of adding blur to images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Overview of adding blur to images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/blur-sharpen-filters/overview-of-adding-blur-to-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.get-started-with-filters.apply-filters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.get-started-with-filters.apply-filters.v0
  source_feature_id: photoshop.leaf.effects-filters.get-started-with-filters.apply-filters
  feature_name: Apply filters
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: get-started-with-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply filters to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Apply filters" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Apply filters
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/get-started-with-filters/apply-filters.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.get-started-with-filters.apply-filters-from-the-filter-gallery.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.get-started-with-filters.apply-filters-from-the-filter-gallery.v0
  source_feature_id: photoshop.leaf.effects-filters.get-started-with-filters.apply-filters-from-the-filter-gallery
  feature_name: Apply filters from the filter gallery
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: get-started-with-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply filters from the filter gallery to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Apply filters from the filter gallery" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Apply filters from the filter gallery
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/get-started-with-filters/apply-filters-from-the-filter-gallery.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.get-started-with-filters.blend-and-fade-filter-effects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.get-started-with-filters.blend-and-fade-filter-effects.v0
  source_feature_id: photoshop.leaf.effects-filters.get-started-with-filters.blend-and-fade-filter-effects
  feature_name: Blend and fade filter effects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: get-started-with-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blend and fade filter effects to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Blend and fade filter effects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Blend and fade filter effects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/get-started-with-filters/blend-and-fade-filter-effects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.get-started-with-filters.filter-gallery.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.get-started-with-filters.filter-gallery.v0
  source_feature_id: photoshop.leaf.effects-filters.get-started-with-filters.filter-gallery
  feature_name: Filter Gallery overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: get-started-with-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Filter Gallery overview to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Filter Gallery overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Filter Gallery overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/get-started-with-filters/filter-gallery.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.get-started-with-filters.filters-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.get-started-with-filters.filters-overview.v0
  source_feature_id: photoshop.leaf.effects-filters.get-started-with-filters.filters-overview
  feature_name: Filters overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: get-started-with-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Filters overview to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Filters overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Filters overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/get-started-with-filters/filters-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.get-started-with-filters.tips-for-creating-special-effects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.get-started-with-filters.tips-for-creating-special-effects.v0
  source_feature_id: photoshop.leaf.effects-filters.get-started-with-filters.tips-for-creating-special-effects
  feature_name: Tips for creating special effects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: get-started-with-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Tips for creating special effects to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Tips for creating special effects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Tips for creating special effects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/get-started-with-filters/tips-for-creating-special-effects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.neural-filters.neural-filter-categories-and-output-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.neural-filters.neural-filter-categories-and-output-options.v0
  source_feature_id: photoshop.leaf.effects-filters.neural-filters.neural-filter-categories-and-output-options
  feature_name: Neural Filter categories and output options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: neural-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Neural Filter categories and output options to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Neural Filter categories and output options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Neural Filter categories and output options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/neural-filters/neural-filter-categories-and-output-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.neural-filters.overview-of-neural-filters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.neural-filters.overview-of-neural-filters.v0
  source_feature_id: photoshop.leaf.effects-filters.neural-filters.overview-of-neural-filters
  feature_name: Overview of Neural Filters
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: neural-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of Neural Filters to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of Neural Filters" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Overview of Neural Filters
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/neural-filters/overview-of-neural-filters.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.neural-filters.use-neural-filters-to-enhance-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.neural-filters.use-neural-filters-to-enhance-images.v0
  source_feature_id: photoshop.leaf.effects-filters.neural-filters.use-neural-filters-to-enhance-images
  feature_name: Use Neural Filters to enhance images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: neural-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Neural Filters to enhance images to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Use Neural Filters to enhance images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Use Neural Filters to enhance images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/neural-filters/use-neural-filters-to-enhance-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.smart-filters.sharpen-a-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.smart-filters.sharpen-a-selection.v0
  source_feature_id: photoshop.leaf.effects-filters.smart-filters.sharpen-a-selection
  feature_name: Sharpen a selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: smart-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Sharpen a selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Sharpen a selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Sharpen a selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/smart-filters/sharpen-a-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.smart-filters.sharpen-controls-with-smart-sharpen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.smart-filters.sharpen-controls-with-smart-sharpen.v0
  source_feature_id: photoshop.leaf.effects-filters.smart-filters.sharpen-controls-with-smart-sharpen
  feature_name: Sharpen controls with smart sharpen
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: smart-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Sharpen controls with smart sharpen to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Sharpen controls with smart sharpen" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Sharpen controls with smart sharpen
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/smart-filters/sharpen-controls-with-smart-sharpen.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.smart-filters.sharpen-image-using-edge-mask.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.smart-filters.sharpen-image-using-edge-mask.v0
  source_feature_id: photoshop.leaf.effects-filters.smart-filters.sharpen-image-using-edge-mask
  feature_name: Sharpen image using edge mask
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: smart-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Sharpen image using edge mask to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Sharpen image using edge mask" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Sharpen image using edge mask
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/smart-filters/sharpen-image-using-edge-mask.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.smart-filters.sharpen-images-with-unsharp-mask.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.smart-filters.sharpen-images-with-unsharp-mask.v0
  source_feature_id: photoshop.leaf.effects-filters.smart-filters.sharpen-images-with-unsharp-mask
  feature_name: Sharpen images with the unsharp mask
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: smart-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Sharpen images with the unsharp mask to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Sharpen images with the unsharp mask" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Sharpen images with the unsharp mask
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/smart-filters/sharpen-images-with-unsharp-mask.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.effects-filters.smart-filters.sharpening-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.effects-filters.smart-filters.sharpening-overview.v0
  source_feature_id: photoshop.leaf.effects-filters.smart-filters.sharpening-overview
  feature_name: Sharpening overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: effects-filters
  source_subcategory: smart-filters
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Sharpening overview to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Sharpening overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Sharpening overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/effects-filters/smart-filters/sharpening-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.generative-ai.frequently-asked-questions-about-generative-ai-features.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.generative-ai.frequently-asked-questions-about-generative-ai-features.v0
  source_feature_id: photoshop.leaf.generative-ai.frequently-asked-questions-about-generative-ai-features
  feature_name: Generative AI features in Photoshop on Desktop FAQ
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: generative-ai
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generative AI features in Photoshop on Desktop FAQ to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Generative AI features in Photoshop on Desktop FAQ" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generative AI features in Photoshop on Desktop FAQ
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/generative-ai/frequently-asked-questions-about-generative-ai-features.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.generative-ai.generative-ai-features-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.generative-ai.generative-ai-features-overview.v0
  source_feature_id: photoshop.leaf.generative-ai.generative-ai-features-overview
  feature_name: Generative AI features overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: generative-ai
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generative AI features overview to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Generative AI features overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generative AI features overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/generative-ai/generative-ai-features-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.generative-ai.get-new-variations-of-generated-content-with-generate-similar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.generative-ai.get-new-variations-of-generated-content-with-generate-similar.v0
  source_feature_id: photoshop.leaf.generative-ai.get-new-variations-of-generated-content-with-generate-similar
  feature_name: Get new variations of generated content
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: generative-ai
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get new variations of generated content to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Get new variations of generated content" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Get new variations of generated content
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/generative-ai/get-new-variations-of-generated-content-with-generate-similar.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.generative-ai.open-firefly-boards.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.generative-ai.open-firefly-boards.v0
  source_feature_id: photoshop.leaf.generative-ai.open-firefly-boards
  feature_name: Use Firefly Boards with Photoshop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: generative-ai
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Firefly Boards with Photoshop to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Use Firefly Boards with Photoshop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Use Firefly Boards with Photoshop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/generative-ai/open-firefly-boards.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.generative-ai.select-an-ai-model-for-generative-control.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.generative-ai.select-an-ai-model-for-generative-control.v0
  source_feature_id: photoshop.leaf.generative-ai.select-an-ai-model-for-generative-control
  feature_name: Select an AI model for generative control
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: generative-ai
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select an AI model for generative control to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Select an AI model for generative control" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Select an AI model for generative control
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/generative-ai/select-an-ai-model-for-generative-control.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.access-discover-panel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.access-discover-panel.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.access-discover-panel
  feature_name: Access the Discover panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Access the Discover panel to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Access the Discover panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Access the Discover panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/access-discover-panel.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.add-remove-panels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.add-remove-panels.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.add-remove-panels
  feature_name: Add and remove panels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add and remove panels to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Add and remove panels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Add and remove panels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/add-remove-panels.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.boost-workflows-with-the-contextual-task-bar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.boost-workflows-with-the-contextual-task-bar.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.boost-workflows-with-the-contextual-task-bar
  feature_name: Boost workflows with the Contextual Task Bar
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Boost workflows with the Contextual Task Bar to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Boost workflows with the Contextual Task Bar" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Boost workflows with the Contextual Task Bar
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/boost-workflows-with-the-contextual-task-bar.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.change-text-size.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.change-text-size.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.change-text-size
  feature_name: Change text size in panels and tooltips
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change text size in panels and tooltips to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Change text size in panels and tooltips" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Change text size in panels and tooltips
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/change-text-size.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.collapse-expand-icons.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.collapse-expand-icons.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.collapse-expand-icons
  feature_name: Expand or collapse panel icons
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Expand or collapse panel icons to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Expand or collapse panel icons" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Expand or collapse panel icons
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/collapse-expand-icons.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.delete-workspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.delete-workspaces.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.delete-workspaces
  feature_name: Delete workspaces
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete workspaces to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Delete workspaces" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Delete workspaces
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/delete-workspaces.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.dock-undock-panels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.dock-undock-panels.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.dock-undock-panels
  feature_name: Dock or undock panels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Dock or undock panels to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Dock or undock panels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Dock or undock panels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/dock-undock-panels.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.hide-show-panels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.hide-show-panels.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.hide-show-panels
  feature_name: Hide or show all panels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Hide or show all panels to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Hide or show all panels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Hide or show all panels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/hide-show-panels.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.homescreen-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.homescreen-overview.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.homescreen-overview
  feature_name: Home screen overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Home screen overview to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Home screen overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Home screen overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/homescreen-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.manipulate-panel-groups.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.manipulate-panel-groups.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.manipulate-panel-groups
  feature_name: Arrange and group panels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Arrange and group panels to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Arrange and group panels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Arrange and group panels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/manipulate-panel-groups.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.move-panels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.move-panels.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.move-panels
  feature_name: Move panels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move panels to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Move panels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Move panels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/move-panels.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.rearrange-document-windows.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.rearrange-document-windows.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.rearrange-document-windows
  feature_name: Rearrange document windows
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rearrange document windows to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Rearrange document windows" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Rearrange document windows
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/rearrange-document-windows.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.restore-workspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.restore-workspaces.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.restore-workspaces
  feature_name: Restore workspaces
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Restore workspaces to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Restore workspaces" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Restore workspaces
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/restore-workspaces.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.save-custom-workspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.save-custom-workspaces.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.save-custom-workspaces
  feature_name: Save custom workspaces
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save custom workspaces to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Save custom workspaces" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Save custom workspaces
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/save-custom-workspaces.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.stack-floating-panels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.stack-floating-panels.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.stack-floating-panels
  feature_name: Stack floating panels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Stack floating panels to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Stack floating panels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Stack floating panels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/stack-floating-panels.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.switch-workspaces.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.switch-workspaces.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.switch-workspaces
  feature_name: Switch workspaces
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Switch workspaces to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Switch workspaces" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Switch workspaces
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/switch-workspaces.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.use-simple-math.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.use-simple-math.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.use-simple-math
  feature_name: Use simple math in number fields
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use simple math in number fields to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use simple math in number fields" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use simple math in number fields
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/use-simple-math.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.learn-the-basics.workspace-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.learn-the-basics.workspace-overview.v0
  source_feature_id: photoshop.leaf.get-started.learn-the-basics.workspace-overview
  feature_name: Workspace overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: learn-the-basics
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Workspace overview to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Workspace overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Workspace overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/workspace-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.create-tool-preset.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.create-tool-preset.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.create-tool-preset
  feature_name: Create tool presets
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create tool presets to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Create tool presets" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Create tool presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/create-tool-preset.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.create-work-snapshots.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.create-work-snapshots.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.create-work-snapshots
  feature_name: Use snapshots in the History panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use snapshots in the History panel to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use snapshots in the History panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use snapshots in the History panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/create-work-snapshots.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.customize-the-toolbar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.customize-the-toolbar.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.customize-the-toolbar
  feature_name: Customize toolbar
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Customize toolbar to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Customize toolbar" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Customize toolbar
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/customize-the-toolbar.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.edit-images-with-ai-assistant.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.edit-images-with-ai-assistant.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.edit-images-with-ai-assistant
  feature_name: Edit images with AI Assistant
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit images with AI Assistant to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Edit images with AI Assistant" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Edit images with AI Assistant
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/edit-images-with-ai-assistant.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.history-log-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.history-log-preferences.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.history-log-preferences
  feature_name: Set History Log preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set History Log preferences to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Set History Log preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Set History Log preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/history-log-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.history-panel-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.history-panel-overview.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.history-panel-overview
  feature_name: History panel settings
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use History panel settings to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "History panel settings" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / History panel settings
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/history-panel-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.manage-image-states.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.manage-image-states.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.manage-image-states
  feature_name: Manage image states
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage image states to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Manage image states" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Manage image states
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/manage-image-states.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.paint-image-states.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.paint-image-states.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.paint-image-states
  feature_name: Paint with image states from the History panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paint with image states from the History panel to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Paint with image states from the History panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Paint with image states from the History panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/paint-image-states.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.restore-image-parts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.restore-image-parts.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.restore-image-parts
  feature_name: Restore parts of an image to a previous state
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Restore parts of an image to a previous state to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Restore parts of an image to a previous state" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Restore parts of an image to a previous state
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/restore-image-parts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.show-or-hide-tool-tips.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.show-or-hide-tool-tips.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.show-or-hide-tool-tips
  feature_name: Tooltips overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Tooltips overview to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Tooltips overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Tooltips overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/show-or-hide-tool-tips.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.spring-loaded-shortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.spring-loaded-shortcuts.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.spring-loaded-shortcuts
  feature_name: Use spring-loaded shortcuts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use spring-loaded shortcuts to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use spring-loaded shortcuts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use spring-loaded shortcuts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/spring-loaded-shortcuts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.use-undo-redo-commands.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.use-undo-redo-commands.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.use-undo-redo-commands
  feature_name: Use Undo and Redo commands
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Undo and Redo commands to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use Undo and Redo commands" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use Undo and Redo commands
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/use-undo-redo-commands.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.set-up-toolbars-panels.view-history-logs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.set-up-toolbars-panels.view-history-logs.v0
  source_feature_id: photoshop.leaf.get-started.set-up-toolbars-panels.view-history-logs
  feature_name: View history logs
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: set-up-toolbars-panels
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View history logs to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "View history logs" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / View history logs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/view-history-logs.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.settings-and-preferences.adjust-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.settings-and-preferences.adjust-preferences.v0
  source_feature_id: photoshop.leaf.get-started.settings-and-preferences.adjust-preferences
  feature_name: Adjust preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: settings-and-preferences
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust preferences to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Adjust preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/settings-and-preferences/adjust-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.settings-and-preferences.backup-and-restore-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.settings-and-preferences.backup-and-restore-preferences.v0
  source_feature_id: photoshop.leaf.get-started.settings-and-preferences.backup-and-restore-preferences
  feature_name: Backup and restore preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: settings-and-preferences
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Backup and restore preferences to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Backup and restore preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Backup and restore preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/settings-and-preferences/backup-and-restore-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.settings-and-preferences.change-tool-pointers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.settings-and-preferences.change-tool-pointers.v0
  source_feature_id: photoshop.leaf.get-started.settings-and-preferences.change-tool-pointers
  feature_name: Change tool pointers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: settings-and-preferences
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change tool pointers to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Change tool pointers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Change tool pointers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/settings-and-preferences/change-tool-pointers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.settings-and-preferences.reset-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.settings-and-preferences.reset-preferences.v0
  source_feature_id: photoshop.leaf.get-started.settings-and-preferences.reset-preferences
  feature_name: Reset preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: settings-and-preferences
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reset preferences to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Reset preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Reset preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/settings-and-preferences/reset-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.get-started.settings-and-preferences.view-keyboard-shortcuts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.get-started.settings-and-preferences.view-keyboard-shortcuts.v0
  source_feature_id: photoshop.leaf.get-started.settings-and-preferences.view-keyboard-shortcuts
  feature_name: View keyboard shortcuts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: get-started
  source_subcategory: settings-and-preferences
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View keyboard shortcuts to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "View keyboard shortcuts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / View keyboard shortcuts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/get-started/settings-and-preferences/view-keyboard-shortcuts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.detect-subject-using-select-subject.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.detect-subject-using-select-subject.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.detect-subject-using-select-subject
  feature_name: Detect subject using Select Subject
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Detect subject using Select Subject to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Detect subject using Select Subject" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Detect subject using Select Subject
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/detect-subject-using-select-subject.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.improved-select-subject-and-remove-background-results.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.improved-select-subject-and-remove-background-results.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.improved-select-subject-and-remove-background-results
  feature_name: Improve Select Subject and Remove Background results
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Improve Select Subject and Remove Background results to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Improve Select Subject and Remove Background results" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Improve Select Subject and Remove Background results
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/improved-select-subject-and-remove-background-results.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.make-improved-hair-selections.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.make-improved-hair-selections.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.make-improved-hair-selections
  feature_name: Improve hair selections with the Refine Hair tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Improve hair selections with the Refine Hair tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Improve hair selections with the Refine Hair tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Improve hair selections with the Refine Hair tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/make-improved-hair-selections.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.make-precise-selections-using-select-people.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.make-precise-selections-using-select-people.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.make-precise-selections-using-select-people
  feature_name: Make precise selections using Select People
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Make precise selections using Select People to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Make precise selections using Select People" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Make precise selections using Select People
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/make-precise-selections-using-select-people.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.mask-all-objects-in-a-layer.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.mask-all-objects-in-a-layer.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.mask-all-objects-in-a-layer
  feature_name: Mask all objects in a layer
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Mask all objects in a layer to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Mask all objects in a layer" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Mask all objects in a layer
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/mask-all-objects-in-a-layer.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.paint-a-selection-with-quick-selection-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.paint-a-selection-with-quick-selection-tool.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.paint-a-selection-with-quick-selection-tool
  feature_name: Paint a selection with Quick Selection tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paint a selection with Quick Selection tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Paint a selection with Quick Selection tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Paint a selection with Quick Selection tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/paint-a-selection-with-quick-selection-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.remove-objects-with-delete-and-fill-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.remove-objects-with-delete-and-fill-selection.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.remove-objects-with-delete-and-fill-selection
  feature_name: Remove objects with Delete and Fill Selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove objects with Delete and Fill Selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Remove objects with Delete and Fill Selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Remove objects with Delete and Fill Selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/remove-objects-with-delete-and-fill-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.automatic-color-based-selections.select-areas-by-color-with-the-magic-wand-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.automatic-color-based-selections.select-areas-by-color-with-the-magic-wand-tool.v0
  source_feature_id: photoshop.leaf.make-selections.automatic-color-based-selections.select-areas-by-color-with-the-magic-wand-tool
  feature_name: Select areas by color with the Magic Wand tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: automatic-color-based-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select areas by color with the Magic Wand tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Select areas by color with the Magic Wand tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select areas by color with the Magic Wand tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/automatic-color-based-selections/select-areas-by-color-with-the-magic-wand-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.create-quick-selections-with-selection-brush-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.freehand-selections.create-quick-selections-with-selection-brush-tool.v0
  source_feature_id: photoshop.leaf.make-selections.freehand-selections.create-quick-selections-with-selection-brush-tool
  feature_name: Create quick selections with the Selection Brush Tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: freehand-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create quick selections with the Selection Brush Tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Create quick selections with the Selection Brush Tool" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Create quick selections with the Selection Brush Tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/create-quick-selections-with-selection-brush-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.draw-freeform-segments-of-a-selection-border.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.freehand-selections.draw-freeform-segments-of-a-selection-border.v0
  source_feature_id: photoshop.leaf.make-selections.freehand-selections.draw-freeform-segments-of-a-selection-border
  feature_name: Draw freeform selections with the Lasso tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: freehand-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw freeform selections with the Lasso tool to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Draw freeform selections with the Lasso tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Draw freeform selections with the Lasso tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/draw-freeform-segments-of-a-selection-border.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.draw-straight-edged-segments-of-a-selection-border.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.freehand-selections.draw-straight-edged-segments-of-a-selection-border.v0
  source_feature_id: photoshop.leaf.make-selections.freehand-selections.draw-straight-edged-segments-of-a-selection-border
  feature_name: Draw straight-edged segments of a selection border
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: freehand-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw straight-edged segments of a selection border to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Draw straight-edged segments of a selection border" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Draw straight-edged segments of a selection border
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/draw-straight-edged-segments-of-a-selection-border.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.save-skin-tones-settings-as-a-preset.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.freehand-selections.save-skin-tones-settings-as-a-preset.v0
  source_feature_id: photoshop.leaf.make-selections.freehand-selections.save-skin-tones-settings-as-a-preset
  feature_name: Save Skin Tones settings as a preset
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: freehand-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save Skin Tones settings as a preset to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Save Skin Tones settings as a preset" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Save Skin Tones settings as a preset
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/save-skin-tones-settings-as-a-preset.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.select-a-color-range-in-photoshop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.freehand-selections.select-a-color-range-in-photoshop.v0
  source_feature_id: photoshop.leaf.make-selections.freehand-selections.select-a-color-range-in-photoshop
  feature_name: Select a Color Range in Photoshop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: freehand-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select a Color Range in Photoshop to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Select a Color Range in Photoshop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select a Color Range in Photoshop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/select-a-color-range-in-photoshop.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.freehand-selections.snap-to-image-edges-using-magnetic-lasso-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.freehand-selections.snap-to-image-edges-using-magnetic-lasso-tool.v0
  source_feature_id: photoshop.leaf.make-selections.freehand-selections.snap-to-image-edges-using-magnetic-lasso-tool
  feature_name: Snap to image edges using Magnetic Lasso tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: freehand-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Snap to image edges using Magnetic Lasso tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Snap to image edges using Magnetic Lasso tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Snap to image edges using Magnetic Lasso tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/freehand-selections/snap-to-image-edges-using-magnetic-lasso-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.get-started-selections.select-objects-with-object-selection-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.get-started-selections.select-objects-with-object-selection-tool.v0
  source_feature_id: photoshop.leaf.make-selections.get-started-selections.select-objects-with-object-selection-tool
  feature_name: Use the Object Selection tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: get-started-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use the Object Selection tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Use the Object Selection tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Use the Object Selection tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/get-started-selections/select-objects-with-object-selection-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.get-started-selections.selection-tools-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.get-started-selections.selection-tools-overview.v0
  source_feature_id: photoshop.leaf.make-selections.get-started-selections.selection-tools-overview
  feature_name: Selection tools overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: get-started-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Selection tools overview to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Selection tools overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Selection tools overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/get-started-selections/selection-tools-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.adjust-a-selection-manually.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.adjust-a-selection-manually.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.adjust-a-selection-manually
  feature_name: Adjust a selection manually
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust a selection manually to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust a selection manually" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Adjust a selection manually
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/adjust-a-selection-manually.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.clean-up-stray-pixels-in-color-based-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.clean-up-stray-pixels-in-color-based-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.clean-up-stray-pixels-in-color-based-selection
  feature_name: Clean up stray pixels in a color-based selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Clean up stray pixels in a color-based selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Clean up stray pixels in a color-based selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Clean up stray pixels in a color-based selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/clean-up-stray-pixels-in-color-based-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.control-the-movement-of-a-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.control-the-movement-of-a-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.control-the-movement-of-a-selection
  feature_name: Control the movement of a selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Control the movement of a selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Control the movement of a selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Control the movement of a selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/control-the-movement-of-a-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.copy-and-paste-selections.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.copy-and-paste-selections.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.copy-and-paste-selections
  feature_name: Copy and paste selections
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Copy and paste selections to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Copy and paste selections" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Copy and paste selections
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/copy-and-paste-selections.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.create-multiple-copies-of-a-selection-within-an-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.create-multiple-copies-of-a-selection-within-an-image.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.create-multiple-copies-of-a-selection-within-an-image
  feature_name: Create multiple copies of a selection within an image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create multiple copies of a selection within an image to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Create multiple copies of a selection within an image" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Create multiple copies of a selection within an image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/create-multiple-copies-of-a-selection-within-an-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.create-selection-around-selection-border.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.create-selection-around-selection-border.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.create-selection-around-selection-border
  feature_name: Create a selection around a selection border
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create a selection around a selection border to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Create a selection around a selection border" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Create a selection around a selection border
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/create-selection-around-selection-border.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.decrease-fringe-on-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.decrease-fringe-on-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.decrease-fringe-on-selection
  feature_name: Decrease the fringe on a selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Decrease the fringe on a selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Decrease the fringe on a selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Decrease the fringe on a selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/decrease-fringe-on-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.define-feathered-edges.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.define-feathered-edges.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.define-feathered-edges
  feature_name: Define a feathered edge for a selection tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Define a feathered edge for a selection tool to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Define a feathered edge for a selection tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Define a feathered edge for a selection tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/define-feathered-edges.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.delete-or-cut-selected-pixels.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.delete-or-cut-selected-pixels.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.delete-or-cut-selected-pixels
  feature_name: Delete or cut selected pixels
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete or cut selected pixels to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Delete or cut selected pixels" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Delete or cut selected pixels
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/delete-or-cut-selected-pixels.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.expand-or-contract-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.expand-or-contract-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.expand-or-contract-selection
  feature_name: Expand or contract a selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Expand or contract a selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Expand or contract a selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Expand or contract a selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/expand-or-contract-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.fringe-pixels-around-a-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.fringe-pixels-around-a-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.fringe-pixels-around-a-selection
  feature_name: Fringe pixels around a selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fringe pixels around a selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Fringe pixels around a selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Fringe pixels around a selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/fringe-pixels-around-a-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.hide-or-show-selection-edges.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.hide-or-show-selection-edges.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.hide-or-show-selection-edges
  feature_name: Hide or show selection edges
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Hide or show selection edges to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Hide or show selection edges" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Hide or show selection edges
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/hide-or-show-selection-edges.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.hover-layer-bounds-in-the-move-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.hover-layer-bounds-in-the-move-tool.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.hover-layer-bounds-in-the-move-tool
  feature_name: Hover layer bounds in the Move tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Hover layer bounds in the Move tool to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Hover layer bounds in the Move tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Hover layer bounds in the Move tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/hover-layer-bounds-in-the-move-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.inverse-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.inverse-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.inverse-selection
  feature_name: Inverse selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Inverse selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Inverse selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Inverse selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/inverse-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.move-selection-or-selection-border.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.move-selection-or-selection-border.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.move-selection-or-selection-border
  feature_name: Move a selection or selection border
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move a selection or selection border to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Move a selection or selection border" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Move a selection or selection border
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/move-selection-or-selection-border.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.paste-one-selection-into-or-outside-another.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.paste-one-selection-into-or-outside-another.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.paste-one-selection-into-or-outside-another
  feature_name: Paste one selection into or outside another
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paste one selection into or outside another to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Paste one selection into or outside another" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Paste one selection into or outside another
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/paste-one-selection-into-or-outside-another.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.refine-and-soften-selection-edges.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.refine-and-soften-selection-edges.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.refine-and-soften-selection-edges
  feature_name: Refine and soften selection edges
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Refine and soften selection edges to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Refine and soften selection edges" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Refine and soften selection edges
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/refine-and-soften-selection-edges.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.refine-your-selection-and-mask.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.refine-your-selection-and-mask.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.refine-your-selection-and-mask
  feature_name: Refine your selection and mask
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Refine your selection and mask to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Refine your selection and mask" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Refine your selection and mask
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/refine-your-selection-and-mask.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.remove-matte-from-selection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.remove-matte-from-selection.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.remove-matte-from-selection
  feature_name: Remove a matte from a selection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove a matte from a selection to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Remove a matte from a selection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Remove a matte from a selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/remove-matte-from-selection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.select-area-intersected-by-other-selections.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.select-area-intersected-by-other-selections.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.select-area-intersected-by-other-selections
  feature_name: Select only an area intersected by other selections
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select only an area intersected by other selections to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Select only an area intersected by other selections" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select only an area intersected by other selections
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/select-area-intersected-by-other-selections.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.make-selections.refine-modify-selections.select-pixels-using-anti-aliasing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.make-selections.refine-modify-selections.select-pixels-using-anti-aliasing.v0
  source_feature_id: photoshop.leaf.make-selections.refine-modify-selections.select-pixels-using-anti-aliasing
  feature_name: Select pixels using anti-aliasing
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: make-selections
  source_subcategory: refine-modify-selections
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select pixels using anti-aliasing to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Select pixels using anti-aliasing" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select pixels using anti-aliasing
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/make-selections/refine-modify-selections/select-pixels-using-anti-aliasing.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.adjust-light-tone.blending-mode-descriptions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.adjust-light-tone.blending-mode-descriptions.v0
  source_feature_id: photoshop.leaf.repair-retouch.adjust-light-tone.blending-mode-descriptions
  feature_name: Blending mode descriptions
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: adjust-light-tone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blending mode descriptions to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Blending mode descriptions" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Blending mode descriptions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/adjust-light-tone/blending-mode-descriptions.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.adjust-light-tone.darken-the-edges-of-your-image-to-bring-focus-to-its-center.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.adjust-light-tone.darken-the-edges-of-your-image-to-bring-focus-to-its-center.v0
  source_feature_id: photoshop.leaf.repair-retouch.adjust-light-tone.darken-the-edges-of-your-image-to-bring-focus-to-its-center
  feature_name: Darken the edges of your image to bring focus to the center
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: adjust-light-tone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Darken the edges of your image to bring focus to the center to modify pixel content or raster-derived appearance through a Studio command that
    can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Darken the edges of your image to bring focus to the center" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Darken the edges of your image to bring focus to the center
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/adjust-light-tone/darken-the-edges-of-your-image-to-bring-focus-to-its-center.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.adjust-light-tone.dodge-or-burn-image-areas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.adjust-light-tone.dodge-or-burn-image-areas.v0
  source_feature_id: photoshop.leaf.repair-retouch.adjust-light-tone.dodge-or-burn-image-areas
  feature_name: Dodge or burn image areas
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: adjust-light-tone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Dodge or burn image areas to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Dodge or burn image areas" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Dodge or burn image areas
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/adjust-light-tone/dodge-or-burn-image-areas.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.auto-erase-with-the-pencil-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.auto-erase-with-the-pencil-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.auto-erase-with-the-pencil-tool
  feature_name: Auto Erase with the Pencil tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Auto Erase with the Pencil tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Auto Erase with the Pencil tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Auto Erase with the Pencil tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/auto-erase-with-the-pencil-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.change-pixels-to-transparent-with-the-background-eraser-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.change-pixels-to-transparent-with-the-background-eraser-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.change-pixels-to-transparent-with-the-background-eraser-tool
  feature_name: Change pixels to transparent with the Background Eraser tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change pixels to transparent with the Background Eraser tool to modify pixel content or raster-derived appearance through a Studio command that
    can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Change pixels to transparent with the Background Eraser tool" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Change pixels to transparent with the Background Eraser tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/change-pixels-to-transparent-with-the-background-eraser-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.change-similar-pixels-with-the-magic-eraser-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.change-similar-pixels-with-the-magic-eraser-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.change-similar-pixels-with-the-magic-eraser-tool
  feature_name: Remove similar pixels with the Magic Eraser tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove similar pixels with the Magic Eraser tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove similar pixels with the Magic Eraser tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove similar pixels with the Magic Eraser tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/change-similar-pixels-with-the-magic-eraser-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.create-360-degree-panoramas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.create-360-degree-panoramas.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.create-360-degree-panoramas
  feature_name: Create 360-degree panoramas
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create 360-degree panoramas to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create 360-degree panoramas" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create 360-degree panoramas
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/create-360-degree-panoramas.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.create-panoramic-images-with-photomerge.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.create-panoramic-images-with-photomerge.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.create-panoramic-images-with-photomerge
  feature_name: Create panoramic images with Photomerge
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create panoramic images with Photomerge to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create panoramic images with Photomerge" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create panoramic images with Photomerge
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/create-panoramic-images-with-photomerge.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.create-smoother-more-polished-brush-strokes-with-stroke-smoothing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.create-smoother-more-polished-brush-strokes-with-stroke-smoothing.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.create-smoother-more-polished-brush-strokes-with-stroke-smoothing
  feature_name: Create smoother brush strokes using stroke smoothing
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create smoother brush strokes using stroke smoothing to modify pixel content or raster-derived appearance through a Studio command that can be
    previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Create smoother brush strokes using stroke smoothing" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Create smoother brush strokes using stroke smoothing
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/create-smoother-more-polished-brush-strokes-with-stroke-smoothing.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.define-an-image-as-a-preset-pattern.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.define-an-image-as-a-preset-pattern.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.define-an-image-as-a-preset-pattern
  feature_name: Define an image as a preset pattern
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Define an image as a preset pattern to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Define an image as a preset pattern" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Define an image as a preset pattern
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/define-an-image-as-a-preset-pattern.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.define-planes-to-adjust-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.define-planes-to-adjust-perspective.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.define-planes-to-adjust-perspective
  feature_name: Define planes to adjust perspective
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Define planes to adjust perspective to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Define planes to adjust perspective" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Define planes to adjust perspective
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/define-planes-to-adjust-perspective.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.edit-different-perspectives-in-the-same-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.edit-different-perspectives-in-the-same-image.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.edit-different-perspectives-in-the-same-image
  feature_name: Edit different perspectives in the same image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit different perspectives in the same image to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Edit different perspectives in the same image" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Edit different perspectives in the same image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/edit-different-perspectives-in-the-same-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.enhance-image-quality-with-generative-upscale.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.enhance-image-quality-with-generative-upscale.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.enhance-image-quality-with-generative-upscale
  feature_name: Enhance image quality with Generative Upscale
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Enhance image quality with Generative Upscale to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Enhance image quality with Generative Upscale" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Enhance image quality with Generative Upscale
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/enhance-image-quality-with-generative-upscale.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.erase-parts-of-an-image-with-the-eraser-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.erase-parts-of-an-image-with-the-eraser-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.erase-parts-of-an-image-with-the-eraser-tool
  feature_name: Erase parts of an image with the Eraser tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Erase parts of an image with the Eraser tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Erase parts of an image with the Eraser tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Erase parts of an image with the Eraser tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/erase-parts-of-an-image-with-the-eraser-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.get-started-with-photomerge.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.get-started-with-photomerge.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.get-started-with-photomerge
  feature_name: Get started with Photomerge
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get started with Photomerge to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Get started with Photomerge" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Get started with Photomerge
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/get-started-with-photomerge.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.healing-brush-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.healing-brush-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.healing-brush-tool
  feature_name: Retouch a large area with the Healing Brush tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Retouch a large area with the Healing Brush tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Retouch a large area with the Healing Brush tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Retouch a large area with the Healing Brush tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/healing-brush-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.keyboard-shortcuts-to-adjust-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.keyboard-shortcuts-to-adjust-perspective.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.keyboard-shortcuts-to-adjust-perspective
  feature_name: Keyboard shortcuts to adjust perspective
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Keyboard shortcuts to adjust perspective to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Keyboard shortcuts to adjust perspective" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Keyboard shortcuts to adjust perspective
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/keyboard-shortcuts-to-adjust-perspective.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.manipulate-the-planes-to-adjust-perspective.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.manipulate-the-planes-to-adjust-perspective.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.manipulate-the-planes-to-adjust-perspective
  feature_name: Manipulate the planes to adjust perspective
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manipulate the planes to adjust perspective to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Manipulate the planes to adjust perspective" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Manipulate the planes to adjust perspective
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/manipulate-the-planes-to-adjust-perspective.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.remove-red-eye-in-flash-photos.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.remove-red-eye-in-flash-photos.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.remove-red-eye-in-flash-photos
  feature_name: Remove red eye in flash photos
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove red eye in flash photos to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove red eye in flash photos" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove red eye in flash photos
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/remove-red-eye-in-flash-photos.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.remove-reflections.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.remove-reflections.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.remove-reflections
  feature_name: Remove reflections
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove reflections to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove reflections" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove reflections
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/remove-reflections.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.repair-a-selected-area-with-the-patch-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.repair-a-selected-area-with-the-patch-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.repair-a-selected-area-with-the-patch-tool
  feature_name: Repair an area with the Patch tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Repair an area with the Patch tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Repair an area with the Patch tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Repair an area with the Patch tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/repair-a-selected-area-with-the-patch-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.clean-restore-images.spot-healing-brush-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.clean-restore-images.spot-healing-brush-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.clean-restore-images.spot-healing-brush-tool
  feature_name: Remove imperfections with the Spot Healing Brush
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: clean-restore-images
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove imperfections with the Spot Healing Brush to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Remove imperfections with the Spot Healing Brush" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Remove imperfections with the Spot Healing Brush
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/clean-restore-images/spot-healing-brush-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.heal-clone.adjust-the-sample-source-overlay-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.heal-clone.adjust-the-sample-source-overlay-options.v0
  source_feature_id: photoshop.leaf.repair-retouch.heal-clone.adjust-the-sample-source-overlay-options
  feature_name: Adjust the sample source overlay options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: heal-clone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust the sample source overlay options to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust the sample source overlay options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Adjust the sample source overlay options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/heal-clone/adjust-the-sample-source-overlay-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.heal-clone.clone-source-panel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.heal-clone.clone-source-panel.v0
  source_feature_id: photoshop.leaf.repair-retouch.heal-clone.clone-source-panel
  feature_name: Clone Source panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: heal-clone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Clone Source panel to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Clone Source panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Clone Source panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/heal-clone/clone-source-panel.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.heal-clone.retouch-images-with-the-clone-stamp-tool.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.heal-clone.retouch-images-with-the-clone-stamp-tool.v0
  source_feature_id: photoshop.leaf.repair-retouch.heal-clone.retouch-images-with-the-clone-stamp-tool
  feature_name: Retouch images with the Clone Stamp tool
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: heal-clone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Retouch images with the Clone Stamp tool to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Retouch images with the Clone Stamp tool" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Retouch images with the Clone Stamp tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/heal-clone/retouch-images-with-the-clone-stamp-tool.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.heal-clone.scale-or-rotate-the-sample-source.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.heal-clone.scale-or-rotate-the-sample-source.v0
  source_feature_id: photoshop.leaf.repair-retouch.heal-clone.scale-or-rotate-the-sample-source
  feature_name: Scale or rotate the sample source
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: heal-clone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale or rotate the sample source to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Scale or rotate the sample source" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Scale or rotate the sample source
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/heal-clone/scale-or-rotate-the-sample-source.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.heal-clone.set-sample-sources-for-cloning-and-healing.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.heal-clone.set-sample-sources-for-cloning-and-healing.v0
  source_feature_id: photoshop.leaf.repair-retouch.heal-clone.set-sample-sources-for-cloning-and-healing
  feature_name: Set sample sources for cloning and healing
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: heal-clone
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set sample sources for cloning and healing to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Set sample sources for cloning and healing" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Set sample sources for cloning and healing
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/heal-clone/set-sample-sources-for-cloning-and-healing.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.adjust-content-aware-fill-settings.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.adjust-content-aware-fill-settings.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.adjust-content-aware-fill-settings
  feature_name: Adjust Content-Aware Fill settings
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust Content-Aware Fill settings to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust Content-Aware Fill settings" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Adjust Content-Aware Fill settings
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/adjust-content-aware-fill-settings.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.apply-or-cancel-fill-changes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.apply-or-cancel-fill-changes.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.apply-or-cancel-fill-changes
  feature_name: Apply or cancel Content-Aware fill changes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply or cancel Content-Aware fill changes to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Apply or cancel Content-Aware fill changes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Apply or cancel Content-Aware fill changes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/apply-or-cancel-fill-changes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.blend-subjects-with-harmonize.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.blend-subjects-with-harmonize.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.blend-subjects-with-harmonize
  feature_name: Blend objects and people into any background with Harmonize
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blend objects and people into any background with Harmonize to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "Blend objects and people into any background with Harmonize" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Blend objects and people into any background with Harmonize
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/blend-subjects-with-harmonize.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-background-in-your-images.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-background-in-your-images.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-background-in-your-images
  feature_name: Remove background in your images
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove background in your images to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove background in your images" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove background in your images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/remove-background-in-your-images.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-objects-from-contextual-task-bar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-objects-from-contextual-task-bar.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-objects-from-contextual-task-bar
  feature_name: Remove objects from within the Contextual Task Bar
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove objects from within the Contextual Task Bar to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove objects from within the Contextual Task Bar" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove objects from within the Contextual Task Bar
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/remove-objects-from-contextual-task-bar.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-objects-with-content-aware-fill.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-objects-with-content-aware-fill.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-objects-with-content-aware-fill
  feature_name: Remove objects and fill the area with Content-Aware Fill
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove objects and fill the area with Content-Aware Fill to modify pixel content or raster-derived appearance through a Studio command that can
    be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove objects and fill the area with Content-Aware Fill" without needing hidden source-app
    context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove objects and fill the area with Content-Aware Fill
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/remove-objects-with-content-aware-fill.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-unwanted-objects-and-distractions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-unwanted-objects-and-distractions.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-unwanted-objects-and-distractions
  feature_name: Remove objects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove objects to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove objects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/remove-unwanted-objects-and-distractions.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-wires-people-distractions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-wires-people-distractions.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.remove-wires-people-distractions
  feature_name: Remove wires, people, and general distractions
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove wires, people, and general distractions to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove wires, people, and general distractions" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove wires, people, and general distractions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/remove-wires-people-distractions.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.replace-background-with-generate-background.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.replace-background-with-generate-background.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.replace-background-with-generate-background
  feature_name: Replace background with Generate Background
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace background with Generate Background to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Replace background with Generate Background" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Replace background with Generate Background
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/replace-background-with-generate-background.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.retouch-tools-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.retouch-tools-overview.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.retouch-tools-overview
  feature_name: Retouch tools overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Retouch tools overview to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Retouch tools overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Retouch tools overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/retouch-tools-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.review-and-refine-general-distractions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.review-and-refine-general-distractions.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.review-and-refine-general-distractions
  feature_name: Review and refine general distractions
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: optional_integration
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Review and refine general distractions to modify pixel content or raster-derived appearance through a Studio command that can be previewed and
    audited.
  user_goal: A Studio operator can perform the source-app workflow named "Review and refine general distractions" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Review and refine general distractions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/review-and-refine-general-distractions.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.tools-to-fine-tune-sampling-and-fill-areas.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.tools-to-fine-tune-sampling-and-fill-areas.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.tools-to-fine-tune-sampling-and-fill-areas
  feature_name: Tools to fine-tune sampling and fill areas
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Tools to fine-tune sampling and fill areas to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Tools to fine-tune sampling and fill areas" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Tools to fine-tune sampling and fill areas
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/tools-to-fine-tune-sampling-and-fill-areas.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.repair-retouch.remove-objects-fill-space.view-full-resolution-preview-in-the-preview-panel.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.repair-retouch.remove-objects-fill-space.view-full-resolution-preview-in-the-preview-panel.v0
  source_feature_id: photoshop.leaf.repair-retouch.remove-objects-fill-space.view-full-resolution-preview-in-the-preview-panel
  feature_name: View full-resolution preview in the Preview panel
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: repair-retouch
  source_subcategory: remove-objects-fill-space
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: optional_integration
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View full-resolution preview in the Preview panel to modify pixel content or raster-derived appearance through a Studio command that can be previewed
    and audited.
  user_goal: A Studio operator can perform the source-app workflow named "View full-resolution preview in the Preview panel" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / View full-resolution preview in the Preview panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/repair-retouch/remove-objects-fill-space/view-full-resolution-preview-in-the-preview-panel.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.enhance-animation-frames.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.enhance-animation-frames.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.enhance-animation-frames
  feature_name: Enhance animation frames
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Enhance animation frames to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Enhance animation frames" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Enhance animation frames
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/enhance-animation-frames.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-files.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-files.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-files
  feature_name: Export artboards as files
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export artboards as files to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Export artboards as files" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export artboards as files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-artboards-as-files.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-pdf.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-pdf.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-artboards-as-pdf
  feature_name: Export artboards as PDF
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export artboards as PDF to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Export artboards as PDF" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export artboards as PDF
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-artboards-as-pdf.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-files-in-different-sizes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-files-in-different-sizes.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-files-in-different-sizes
  feature_name: Export files in different sizes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export files in different sizes to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Export files in different sizes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export files in different sizes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-files-in-different-sizes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-layers-as-files.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-layers-as-files.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-layers-as-files
  feature_name: Export layers as files
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export layers as files to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Export layers as files" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export layers as files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-layers-as-files.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-settings-and-export-location-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-settings-and-export-location-preferences.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-settings-and-export-location-preferences
  feature_name: Export settings and export location preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export settings and export location preferences to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Export settings and export location preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export settings and export location preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-settings-and-export-location-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-to-cloud.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-to-cloud.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-to-cloud
  feature_name: Save and export to cloud
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: optional_integration
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save and export to cloud to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Save and export to cloud" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Save and export to cloud
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-to-cloud.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-video-files-or-image-sequences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-video-files-or-image-sequences.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-video-files-or-image-sequences
  feature_name: Export video files or image sequences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export video files or image sequences to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Export video files or image sequences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export video files or image sequences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-video-files-or-image-sequences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.export-your-work-using-the-quick-export-as-option.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.export-your-work-using-the-quick-export-as-option.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.export-your-work-using-the-quick-export-as-option
  feature_name: Export your work using the Quick Export as option
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export your work using the Quick Export as option to define interactive, form, animation, or media behavior for documents that support runtime
    output.
  user_goal: A Studio operator can perform the source-app workflow named "Export your work using the Quick Export as option" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Export your work using the Quick Export as option
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/export-your-work-using-the-quick-export-as-option.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.file-compression-in-photoshop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.file-compression-in-photoshop.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.file-compression-in-photoshop
  feature_name: File compression in Photoshop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use File compression in Photoshop to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "File compression in Photoshop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / File compression in Photoshop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/file-compression-in-photoshop.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.fine-tune-your-export-settings-using-the-export-as-option.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.fine-tune-your-export-settings-using-the-export-as-option.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.fine-tune-your-export-settings-using-the-export-as-option
  feature_name: Fine-tune export settings with Export As
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fine-tune export settings with Export As to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Fine-tune export settings with Export As" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Fine-tune export settings with Export As
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/fine-tune-your-export-settings-using-the-export-as-option.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.flatten-frames-into-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.flatten-frames-into-layers.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.flatten-frames-into-layers
  feature_name: Flatten frames into layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Flatten frames into layers to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Flatten frames into layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Flatten frames into layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/flatten-frames-into-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.photoshop-file-formats-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.photoshop-file-formats-overview.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.photoshop-file-formats-overview
  feature_name: Photoshop file formats overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Photoshop file formats overview to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Photoshop file formats overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Photoshop file formats overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/photoshop-file-formats-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.export-files-to-different-formats.video-and-animation-export-formats.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.export-files-to-different-formats.video-and-animation-export-formats.v0
  source_feature_id: photoshop.leaf.save-and-export.export-files-to-different-formats.video-and-animation-export-formats
  feature_name: Video and animation export formats
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: export-files-to-different-formats
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Video and animation export formats to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Video and animation export formats" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Video and animation export formats
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/video-and-animation-export-formats.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.metadata-content-credentials.export-your-work-with-content-credentials.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.metadata-content-credentials.export-your-work-with-content-credentials.v0
  source_feature_id: photoshop.leaf.save-and-export.metadata-content-credentials.export-your-work-with-content-credentials
  feature_name: Export your work with Content Credentials
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: metadata-content-credentials
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export your work with Content Credentials to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Export your work with Content Credentials" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Export your work with Content Credentials
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/metadata-content-credentials/export-your-work-with-content-credentials.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.metadata-content-credentials.preview-content-credentials.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.metadata-content-credentials.preview-content-credentials.v0
  source_feature_id: photoshop.leaf.save-and-export.metadata-content-credentials.preview-content-credentials
  feature_name: Preview Content Credentials
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: metadata-content-credentials
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Preview Content Credentials to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Preview Content Credentials" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Preview Content Credentials
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/metadata-content-credentials/preview-content-credentials.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.metadata-content-credentials.use-content-credentials.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.metadata-content-credentials.use-content-credentials.v0
  source_feature_id: photoshop.leaf.save-and-export.metadata-content-credentials.use-content-credentials
  feature_name: Use Content Credentials
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: metadata-content-credentials
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Content Credentials to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use Content Credentials" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use Content Credentials
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/metadata-content-credentials/use-content-credentials.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.common-questions-on-photoshop-cloud-documents.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.common-questions-on-photoshop-cloud-documents.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.common-questions-on-photoshop-cloud-documents
  feature_name: Common questions on Photoshop cloud documents
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Common questions on Photoshop cloud documents to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Common questions on Photoshop cloud documents" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Common questions on Photoshop cloud documents
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/common-questions-on-photoshop-cloud-documents.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.file-saving-properties-and-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.file-saving-properties-and-preferences.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.file-saving-properties-and-preferences
  feature_name: File saving properties and preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use File saving properties and preferences to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "File saving properties and preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / File saving properties and preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/file-saving-properties-and-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.macos-image-preview-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.macos-image-preview-options.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.macos-image-preview-options
  feature_name: macOS image preview options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use macOS image preview options to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "macOS image preview options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / macOS image preview options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/macos-image-preview-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.revert-to-legacy-save-as-options.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.revert-to-legacy-save-as-options.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.revert-to-legacy-save-as-options
  feature_name: Revert to legacy Save As options
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Revert to legacy Save As options to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Revert to legacy Save As options" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Revert to legacy Save As options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/revert-to-legacy-save-as-options.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-as-photoshop-pdf.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.save-as-photoshop-pdf.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.save-as-photoshop-pdf
  feature_name: Save as Photoshop PDF
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save as Photoshop PDF to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Save as Photoshop PDF" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Save as Photoshop PDF
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/save-as-photoshop-pdf.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-for-web.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.save-for-web.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.save-for-web
  feature_name: Save for Web
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save for Web to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Save for Web" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Save for Web
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/save-for-web.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-large-documents.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.save-large-documents.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.save-large-documents
  feature_name: Save large documents
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save large documents to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Save large documents" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Save large documents
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/save-large-documents.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.save-and-export.save-files.save-your-work.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.save-and-export.save-files.save-your-work.v0
  source_feature_id: photoshop.leaf.save-and-export.save-files.save-your-work
  feature_name: Save your work
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: save-and-export
  source_subcategory: save-files
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save your work to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Save your work" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Save your work
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/save-and-export/save-files/save-your-work.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.share-and-collaborate.collaborate-and-edit.share-and-collaborate-with-projects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.share-and-collaborate.collaborate-and-edit.share-and-collaborate-with-projects.v0
  source_feature_id: photoshop.leaf.share-and-collaborate.collaborate-and-edit.share-and-collaborate-with-projects
  feature_name: Share and collaborate with Projects
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: share-and-collaborate
  source_subcategory: collaborate-and-edit
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Share and collaborate with Projects to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Share and collaborate with Projects" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Share and collaborate with Projects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/share-and-collaborate/collaborate-and-edit/share-and-collaborate-with-projects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.share-and-collaborate.collaborate-and-edit.work-with-projects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.share-and-collaborate.collaborate-and-edit.work-with-projects.v0
  source_feature_id: photoshop.leaf.share-and-collaborate.collaborate-and-edit.work-with-projects
  feature_name: Create Projects and add files
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: share-and-collaborate
  source_subcategory: collaborate-and-edit
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create Projects and add files to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Create Projects and add files" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Create Projects and add files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/share-and-collaborate/collaborate-and-edit/work-with-projects.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.characters-glyphs.add-emoji-glyphs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.characters-glyphs.add-emoji-glyphs.v0
  source_feature_id: photoshop.leaf.text-typography.characters-glyphs.add-emoji-glyphs
  feature_name: Add Emoji glyphs
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: characters-glyphs
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add Emoji glyphs to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Add Emoji glyphs" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add Emoji glyphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/characters-glyphs/add-emoji-glyphs.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.characters-glyphs.add-glyphs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.characters-glyphs.add-glyphs.v0
  source_feature_id: photoshop.leaf.text-typography.characters-glyphs.add-glyphs
  feature_name: Add glyphs
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: characters-glyphs
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add glyphs to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Add glyphs" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add glyphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/characters-glyphs/add-glyphs.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.characters-glyphs.enable-glyph-protection.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.characters-glyphs.enable-glyph-protection.v0
  source_feature_id: photoshop.leaf.text-typography.characters-glyphs.enable-glyph-protection
  feature_name: Enable glyph protection
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: characters-glyphs
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Enable glyph protection to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Enable glyph protection" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Enable glyph protection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/characters-glyphs/enable-glyph-protection.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.characters-glyphs.use-on-canvas-glyph-alternatives.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.characters-glyphs.use-on-canvas-glyph-alternatives.v0
  source_feature_id: photoshop.leaf.text-typography.characters-glyphs.use-on-canvas-glyph-alternatives
  feature_name: Use on-canvas glyph alternatives
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: characters-glyphs
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use on-canvas glyph alternatives to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Use on-canvas glyph alternatives" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Use on-canvas glyph alternatives
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/characters-glyphs/use-on-canvas-glyph-alternatives.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.characters-glyphs.work-with-opentype-svg-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.characters-glyphs.work-with-opentype-svg-fonts.v0
  source_feature_id: photoshop.leaf.text-typography.characters-glyphs.work-with-opentype-svg-fonts
  feature_name: Work with OpenType SVG fonts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: characters-glyphs
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with OpenType SVG fonts to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Work with OpenType SVG fonts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Work with OpenType SVG fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/characters-glyphs/work-with-opentype-svg-fonts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.add-bulleted-and-numbered-lists.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.add-bulleted-and-numbered-lists.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.add-bulleted-and-numbered-lists
  feature_name: Add bulleted and numbered lists
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add bulleted and numbered lists to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Add bulleted and numbered lists" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add bulleted and numbered lists
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/add-bulleted-and-numbered-lists.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.add-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.add-text.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.add-text
  feature_name: Add text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add text to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Add text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/add-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.change-text-color.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.change-text-color.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.change-text-color
  feature_name: Change text color
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change text color to control tone, color, gamut, or gradient behavior with explicit color-management context.
  user_goal: A Studio operator can perform the source-app workflow named "Change text color" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Change text color
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/change-text-color.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.copy-and-paste-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.copy-and-paste-text.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.copy-and-paste-text
  feature_name: Copy and paste text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Copy and paste text to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Copy and paste text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Copy and paste text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/copy-and-paste-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.edit-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.edit-text.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.edit-text
  feature_name: Edit text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit text to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Edit text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Edit text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/edit-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.move-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.move-text.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.move-text
  feature_name: Move text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move text to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Move text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Move text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/move-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.resize-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.resize-text.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.resize-text
  feature_name: Resize text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resize text to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Resize text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Resize text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/resize-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.rotate-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.rotate-text.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.rotate-text
  feature_name: Rotate text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate text to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Rotate text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Rotate text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/rotate-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.setup-paragraph-formatting.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.setup-paragraph-formatting.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.setup-paragraph-formatting
  feature_name: Set up paragraph formatting
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set up paragraph formatting to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Set up paragraph formatting" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Set up paragraph formatting
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/setup-paragraph-formatting.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.get-started-with-text.update-cjk-text-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.get-started-with-text.update-cjk-text-layers.v0
  source_feature_id: photoshop.leaf.text-typography.get-started-with-text.update-cjk-text-layers
  feature_name: Update text layers for vector-based output
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: get-started-with-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Update text layers for vector-based output to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop
    workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Update text layers for vector-based output" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Update text layers for vector-based output
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/get-started-with-text/update-cjk-text-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.international-text-languages.create-documents-using-international-languages-scripts-and-type.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.international-text-languages.create-documents-using-international-languages-scripts-and-type.v0
  source_feature_id: photoshop.leaf.text-typography.international-text-languages.create-documents-using-international-languages-scripts-and-type
  feature_name: Create documents using international languages, scripts, and text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: international-text-languages
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create documents using international languages, scripts, and text to create, edit, style, compose, or validate text and typographic behavior in
    Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Create documents using international languages, scripts, and text" without needing hidden
    source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Create documents using international languages, scripts, and text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/international-text-languages/create-documents-using-international-languages-scripts-and-type.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.international-text-languages.overview-of-unified-text-engine.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.international-text-languages.overview-of-unified-text-engine.v0
  source_feature_id: photoshop.leaf.text-typography.international-text-languages.overview-of-unified-text-engine
  feature_name: Overview of Unified Text Engine
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: international-text-languages
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of Unified Text Engine to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of Unified Text Engine" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Overview of Unified Text Engine
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/international-text-languages/overview-of-unified-text-engine.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.about-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.about-fonts.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.about-fonts
  feature_name: About fonts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About fonts to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "About fonts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / About fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/about-fonts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.apply-opentype-features.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.apply-opentype-features.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.apply-opentype-features
  feature_name: Apply OpenType features
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply OpenType features to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Apply OpenType features" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Apply OpenType features
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/apply-opentype-features.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.change-the-font-on-multiple-layers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.change-the-font-on-multiple-layers.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.change-the-font-on-multiple-layers
  feature_name: Change the font across multiple layers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change the font across multiple layers to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Change the font across multiple layers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Change the font across multiple layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/change-the-font-on-multiple-layers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.match-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.match-fonts.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.match-fonts
  feature_name: Match fonts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Match fonts to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Match fonts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Match fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/match-fonts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.overview-of-opentype-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.overview-of-opentype-fonts.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.overview-of-opentype-fonts
  feature_name: Overview of OpenType fonts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of OpenType fonts to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of OpenType fonts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Overview of OpenType fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/overview-of-opentype-fonts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.replace-missing-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.replace-missing-fonts.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.replace-missing-fonts
  feature_name: Replace missing fonts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace missing fonts to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Replace missing fonts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Replace missing fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/replace-missing-fonts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.search-for-and-apply-a-specific-font-style.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.search-for-and-apply-a-specific-font-style.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.search-for-and-apply-a-specific-font-style
  feature_name: Search for and apply font styles
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Search for and apply font styles to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Search for and apply font styles" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Search for and apply font styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/search-for-and-apply-a-specific-font-style.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.select-manage-fonts.use-opentype-variable-fonts.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.select-manage-fonts.use-opentype-variable-fonts.v0
  source_feature_id: photoshop.leaf.text-typography.select-manage-fonts.use-opentype-variable-fonts
  feature_name: Use OpenType variable fonts
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: select-manage-fonts
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use OpenType variable fonts to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Use OpenType variable fonts" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Use OpenType variable fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/select-manage-fonts/use-opentype-variable-fonts.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.add-drop-shadows-to-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.add-drop-shadows-to-text.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.add-drop-shadows-to-text
  feature_name: Add drop shadows to text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add drop shadows to text to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Add drop shadows to text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add drop shadows to text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/add-drop-shadows-to-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.add-text-along-paths-or-inside-shapes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.add-text-along-paths-or-inside-shapes.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.add-text-along-paths-or-inside-shapes
  feature_name: Add text along paths or inside shapes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add text along paths or inside shapes to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Add text along paths or inside shapes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add text along paths or inside shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/add-text-along-paths-or-inside-shapes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.convert-text-to-shapes-or-work-paths.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.convert-text-to-shapes-or-work-paths.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.convert-text-to-shapes-or-work-paths
  feature_name: Convert text to shapes or work paths
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert text to shapes or work paths to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Convert text to shapes or work paths" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Convert text to shapes or work paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/convert-text-to-shapes-or-work-paths.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.create-text-selection-borders.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.create-text-selection-borders.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.create-text-selection-borders
  feature_name: Create text selection borders
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create text selection borders to define an editable target region that later tools can consume without ambiguity.
  user_goal: A Studio operator can perform the source-app workflow named "Create text selection borders" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Create text selection borders
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/create-text-selection-borders.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.flip-or-move-text-along-a-path.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.flip-or-move-text-along-a-path.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.flip-or-move-text-along-a-path
  feature_name: Flip or move text along a path
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Flip or move text along a path to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Flip or move text along a path" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Flip or move text along a path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/flip-or-move-text-along-a-path.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.modify-text-paths.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.modify-text-paths.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.modify-text-paths
  feature_name: Modify text paths
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify text paths to author or transform resolution-independent geometry for Studio documents.
  user_goal: A Studio operator can perform the source-app workflow named "Modify text paths" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Modify text paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/modify-text-paths.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.text-on-paths-shapes.warp-and-unwarp-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.text-on-paths-shapes.warp-and-unwarp-text.v0
  source_feature_id: photoshop.leaf.text-typography.text-on-paths-shapes.warp-and-unwarp-text
  feature_name: Warp and unwarp text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: text-on-paths-shapes
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Warp and unwarp text to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Warp and unwarp text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Warp and unwarp text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/text-on-paths-shapes/warp-and-unwarp-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.type-layers-creation.fill-text-with-image.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.type-layers-creation.fill-text-with-image.v0
  source_feature_id: photoshop.leaf.text-typography.type-layers-creation.fill-text-with-image
  feature_name: Fill text with image
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: type-layers-creation
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioLayerGraph
  primitive_domain: layer
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fill text with image to create, arrange, combine, or non-destructively control visual layer state imported from Photoshop workflows.
  user_goal: A Studio operator can perform the source-app workflow named "Fill text with image" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioLayerGraph / Fill text with image
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - studio.layer_graph.create_layer.v0
  verification_refs:
  - needs_fixture.layer.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/type-layers-creation/fill-text-with-image.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.work-with-dynamic-text.adjust-formatting-and-resize-text-with-dynamic-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.work-with-dynamic-text.adjust-formatting-and-resize-text-with-dynamic-text.v0
  source_feature_id: photoshop.leaf.text-typography.work-with-dynamic-text.adjust-formatting-and-resize-text-with-dynamic-text
  feature_name: Adjust formatting and resize text
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: work-with-dynamic-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust formatting and resize text to define interactive, form, animation, or media behavior for documents that support runtime output.
  user_goal: A Studio operator can perform the source-app workflow named "Adjust formatting and resize text" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Adjust formatting and resize text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/work-with-dynamic-text/adjust-formatting-and-resize-text-with-dynamic-text.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.work-with-dynamic-text.dynamic-text-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.work-with-dynamic-text.dynamic-text-overview.v0
  source_feature_id: photoshop.leaf.text-typography.work-with-dynamic-text.dynamic-text-overview
  feature_name: Dynamic Text Overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: work-with-dynamic-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Dynamic Text Overview to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Dynamic Text Overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Dynamic Text Overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/work-with-dynamic-text/dynamic-text-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.text-typography.work-with-dynamic-text.reposition-start-and-end-points.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.text-typography.work-with-dynamic-text.reposition-start-and-end-points.v0
  source_feature_id: photoshop.leaf.text-typography.work-with-dynamic-text.reposition-start-and-end-points
  feature_name: Reposition start and end points
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: text-typography
  source_subcategory: work-with-dynamic-text
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reposition start and end points to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Reposition start and end points" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Reposition start and end points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/text-typography/work-with-dynamic-text/reposition-start-and-end-points.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.create-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.create-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.create-guides
  feature_name: Create guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create guides to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Create guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Create guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/create-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.edit-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.edit-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.edit-guides
  feature_name: Edit guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit guides to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Edit guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Edit guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/edit-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.move-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.move-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.move-guides
  feature_name: Move guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move guides to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Move guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Move guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/move-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.navigation-and-measuring-tools-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.navigation-and-measuring-tools-overview.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.navigation-and-measuring-tools-overview
  feature_name: Overview of navigation and measuring tools
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of navigation and measuring tools to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of navigation and measuring tools" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Overview of navigation and measuring tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/navigation-and-measuring-tools-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.overview-of-guides-grids-and-smart-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.overview-of-guides-grids-and-smart-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.overview-of-guides-grids-and-smart-guides
  feature_name: Overview of guides, grids, and smart guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of guides, grids, and smart guides to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of guides, grids, and smart guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Overview of guides, grids, and smart guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/overview-of-guides-grids-and-smart-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.remove-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.remove-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.remove-guides
  feature_name: Remove guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioRasterPipeline
  primitive_domain: raster
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove guides to modify pixel content or raster-derived appearance through a Studio command that can be previewed and audited.
  user_goal: A Studio operator can perform the source-app workflow named "Remove guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioRasterPipeline / Remove guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.raster.v0
  verification_refs:
  - needs_fixture.raster.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/remove-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.set-guide-and-grid-preferences.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.set-guide-and-grid-preferences.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.set-guide-and-grid-preferences
  feature_name: Set guide and grid preferences
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set guide and grid preferences to create, edit, style, compose, or validate text and typographic behavior in Studio.
  user_goal: A Studio operator can perform the source-app workflow named "Set guide and grid preferences" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Set guide and grid preferences
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/set-guide-and-grid-preferences.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.show-or-hide-guides-grids-and-smart-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.show-or-hide-guides-grids-and-smart-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.show-or-hide-guides-grids-and-smart-guides
  feature_name: Show or hide guides, grids, and smart guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Show or hide guides, grids, and smart guides to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Show or hide guides, grids, and smart guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Show or hide guides, grids, and smart guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/show-or-hide-guides-grids-and-smart-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.work-efficiently-with-smart-guides.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.work-efficiently-with-smart-guides.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.alignment-grids-guides.work-efficiently-with-smart-guides
  feature_name: Work efficiently with Smart Guides
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: alignment-grids-guides
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work efficiently with Smart Guides to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Work efficiently with Smart Guides" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Work efficiently with Smart Guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/alignment-grids-guides/work-efficiently-with-smart-guides.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.create-edit-and-delete-data-point-presets.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.create-edit-and-delete-data-point-presets.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.create-edit-and-delete-data-point-presets
  feature_name: Create, edit, and delete data point presets
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create, edit, and delete data point presets to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Create, edit, and delete data point presets" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Create, edit, and delete data point presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/create-edit-and-delete-data-point-presets.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-measurement-logs.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-measurement-logs.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-measurement-logs
  feature_name: Manage measurement logs
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage measurement logs to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Manage measurement logs" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Manage measurement logs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/manage-measurement-logs.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-measurement-scales.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-measurement-scales.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-measurement-scales
  feature_name: Manage measurement scales
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage measurement scales to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Manage measurement scales" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Manage measurement scales
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/manage-measurement-scales.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-scale-markers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-scale-markers.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.manage-scale-markers
  feature_name: Manage scale markers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage scale markers to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Manage scale markers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Manage scale markers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/manage-scale-markers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-data-points-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-data-points-overview.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-data-points-overview
  feature_name: Measurement data points overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Measurement data points overview to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Measurement data points overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Measurement data points overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/measurement-data-points-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-log-for-measurements.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-log-for-measurements.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-log-for-measurements
  feature_name: Use the measurement log for performing measurements
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use the measurement log for performing measurements to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "Use the measurement log for performing measurements" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Use the measurement log for performing measurements
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/measurement-log-for-measurements.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-scale-and-scale-markers.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-scale-and-scale-markers.v0
  source_feature_id: photoshop.leaf.use-grids-measurement-guides.measure-scale.measurement-scale-and-scale-markers
  feature_name: About the measurement scale and scale markers
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: use-grids-measurement-guides
  source_subcategory: measure-scale
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About the measurement scale and scale markers to assemble pages, spreads, frames, guides, or repeated layout structures.
  user_goal: A Studio operator can perform the source-app workflow named "About the measurement scale and scale markers" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / About the measurement scale and scale markers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/use-grids-measurement-guides/measure-scale/measurement-scale-and-scale-markers.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.ai-assistant-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.ai-assistant-overview.v0
  source_feature_id: photoshop.leaf.whats-new.ai-assistant-overview
  feature_name: AI Assistant overview
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use AI Assistant overview to expose model-assisted behavior as an explicit optional provider-backed Studio command.
  user_goal: A Studio operator can perform the source-app workflow named "AI Assistant overview" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / AI Assistant overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/ai-assistant-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.enable-and-use-technology-previews.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.enable-and-use-technology-previews.v0
  source_feature_id: photoshop.leaf.whats-new.enable-and-use-technology-previews
  feature_name: Use technology previews
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use technology previews to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Use technology previews" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Use technology previews
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/enable-and-use-technology-previews.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.list-of-technology-preview-features.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.list-of-technology-preview-features.v0
  source_feature_id: photoshop.leaf.whats-new.list-of-technology-preview-features
  feature_name: List of technology preview features
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: optional_integration
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use List of technology preview features to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "List of technology preview features" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / List of technology preview features
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/list-of-technology-preview-features.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.photoshop-desktop-beta-overview.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.photoshop-desktop-beta-overview.v0
  source_feature_id: photoshop.leaf.whats-new.photoshop-desktop-beta-overview
  feature_name: Overview of Adobe Photoshop (beta) on desktop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Overview of Adobe Photoshop (beta) on desktop to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Overview of Adobe Photoshop (beta) on desktop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Overview of Adobe Photoshop (beta) on desktop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/photoshop-desktop-beta-overview.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.photoshop-on-desktop-release-notes.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.photoshop-on-desktop-release-notes.v0
  source_feature_id: photoshop.leaf.whats-new.photoshop-on-desktop-release-notes
  feature_name: Adobe Photoshop on desktop release notes
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adobe Photoshop on desktop release notes to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "Adobe Photoshop on desktop release notes" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / Adobe Photoshop on desktop release notes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/photoshop-on-desktop-release-notes.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.whats-new-in-adobe-photoshop-on-desktop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.whats-new-in-adobe-photoshop-on-desktop.v0
  source_feature_id: photoshop.leaf.whats-new.whats-new-in-adobe-photoshop-on-desktop
  feature_name: What's new in Adobe Photoshop on desktop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use What's new in Adobe Photoshop on desktop to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "What's new in Adobe Photoshop on desktop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / What's new in Adobe Photoshop on desktop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/whats-new-in-adobe-photoshop-on-desktop.html
- source_distilled_feature_id: osd.photoshop.photoshop.leaf.whats-new.whats-new-in-photoshop-beta-on-desktop.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.photoshop.leaf.whats-new.whats-new-in-photoshop-beta-on-desktop.v0
  source_feature_id: photoshop.leaf.whats-new.whats-new-in-photoshop-beta-on-desktop
  feature_name: What's new in Adobe Photoshop (Beta) on desktop
  source_apps:
  - Photoshop
  source_inventory: SFR-PHOTOSHOP-LEAF-INDEX
  source_category: whats-new
  source_subcategory: unknown
  source_domain_ledger: 34-photoshop-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioExportRecipe
  primitive_domain: export
  provider_posture: local_primitive_candidate
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use What's new in Adobe Photoshop (Beta) on desktop to produce, package, print, or hand off Studio output with reproducible export settings.
  user_goal: A Studio operator can perform the source-app workflow named "What's new in Adobe Photoshop (Beta) on desktop" without needing hidden source-app context.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioExportRecipe / What's new in Adobe Photoshop (Beta) on desktop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.export.v0
  verification_refs:
  - needs_fixture.export.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: SFR-PHOTOSHOP-LEAF-INDEX
    path: 06-photoshop-leaf-index.md
  - label: official_help_entry_or_index
    url: https://helpx.adobe.com/photoshop/desktop/whats-new/whats-new-in-photoshop-beta-on-desktop.html
```

</topic>

<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for this generated row ledger.">

### [SFR-PHOTOSHOP-SOURCE-DISTILLED-FEATURE-ROWS.sources] Sources

```yaml
sources:
- id: ROWS-S01
  path: 15-photoshop-feature-use-cards.md
  note: Generated Feature Use Cards used as row source.
- id: ROWS-S02
  path: 34-photoshop-source-distilled-domain-ledger.md
  note: Online-source-distilled domain ledger used as row context.
- id: ROWS-S03
  path: 33-online-source-distilled-feature-ledger.md
  note: Canonical source-distilled merge record.
```

</topic>
