---
file_id: 41-illustrator-source-distilled-feature-rows
file_kind: source_distilled_feature_rows
topic_id: SFR-ILLUSTRATOR-SOURCE-DISTILLED-FEATURE-ROWS
title: Illustrator Source Distilled Feature Rows
status: draft
updated_at: '2026-07-05'
app_key: illustrator
source_cards: 24-illustrator-feature-use-cards.md
source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
feature_row_count: 515
source_ref_count: 515
---

## [SFR-ILLUSTRATOR-SOURCE-DISTILLED-FEATURE-ROWS] Illustrator Source Distilled Feature Rows

<topic id="feature-row-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Coverage and source policy for generated source-distilled feature rows.">

### [SFR-ILLUSTRATOR-SOURCE-DISTILLED-FEATURE-ROWS.coverage] Feature Row Coverage

```yaml
coverage:
  app_key: illustrator
  source_cards: 24-illustrator-feature-use-cards.md
  source_inventory: 22-illustrator-leaf-index.md
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_row_count: 515
  distillation_status: online_source_distilled_feature_rows
  installed_exports_role: optional_enrichment_only
  naming_rule: Vendor product names remain source/provenance and compatibility references only.
  manual_handoff_rule: Promote manual_topic_candidate into the internal Studio UserManual in the same change that implements
    the feature behavior.
```

</topic>

<topic id="source-distilled-feature-rows" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable source-distilled feature rows.">

### [SFR-ILLUSTRATOR-SOURCE-DISTILLED-FEATURE-ROWS.rows] Source Distilled Feature Rows

```yaml
source_distilled_feature_rows:
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-whats-new-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-whats-new-html.v0
  source_feature_id: illustrator.desktop.leaf.using-whats-new-html
  feature_name: What's New
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use What's New to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named What's New with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / What's New
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/using/whats-new.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-new-features-whats-new-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-new-features-whats-new-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-new-features-whats-new-html
  feature_name: What's new in Adobe Illustrator on desktop
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: new_features
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use What's new in Adobe Illustrator on desktop to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named What's new in Adobe Illustrator on desktop with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / What's new in Adobe Illustrator on desktop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/new-features/whats-new.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-new-features-release-notes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-new-features-release-notes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-new-features-release-notes-html
  feature_name: Adobe Illustrator on desktop release notes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: new_features
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adobe Illustrator on desktop release notes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Adobe Illustrator on desktop release notes with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adobe Illustrator on desktop release notes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/new-features/release-notes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-new-features-illustrator-beta-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-new-features-illustrator-beta-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-new-features-illustrator-beta-overview-html
  feature_name: Adobe Illustrator on desktop (Beta) overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: new_features
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adobe Illustrator on desktop (Beta) overview to make workspace, preference, navigation, and diagnostic behavior predictable for operators and
    models.
  user_goal: A Studio operator can perform the source workflow named Adobe Illustrator on desktop (Beta) overview with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Adobe Illustrator on desktop (Beta) overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/new-features/illustrator-beta-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-new-features-performance-enhancements-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-new-features-performance-enhancements-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-new-features-performance-enhancements-html
  feature_name: Performance enhancements in Adobe Illustrator on desktop
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: new_features
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Performance enhancements in Adobe Illustrator on desktop to create, edit, transform, or inspect vector geometry in Studio without relying on a
    vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Performance enhancements in Adobe Illustrator on desktop with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Performance enhancements in Adobe Illustrator on desktop
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/new-features/performance-enhancements.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-supported-file-formats-html
  feature_name: Supported file formats
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Supported file formats to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Supported file formats with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Supported file formats
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/supported-file-formats.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-homescreen-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-homescreen-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-homescreen-overview-html
  feature_name: Homescreen overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Homescreen overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Homescreen overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Homescreen overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/homescreen-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-show-or-hide-the-homescreen-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-show-or-hide-the-homescreen-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-show-or-hide-the-homescreen-html
  feature_name: Show or hide the homescreen
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Show or hide the homescreen to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Show or hide the homescreen with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Show or hide the homescreen
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/show-or-hide-the-homescreen.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-workspace-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-workspace-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-workspace-overview-html
  feature_name: Workspace overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Workspace overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Workspace overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Workspace overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/workspace-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-modify-workspaces-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-modify-workspaces-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-modify-workspaces-html
  feature_name: Modify workspaces
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify workspaces to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Modify workspaces with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Modify workspaces
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/modify-workspaces.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-manage-workspaces-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-manage-workspaces-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-manage-workspaces-html
  feature_name: Manage workspaces
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage workspaces to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Manage workspaces with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Manage workspaces
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/manage-workspaces.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-switch-between-the-workspace-and-homescreen-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-switch-between-the-workspace-and-homescreen-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-switch-between-the-workspace-and-homescreen-html
  feature_name: Switch between the workspace and the homescreen
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Switch between the workspace and the homescreen to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Switch between the workspace and the homescreen with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Switch between the workspace and the homescreen
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/switch-between-the-workspace-and-homescreen.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-properties-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-properties-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-properties-panel-overview-html
  feature_name: Properties panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Properties panel overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Properties panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Properties panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/properties-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-enter-values-in-panels-and-dialog-boxes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-enter-values-in-panels-and-dialog-boxes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-enter-values-in-panels-and-dialog-boxes-html
  feature_name: Set properties with precise values
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set properties with precise values to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Set properties with precise values with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Set properties with precise values
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/enter-values-in-panels-and-dialog-boxes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-control-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-control-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-control-panel-overview-html
  feature_name: Control panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Control panel overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Control panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Control panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/control-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-contextual-task-bar-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-contextual-task-bar-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-contextual-task-bar-overview-html
  feature_name: Contextual Task Bar overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Contextual Task Bar overview to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Contextual Task Bar overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Contextual Task Bar overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/contextual-task-bar-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-discover-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-discover-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-discover-panel-overview-html
  feature_name: Discover panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Discover panel overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Discover panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Discover panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/discover-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-learn-with-discover-panel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-get-started-learn-the-basics-learn-with-discover-panel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-get-started-learn-the-basics-learn-with-discover-panel-html
  feature_name: Learn faster with the Discover panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: get_started
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Learn faster with the Discover panel to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Learn faster with the Discover panel with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Learn faster with the Discover panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/get-started/learn-the-basics/learn-with-discover-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-new-document-dialog-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-new-document-dialog-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-new-document-dialog-overview-html
  feature_name: New Document dialog overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use New Document dialog overview to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named New Document dialog overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / New Document dialog overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/new-document-dialog-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-presets-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-presets-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-presets-html
  feature_name: Create documents using presets
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create documents using presets to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create documents using presets with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create documents using presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/create-documents-using-presets.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-and-save-custom-document-presets-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-and-save-custom-document-presets-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-and-save-custom-document-presets-html
  feature_name: Create and save custom document presets
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create and save custom document presets to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create and save custom document presets with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create and save custom document presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/create-and-save-custom-document-presets.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-blank-templates-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-blank-templates-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-blank-templates-html
  feature_name: Create documents using blank templates
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create documents using blank templates to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create documents using blank templates with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create documents using blank templates
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/create-documents-using-blank-templates.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-files-on-large-canvases-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-files-on-large-canvases-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-files-on-large-canvases-html
  feature_name: Create files with large canvases
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create files with large canvases to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create files with large canvases with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create files with large canvases
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/create-files-on-large-canvases.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-templates-from-adobe.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-templates-from-adobe.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-create-documents-using-templates-from-adobe
  feature_name: Create documents using templates from Adobe Stock
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create documents using templates from Adobe Stock to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create documents using templates from Adobe Stock with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create documents using templates from Adobe Stock
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/create-documents-using-templates-from-adobe-stock.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-rotate-canvas-view-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-rotate-canvas-view-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-rotate-canvas-view-html
  feature_name: Rotate canvas view
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate canvas view to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Rotate canvas view with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Rotate canvas view
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/rotate-canvas-view.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-organize-share-and-collaborate-using-project.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-organize-share-and-collaborate-using-project.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-organize-share-and-collaborate-using-project
  feature_name: Organize, share, and collaborate using Projects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: local_first_collaboration_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Organize, share, and collaborate using Projects to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Organize, share, and collaborate using Projects with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Organize, share, and collaborate using Projects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/organize-share-and-collaborate-using-projects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-access-projects-in-the-illustrator-workspace.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-access-projects-in-the-illustrator-workspace.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-access-projects-in-the-illustrator-workspace
  feature_name: Access projects in the Illustrator workspace and other apps
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Access projects in the Illustrator workspace and other apps to preserve compatibility with existing creative file and asset workflows through
    explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Access projects in the Illustrator workspace and other apps with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Access projects in the Illustrator workspace and other apps
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/access-projects-in-the-illustrator-workspace-and-other-apps.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-find-and-edit-adobe-express-templates-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-find-and-edit-adobe-express-templates-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-start-a-new-file-find-and-edit-adobe-express-templates-html
  feature_name: Find and edit Adobe Express templates
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find and edit Adobe Express templates to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Find and edit Adobe Express templates with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Find and edit Adobe Express templates
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/start-a-new-file/find-and-edit-adobe-express-templates.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-linked-photoshop-documents-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-linked-photoshop-documents-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-linked-photoshop-documents-html
  feature_name: Place linked Photoshop documents
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Place linked Photoshop documents to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Place linked Photoshop documents with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Place linked Photoshop documents
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-from-other-apps/place-linked-photoshop-documents.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-paths-from-photoshop-to-illustrat.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-paths-from-photoshop-to-illustrat.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-paths-from-photoshop-to-illustrat
  feature_name: Move paths from Photoshop to Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move paths from Photoshop to Illustrator to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Move paths from Photoshop to Illustrator with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Move paths from Photoshop to Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-from-other-apps/move-paths-from-photoshop-to-illustrator.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-part-of-an-image-from-photoshop-t.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-part-of-an-image-from-photoshop-t.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-move-part-of-an-image-from-photoshop-t
  feature_name: Move part of an image from Photoshop to Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move part of an image from Photoshop to Illustrator to preserve compatibility with existing creative file and asset workflows through explicit
    import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Move part of an image from Photoshop to Illustrator with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Move part of an image from Photoshop to Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-from-other-apps/move-part-of-an-image-from-photoshop-to-illustrator.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-photoshop-import-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-photoshop-import-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-photoshop-import-options-html
  feature_name: Photoshop import options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Photoshop import options to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Photoshop import options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Photoshop import options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-from-other-apps/photoshop-import-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-and-edit-adobe-firefly-output-in.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-and-edit-adobe-firefly-output-in.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-from-other-apps-place-and-edit-adobe-firefly-output-in
  feature_name: Place and edit images generated on the Adobe Firefly website
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Place and edit images generated on the Adobe Firefly website as a provider-neutral or local-model-assisted Studio workflow with explicit receipts
    and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Place and edit images generated on the Adobe Firefly website with Handshake-native commands,
    local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Place and edit images generated on the Adobe Firefly website
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-from-other-apps/place-and-edit-adobe-firefly-output-in-app.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-adobe-pdf-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-adobe-pdf-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-adobe-pdf-files-html
  feature_name: Import Adobe PDF files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import Adobe PDF files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Import Adobe PDF files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Import Adobe PDF files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-other-file-types/import-adobe-pdf-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-adobe-pdf-placement-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-adobe-pdf-placement-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-adobe-pdf-placement-options-html
  feature_name: Place Adobe PDF files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Place Adobe PDF files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Place Adobe PDF files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Place Adobe PDF files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-other-file-types/adobe-pdf-placement-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-autocad-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-autocad-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-autocad-files-html
  feature_name: Import AutoCAD files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import AutoCAD files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Import AutoCAD files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Import AutoCAD files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-other-file-types/import-autocad-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-monotone-duotone-and-tritone-i.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-monotone-duotone-and-tritone-i.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-monotone-duotone-and-tritone-i
  feature_name: Import monotone, duotone, and tritone images from Adobe PDF files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import monotone, duotone, and tritone images from Adobe PDF files to preserve compatibility with existing creative file and asset workflows through
    explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Import monotone, duotone, and tritone images from Adobe PDF files with Handshake-native commands,
    local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Import monotone, duotone, and tritone images from Adobe PDF files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-other-file-types/import-monotone-duotone-and-tritone-images-from-adobe-pdf-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-dcs-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-dcs-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-import-other-file-types-import-dcs-files-html
  feature_name: Import DCS files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import DCS files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Import DCS files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Import DCS files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/import-other-file-types/import-dcs-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-project-files-upload-download-project-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-manage-project-files-upload-download-project-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-manage-project-files-upload-download-project-files-html
  feature_name: Upload and download project files in Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Upload and download project files in Illustrator to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Upload and download project files in Illustrator with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Upload and download project files in Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/manage-project-files/upload-download-project-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-links-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-links-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-links-panel-overview-html
  feature_name: Links panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Links panel overview to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Links panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Links panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/manage-linked-and-embedded-files/links-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-relink-replace-or-update-lin.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-relink-replace-or-update-lin.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-relink-replace-or-update-lin
  feature_name: Relink, replace, or update linked files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Relink, replace, or update linked files to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Relink, replace, or update linked files with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Relink, replace, or update linked files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/manage-linked-and-embedded-files/relink-replace-or-update-linked-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-embed-images-and-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-embed-images-and-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-add-and-import-files-manage-linked-and-embedded-files-embed-images-and-files-html
  feature_name: Embed and unembed images and files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: add_and_import_files
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Embed and unembed images and files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Embed and unembed images and files with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Embed and unembed images and files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/add-and-import-files/manage-linked-and-embedded-files/embed-images-and-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generative-ai-faq-illustrator-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generative-ai-faq-illustrator-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generative-ai-faq-illustrator-html
  feature_name: Common questions about generative AI features in Adobe Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Common questions about generative AI features in Adobe Illustrator as a provider-neutral or local-model-assisted Studio workflow with explicit
    receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Common questions about generative AI features in Adobe Illustrator with Handshake-native commands,
    local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Common questions about generative AI features in Adobe Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generative-ai-faq-illustrator.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-partner-models-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-partner-models-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-partner-models-overview-html
  feature_name: Partner models in Adobe Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Partner models in Adobe Illustrator as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Partner models in Adobe Illustrator with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Partner models in Adobe Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/partner-models-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-scenes-subjects-and-icons-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generate-scenes-subjects-and-icons-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generate-scenes-subjects-and-icons-html
  feature_name: Generate scenes, subjects, and icons
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate scenes, subjects, and icons as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Generate scenes, subjects, and icons with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate scenes, subjects, and icons
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generate-scenes-subjects-and-icons.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-use-auto-select-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-use-auto-select-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-use-auto-select-html
  feature_name: Use Auto Select
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Auto Select as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Use Auto Select with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Use Auto Select
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/use-auto-select.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-similar-variations-without-text-prompts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generate-similar-variations-without-text-prompts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generate-similar-variations-without-text-prompts-html
  feature_name: Generate similar variations without text prompts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate similar variations without text prompts as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud
    dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Generate similar variations without text prompts with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate similar variations without text prompts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generate-similar-variations-without-text-prompts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-patterns-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generate-patterns-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generate-patterns-html
  feature_name: Generate patterns
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate patterns as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Generate patterns with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate patterns
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generate-patterns.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-manage-pattern-variations-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-manage-pattern-variations-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-manage-pattern-variations-html
  feature_name: Manage pattern variations
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage pattern variations as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Manage pattern variations with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Manage pattern variations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/manage-pattern-variations.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-edit-generated-patterns-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-edit-generated-patterns-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-edit-generated-patterns-html
  feature_name: Edit generated patterns
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit generated patterns as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Edit generated patterns with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Edit generated patterns
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/edit-generated-patterns.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-shape-fills-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generate-shape-fills-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generate-shape-fills-html
  feature_name: Generate shape fills
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate shape fills as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Generate shape fills with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate shape fills
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generate-shape-fills.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-scenarios-with-repeat-shape-fill-generation-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-scenarios-with-repeat-shape-fill-generation-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-scenarios-with-repeat-shape-fill-generation-html
  feature_name: Scenarios with repeat shape fill generation
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scenarios with repeat shape fill generation as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud
    dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Scenarios with repeat shape fill generation with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Scenarios with repeat shape fill generation
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/scenarios-with-repeat-shape-fill-generation.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-recolor-artwork-with-generative-recolor-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-recolor-artwork-with-generative-recolor-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-recolor-artwork-with-generative-recolor-html
  feature_name: Recolor artwork with text prompts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Recolor artwork with text prompts as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Recolor artwork with text prompts with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Recolor artwork with text prompts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/recolor-artwork-with-generative-recolor.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-expand-artwork-with-generative-expand-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-expand-artwork-with-generative-expand-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-expand-artwork-with-generative-expand-html
  feature_name: Generate vector graphics to expand artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate vector graphics to expand artwork as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Generate vector graphics to expand artwork with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate vector graphics to expand artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/expand-artwork-with-generative-expand.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-options-to-expand-the-expanded-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-options-to-expand-the-expanded-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-options-to-expand-the-expanded-artwork-html
  feature_name: Options to expand the expanded artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Options to expand the expanded artwork as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Options to expand the expanded artwork with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Options to expand the expanded artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/options-to-expand-the-expanded-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-print-bleed-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generate-print-bleed-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generate-print-bleed-html
  feature_name: Generate print bleed
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate print bleed as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Generate print bleed with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate print bleed
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generate-print-bleed.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-expand-images-with-generative-expand-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-expand-images-with-generative-expand-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-expand-images-with-generative-expand-html
  feature_name: Generate content to expand images
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate content to expand images as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Generate content to expand images with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate content to expand images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/expand-images-with-generative-expand.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-view-artwork-from-any-angle-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-view-artwork-from-any-angle-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-view-artwork-from-any-angle-html
  feature_name: View 2D objects from new angles
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View 2D objects from new angles as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in
    the core.
  user_goal: A Studio operator can perform the source workflow named View 2D objects from new angles with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / View 2D objects from new angles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/view-artwork-from-any-angle.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-remove-background-from-images-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-remove-background-from-images-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-remove-background-from-images-html
  feature_name: Remove background from images
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove background from images as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in
    the core.
  user_goal: A Studio operator can perform the source workflow named Remove background from images with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Remove background from images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/remove-background-from-images.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-turntable-and-3d-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-turntable-and-3d-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-turntable-and-3d-html
  feature_name: When to use Turntable and 3D
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use When to use Turntable and 3D as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named When to use Turntable and 3D with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / When to use Turntable and 3D
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/turntable-and-3d.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-generate-vector-artwork-from-images-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-generate-vector-artwork-from-images-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-generate-vector-artwork-from-images-html
  feature_name: Generate vector artwork from raster images
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate vector artwork from raster images as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Generate vector artwork from raster images with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate vector artwork from raster images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/generate-vector-artwork-from-images.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-manage-generated-variations-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-manage-generated-variations-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-manage-generated-variations-html
  feature_name: Manage generated variations
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage generated variations as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Manage generated variations with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Manage generated variations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/manage-generated-variations.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-scenarios-with-linked-variations-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-scenarios-with-linked-variations-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-scenarios-with-linked-variations-html
  feature_name: Scenarios with linked variations
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scenarios with linked variations as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Scenarios with linked variations with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Scenarios with linked variations
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/scenarios-with-linked-variations.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-edit-generated-artwork-using-prompts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-edit-generated-artwork-using-prompts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-edit-generated-artwork-using-prompts-html
  feature_name: Edit generated artwork using prompts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit generated artwork using prompts as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Edit generated artwork using prompts with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Edit generated artwork using prompts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/edit-generated-artwork-using-prompts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-generative-ai-proofread-translate-rephrase-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-generative-ai-proofread-translate-rephrase-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-generative-ai-proofread-translate-rephrase-text-html
  feature_name: Generate, rewrite, proofread, and translate text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: use_generative_ai
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Generate, rewrite, proofread, and translate text as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud
    dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Generate, rewrite, proofread, and translate text with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Generate, rewrite, proofread, and translate text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-generative-ai/proofread-translate-rephrase-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-drawing-modes-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-drawing-modes-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-drawing-modes-overview-html
  feature_name: Drawing modes overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Drawing modes overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Drawing modes overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Drawing modes overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/learn-drawing-basics/drawing-modes-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-paths-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-paths-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-paths-overview-html
  feature_name: Paths overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paths overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Paths overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Paths overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/learn-drawing-basics/paths-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-adjust-anchor-point-handle-and-bounding.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-adjust-anchor-point-handle-and-bounding.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-adjust-anchor-point-handle-and-bounding
  feature_name: Adjust anchor point, handle, and bounding box display size
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust anchor point, handle, and bounding box display size to create, edit, transform, or inspect vector geometry in Studio without relying on
    a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Adjust anchor point, handle, and bounding box display size with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adjust anchor point, handle, and bounding box display size
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/learn-drawing-basics/adjust-anchor-point-handle-and-bounding-box-display-size.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-set-direction-lines-and-points-appearan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-set-direction-lines-and-points-appearan.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-learn-drawing-basics-set-direction-lines-and-points-appearan
  feature_name: Show or hide direction lines for anchor points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Show or hide direction lines for anchor points to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Show or hide direction lines for anchor points with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Show or hide direction lines for anchor points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/learn-drawing-basics/set-direction-lines-and-points-appearance.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-lines-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-lines-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-lines-html
  feature_name: Draw lines
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw lines to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw lines with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw lines
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-lines.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-arcs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-arcs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-arcs-html
  feature_name: Draw arcs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw arcs to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw arcs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw arcs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-arcs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-stars-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-stars-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-stars-html
  feature_name: Draw stars
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw stars to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw stars with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw stars
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-stars.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-spirals-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-spirals-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-spirals-html
  feature_name: Draw spirals
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw spirals to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw spirals with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw spirals
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-spirals.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-freeform-paths-with-the-pencil-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-freeform-paths-with-the-pencil-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-freeform-paths-with-the-pencil-tool-html
  feature_name: Draw freeform paths with the Pencil tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw freeform paths with the Pencil tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Draw freeform paths with the Pencil tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw freeform paths with the Pencil tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-freeform-paths-with-the-pencil-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-straight-lines-with-the-pencil-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-straight-lines-with-the-pencil-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-straight-lines-with-the-pencil-tool-html
  feature_name: Draw straight lines with the Pencil tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw straight lines with the Pencil tool as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Draw straight lines with the Pencil tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Draw straight lines with the Pencil tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-straight-lines-with-the-pencil-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-extend-paths-with-the-pencil-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-extend-paths-with-the-pencil-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-extend-paths-with-the-pencil-tool-html
  feature_name: Extend paths with the Pencil tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Extend paths with the Pencil tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Extend paths with the Pencil tool with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Extend paths with the Pencil tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/extend-paths-with-the-pencil-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-reshape-paths-with-the-pencil-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-reshape-paths-with-the-pencil-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-reshape-paths-with-the-pencil-tool-html
  feature_name: Reshape paths with the Pencil tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reshape paths with the Pencil tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Reshape paths with the Pencil tool with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Reshape paths with the Pencil tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/reshape-paths-with-the-pencil-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-connect-two-paths-with-the-pencil-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-connect-two-paths-with-the-pencil-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-connect-two-paths-with-the-pencil-tool-html
  feature_name: Connect two paths with the Pencil tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Connect two paths with the Pencil tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Connect two paths with the Pencil tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Connect two paths with the Pencil tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/connect-two-paths-with-the-pencil-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-pencil-tool-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-pencil-tool-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-pencil-tool-options-html
  feature_name: Pencil tool options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Pencil tool options to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Pencil tool options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Pencil tool options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/pencil-tool-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-shapes-with-the-curvature-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-shapes-with-the-curvature-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-shapes-with-the-curvature-tool-html
  feature_name: Draw shapes with the Curvature tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw shapes with the Curvature tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw shapes with the Curvature tool with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw shapes with the Curvature tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-shapes-with-the-curvature-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-curves-with-the-pen-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-curves-with-the-pen-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-curves-with-the-pen-tool-html
  feature_name: Draw curves with the Pen tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw curves with the Pen tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw curves with the Pen tool with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw curves with the Pen tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-curves-with-the-pen-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-line-segments-with-the-pen-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-line-segments-with-the-pen-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-line-segments-with-the-pen-tool-html
  feature_name: Draw line segments with the Pen tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw line segments with the Pen tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw line segments with the Pen tool with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw line segments with the Pen tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-line-segments-with-the-pen-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-curves-followed-by-straight-lines-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-curves-followed-by-straight-lines-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-curves-followed-by-straight-lines-html
  feature_name: Draw curves followed by straight lines
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw curves followed by straight lines as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Draw curves followed by straight lines with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Draw curves followed by straight lines
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-curves-followed-by-straight-lines.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-straight-lines-followed-by-curves-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-straight-lines-followed-by-curves-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-straight-lines-followed-by-curves-html
  feature_name: Draw straight lines followed by curves
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw straight lines followed by curves as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Draw straight lines followed by curves with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Draw straight lines followed by curves
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-straight-lines-followed-by-curves.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-two-curved-segments-connected-by-a-corner-h.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-two-curved-segments-connected-by-a-corner-h.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-draw-two-curved-segments-connected-by-a-corner-h
  feature_name: Draw two curved segments connected by a corner
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw two curved segments connected by a corner to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw two curved segments connected by a corner with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw two curved segments connected by a corner
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/draw-two-curved-segments-connected-by-a-corner.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-preview-paths-drawn-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-preview-paths-drawn-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-draw-shapes-preview-paths-drawn-html
  feature_name: Preview the path
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Preview the path to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Preview the path with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Preview the path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/draw-shapes/preview-paths-drawn.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-move-live-shapes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-move-live-shapes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-move-live-shapes-html
  feature_name: Move live shapes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move live shapes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Move live shapes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move live shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/move-live-shapes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-rotate-live-shapes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-rotate-live-shapes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-rotate-live-shapes-html
  feature_name: Rotate live shapes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate live shapes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rotate live shapes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rotate live shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/rotate-live-shapes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-scale-live-shapes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-scale-live-shapes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-scale-live-shapes-html
  feature_name: Scale live shapes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale live shapes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Scale live shapes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Scale live shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/scale-live-shapes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-change-corner-radius-of-live-shapes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-change-corner-radius-of-live-shapes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-change-corner-radius-of-live-shapes-html
  feature_name: Change corner radius of live shapes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change corner radius of live shapes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Change corner radius of live shapes with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Change corner radius of live shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/change-corner-radius-of-live-shapes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-add-or-remove-sides-from-stars-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-add-or-remove-sides-from-stars-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-add-or-remove-sides-from-stars-html
  feature_name: Add or remove sides from stars
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add or remove sides from stars to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add or remove sides from stars with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add or remove sides from stars
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/add-or-remove-sides-from-stars.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-change-inner-and-outer-radius-of-stars-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-change-inner-and-outer-radius-of-stars-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-change-inner-and-outer-radius-of-stars-ht
  feature_name: Change inner and outer radii of stars
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change inner and outer radii of stars to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Change inner and outer radii of stars with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Change inner and outer radii of stars
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/change-inner-and-outer-radius-of-stars.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-add-or-remove-sides-from-polygons-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-add-or-remove-sides-from-polygons-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-add-or-remove-sides-from-polygons-html
  feature_name: Add or remove sides from polygons
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add or remove sides from polygons to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add or remove sides from polygons with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add or remove sides from polygons
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/add-or-remove-sides-from-polygons.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-create-pie-shapes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-create-pie-shapes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-live-shapes-create-pie-shapes-html
  feature_name: Create pie shapes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create pie shapes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Create pie shapes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create pie shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-live-shapes/create-pie-shapes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-create-smooth-paths-with-the-smooth-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-create-smooth-paths-with-the-smooth-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-create-smooth-paths-with-the-smooth-tool-html
  feature_name: Create smooth paths with the Smooth tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create smooth paths with the Smooth tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Create smooth paths with the Smooth tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create smooth paths with the Smooth tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/create-smooth-paths-with-the-smooth-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-refine-path-segments-with-the-smooth-slider-htm.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-refine-path-segments-with-the-smooth-slider-htm.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-refine-path-segments-with-the-smooth-slider-htm
  feature_name: Refine path segments with the Smooth slider
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioInteractiveDocumentSurface
  primitive_domain: interactive
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Refine path segments with the Smooth slider to define prototype, presentation, motion, animation, or runtime interaction behavior in Studio.
  user_goal: A Studio operator can perform the source workflow named Refine path segments with the Smooth slider with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioInteractiveDocumentSurface / Refine path segments with the Smooth slider
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.interactive.v0
  verification_refs:
  - needs_fixture.interactive.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/refine-path-segments-with-the-smooth-slider.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-select-path-segments-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-select-path-segments-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-select-path-segments-html
  feature_name: Select and edit path segments
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select and edit path segments to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Select and edit path segments with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Select and edit path segments
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/select-path-segments.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-select-anchor-points-in-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-select-anchor-points-in-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-select-anchor-points-in-paths-html
  feature_name: Select anchor points to modify paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select anchor points to modify paths to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Select anchor points to modify paths with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Select anchor points to modify paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/select-anchor-points-in-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-convert-anchor-points-on-a-path-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-convert-anchor-points-on-a-path-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-convert-anchor-points-on-a-path-html
  feature_name: Convert anchor points on a path
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert anchor points on a path to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Convert anchor points on a path with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Convert anchor points on a path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/convert-anchor-points-on-a-path.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-average-the-position-of-anchor-points-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-average-the-position-of-anchor-points-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-average-the-position-of-anchor-points-html
  feature_name: Average the position of the anchor points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Average the position of the anchor points to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Average the position of the anchor points with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Average the position of the anchor points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/average-the-position-of-anchor-points.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-add-or-remove-anchor-points-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-add-or-remove-anchor-points-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-add-or-remove-anchor-points-html
  feature_name: Add or remove anchor points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add or remove anchor points to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add or remove anchor points with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add or remove anchor points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/add-or-remove-anchor-points.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-turn-off-automatic-addition-or-deletion-of-anch.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-turn-off-automatic-addition-or-deletion-of-anch.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-turn-off-automatic-addition-or-deletion-of-anch
  feature_name: Turn off automatic addition or deletion of anchor points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Turn off automatic addition or deletion of anchor points to create, edit, transform, or inspect vector geometry in Studio without relying on a
    vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Turn off automatic addition or deletion of anchor points with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Turn off automatic addition or deletion of anchor points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/turn-off-automatic-addition-or-deletion-of-anchor-points.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-find-and-delete-stray-anchor-points-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-find-and-delete-stray-anchor-points-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-find-and-delete-stray-anchor-points-html
  feature_name: Find and delete stray anchor points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find and delete stray anchor points to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Find and delete stray anchor points with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Find and delete stray anchor points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/find-and-delete-stray-anchor-points.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-copy-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-copy-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-copy-paths-html
  feature_name: Copy paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Copy paths to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Copy paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Copy paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/copy-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-auto-simplify-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-auto-simplify-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-auto-simplify-paths-html
  feature_name: Auto simplify paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Auto simplify paths to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Auto simplify paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Auto simplify paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/auto-simplify-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-manually-simplify-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-manually-simplify-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-manually-simplify-paths-html
  feature_name: Manually simplify paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manually simplify paths to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Manually simplify paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Manually simplify paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/manually-simplify-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-simplify-paths-advanced-options-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-simplify-paths-advanced-options-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-simplify-paths-advanced-options-overview-html
  feature_name: Simplify paths advanced options overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Simplify paths advanced options overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Simplify paths advanced options overview with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Simplify paths advanced options overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/simplify-paths-advanced-options-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-simplify-path-benefits-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-simplify-path-benefits-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-simplify-path-benefits-html
  feature_name: Simplify path benefits
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Simplify path benefits to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Simplify path benefits with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Simplify path benefits
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/simplify-path-benefits.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-split-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-split-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-split-paths-html
  feature_name: Cut paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Cut paths to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Cut paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Cut paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/split-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-erase-paths-using-eraser-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-erase-paths-using-eraser-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-erase-paths-using-eraser-tool-html
  feature_name: Erase paths using the Eraser tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Erase paths using the Eraser tool to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Erase paths using the Eraser tool with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Erase paths using the Eraser tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/erase-paths-using-eraser-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-erase-parts-of-a-path-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-erase-parts-of-a-path-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-erase-parts-of-a-path-html
  feature_name: Erase parts of a path
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Erase parts of a path to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Erase parts of a path with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Erase parts of a path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/erase-parts-of-a-path.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-adjust-path-smoothness-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-adjust-path-smoothness-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-adjust-path-smoothness-html
  feature_name: Adjust path smoothness
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust path smoothness to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Adjust path smoothness with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adjust path smoothness
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/adjust-path-smoothness.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-refine-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-refine-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-draw-shapes-and-paths-modify-paths-refine-paths-html
  feature_name: Refine paths in Illustrator using Anchor Point and Smooth tools
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: draw_shapes_and_paths
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Refine paths in Illustrator using Anchor Point and Smooth tools to create, edit, transform, or inspect vector geometry in Studio without relying
    on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Refine paths in Illustrator using Anchor Point and Smooth tools with Handshake-native commands,
    local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Refine paths in Illustrator using Anchor Point and Smooth tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/draw-shapes-and-paths/modify-paths/refine-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-html
  feature_name: Select objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-by-characteristics-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-by-characteristics-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-by-characteristics-html
  feature_name: Select objects by characteristics
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects by characteristics as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select objects by characteristics with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select objects by characteristics
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-objects-by-characteristics.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-completely-inside-the-marquee-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-completely-inside-the-marquee-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-completely-inside-the-marquee-html
  feature_name: Select objects enclosed in a marquee
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects enclosed in a marquee as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select objects enclosed in a marquee with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select objects enclosed in a marquee
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-objects-completely-inside-the-marquee.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-object-groups-and-nested-object-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-object-groups-and-nested-object-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-object-groups-and-nested-object-groups-html
  feature_name: Select object groups and nested object groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select object groups and nested object groups as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select object groups and nested object groups with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select object groups and nested object groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-object-groups-and-nested-object-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-faces-and-edges-in-live-paint-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-faces-and-edges-in-live-paint-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-faces-and-edges-in-live-paint-groups-html
  feature_name: Select faces and edges in Live Paint groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select faces and edges in Live Paint groups as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud
    dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Select faces and edges in Live Paint groups with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Select faces and edges in Live Paint groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-faces-and-edges-in-live-paint-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-group-ungroup-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-group-ungroup-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-group-ungroup-objects-html
  feature_name: Group or ungroup objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Group or ungroup objects as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Group or ungroup objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Group or ungroup objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/group-ungroup-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-isolate-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-isolate-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-isolate-objects-html
  feature_name: Isolate objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Isolate objects as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Isolate objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Isolate objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/isolate-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-save-object-selections-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-save-object-selections-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-save-object-selections-html
  feature_name: Save object selections
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save object selections to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Save object selections with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Save object selections
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/save-object-selections.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-layers-nested-selection-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-layers-nested-selection-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-layers-nested-selection-html
  feature_name: Select objects using layers and nested selection
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects using layers and nested selection as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select objects using layers and nested selection with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select objects using layers and nested selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-objects-layers-nested-selection.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-magic-wand-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-magic-wand-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-objects-magic-wand-tool-html
  feature_name: Select objects using the Magic Wand tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects using the Magic Wand tool as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select objects using the Magic Wand tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select objects using the Magic Wand tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-objects-magic-wand-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-transform-objects-selection-tools-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-transform-objects-selection-tools-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-select-objects-select-transform-objects-selection-tools-html
  feature_name: Select objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/select-objects/select-transform-objects-selection-tools.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-objects-html
  feature_name: Move objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Move objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/move-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-multiple-objects-at-once-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-multiple-objects-at-once-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-multiple-objects-at-once-html
  feature_name: Move multiple objects at once
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move multiple objects at once to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Move multiple objects at once with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move multiple objects at once
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/move-multiple-objects-at-once.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-objects-by-specific-distances-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-objects-by-specific-distances-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-objects-by-specific-distances-html
  feature_name: Move objects by specific distances
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move objects by specific distances to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Move objects by specific distances with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move objects by specific distances
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/move-objects-by-specific-distances.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-or-duplicate-an-object-by-pasting-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-or-duplicate-an-object-by-pasting-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-move-or-duplicate-an-object-by-pasting-html
  feature_name: Move or duplicate an object by pasting
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move or duplicate an object by pasting to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Move or duplicate an object by pasting with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move or duplicate an object by pasting
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/move-or-duplicate-an-object-by-pasting.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-paste-an-object-relative-to-other-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-paste-an-object-relative-to-other-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-paste-an-object-relative-to-other-objects-html
  feature_name: Paste an object relative to other objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paste an object relative to other objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Paste an object relative to other objects with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Paste an object relative to other objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/paste-an-object-relative-to-other-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-align-and-distribute-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-align-and-distribute-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-align-and-distribute-objects-html
  feature_name: Align or distribute selected objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align or distribute selected objects as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Align or distribute selected objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Align or distribute selected objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/align-and-distribute-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-distribute-objects-by-specific-distances-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-distribute-objects-by-specific-distances-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-distribute-objects-by-specific-distances-html
  feature_name: Distribute objects by specific distances
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Distribute objects by specific distances to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Distribute objects by specific distances with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Distribute objects by specific distances
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/distribute-objects-by-specific-distances.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-expand-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-expand-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-expand-objects-html
  feature_name: Expand objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Expand objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Expand objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Expand objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/expand-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-objects-html
  feature_name: Rotate objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rotate objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rotate objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/rotate-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-multiple-objects-individually-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-multiple-objects-individually-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-multiple-objects-individually-html
  feature_name: Rotate multiple objects individually
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate multiple objects individually to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rotate multiple objects individually with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rotate multiple objects individually
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/rotate-multiple-objects-individually.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-objects-by-specific-angles-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-objects-by-specific-angles-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-rotate-objects-by-specific-angles-html
  feature_name: Rotate objects by specific angles
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate objects by specific angles to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rotate objects by specific angles with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rotate objects by specific angles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/rotate-objects-by-specific-angles.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-reflect-or-flip-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-reflect-or-flip-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-reflect-or-flip-objects-html
  feature_name: Reflect or flip objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reflect or flip objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Reflect or flip objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Reflect or flip objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/reflect-or-flip-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-reflect-objects-along-an-axis-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-reflect-objects-along-an-axis-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-arrange-objects-reflect-objects-along-an-axis-html
  feature_name: Reflect objects along an axis
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reflect objects along an axis to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Reflect objects along an axis with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Reflect objects along an axis
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/arrange-objects/reflect-objects-along-an-axis.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-divide-or-split-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-divide-or-split-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-divide-or-split-objects-html
  feature_name: Divide or split objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Divide or split objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Divide or split objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Divide or split objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/divide-or-split-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-cut-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-cut-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-cut-objects-html
  feature_name: Cut objects using the Knife and Scissors tools
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Cut objects using the Knife and Scissors tools to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Cut objects using the Knife and Scissors tools with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Cut objects using the Knife and Scissors tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/cut-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-duplicate-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-duplicate-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-duplicate-objects-html
  feature_name: Duplicate objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Duplicate objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Duplicate objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Duplicate objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/duplicate-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-offset-duplicate-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-offset-duplicate-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-offset-duplicate-objects-html
  feature_name: Create duplicate objects using Offset Path
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create duplicate objects using Offset Path to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Create duplicate objects using Offset Path with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create duplicate objects using Offset Path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/offset-duplicate-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-similar-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-similar-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-similar-objects-html
  feature_name: Edit similar objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit similar objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit similar objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit similar objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/edit-similar-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-groups-with-similar-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-groups-with-similar-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-groups-with-similar-objects-html
  feature_name: Edit groups with similar objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit groups with similar objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit groups with similar objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit groups with similar objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/edit-groups-with-similar-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-about-clipping-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-about-clipping-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-about-clipping-masks-html
  feature_name: About clipping masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About clipping masks as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named About clipping masks with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / About clipping masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/about-clipping-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-create-clipping-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-create-clipping-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-create-clipping-masks-html
  feature_name: Create clipping masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create clipping masks as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Create clipping masks with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Create clipping masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/create-clipping-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-clipping-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-clipping-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-edit-clipping-masks-html
  feature_name: Edit clipping masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit clipping masks as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Edit clipping masks with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Edit clipping masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/edit-clipping-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-add-remove-or-release-objects-from-clipping-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-add-remove-or-release-objects-from-clipping-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-add-remove-or-release-objects-from-clipping-masks-html
  feature_name: Add, remove, or release objects from clipping masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add, remove, or release objects from clipping masks as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Add, remove, or release objects from clipping masks with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Add, remove, or release objects from clipping masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/add-remove-or-release-objects-from-clipping-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-hide-parts-of-objects-with-clipping-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-hide-parts-of-objects-with-clipping-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-hide-parts-of-objects-with-clipping-masks-html
  feature_name: Hide parts of objects with clipping masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Hide parts of objects with clipping masks as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Hide parts of objects with clipping masks with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Hide parts of objects with clipping masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/hide-parts-of-objects-with-clipping-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-copy-artwork-using-clipboard-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-edit-objects-copy-artwork-using-clipboard-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-edit-objects-copy-artwork-using-clipboard-html
  feature_name: Copy artwork using the clipboard
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Copy artwork using the clipboard to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Copy artwork using the clipboard with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Copy artwork using the clipboard
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/edit-objects/copy-artwork-using-clipboard.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-pathfinder-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-pathfinder-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-pathfinder-panel-overview-html
  feature_name: Pathfinder panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Pathfinder panel overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Pathfinder panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Pathfinder panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/pathfinder-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-combine-objects-using-pathfinder-effects.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-combine-objects-using-pathfinder-effects.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-combine-objects-using-pathfinder-effects
  feature_name: Edit areas of overlapping objects with Pathfinder
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit areas of overlapping objects with Pathfinder to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit areas of overlapping objects with Pathfinder with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit areas of overlapping objects with Pathfinder
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/combine-objects-using-pathfinder-effects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-compound-shapes-with-pathfinder-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-compound-shapes-with-pathfinder-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-compound-shapes-with-pathfinder-ht
  feature_name: Create compound shapes with Pathfinder
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create compound shapes with Pathfinder to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Create compound shapes with Pathfinder with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create compound shapes with Pathfinder
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/create-compound-shapes-with-pathfinder.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-compound-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-compound-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-compound-paths-html
  feature_name: Create compound paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create compound paths to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Create compound paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create compound paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/create-compound-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-transform-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-transform-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-transform-objects-html
  feature_name: Transform objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transform objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Transform objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Transform objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/transform-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-transform-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-transform-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-transform-panel-overview-html
  feature_name: Transform panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transform panel overview to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Transform panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Transform panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/transform-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-scale-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-scale-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-scale-objects-html
  feature_name: Scale objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Scale objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Scale objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/scale-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-scale-multiple-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-scale-multiple-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-scale-multiple-objects-html
  feature_name: Scale multiple objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale multiple objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Scale multiple objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Scale multiple objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/scale-multiple-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-distort-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-distort-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-distort-objects-html
  feature_name: Distort object
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Distort object to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Distort object with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Distort object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/distort-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-distort-objects-with-envelopes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-distort-objects-with-envelopes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-distort-objects-with-envelopes-html
  feature_name: Distort objects with envelopes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Distort objects with envelopes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Distort objects with envelopes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Distort objects with envelopes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/distort-objects-with-envelopes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-edit-the-contents-of-envelopes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-edit-the-contents-of-envelopes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-edit-the-contents-of-envelopes-html
  feature_name: Edit the contents of envelopes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit the contents of envelopes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit the contents of envelopes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit the contents of envelopes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/edit-the-contents-of-envelopes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-envelope-panel-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-envelope-panel-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-envelope-panel-options-html
  feature_name: Envelope panel options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Envelope panel options to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Envelope panel options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Envelope panel options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/envelope-panel-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-shear-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-shear-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-shear-objects-html
  feature_name: Shear objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Shear objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Shear objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Shear objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/shear-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-intertwined-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-intertwined-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-create-intertwined-objects-html
  feature_name: Intertwine objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Intertwine objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Intertwine objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Intertwine objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/create-intertwined-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-about-perspective-drawing-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-about-perspective-drawing-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-about-perspective-drawing-html
  feature_name: About perspective drawing
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About perspective drawing to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named About perspective drawing with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / About perspective drawing
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/about-perspective-drawing.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-draw-objects-in-perspective-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-draw-objects-in-perspective-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-draw-objects-in-perspective-html
  feature_name: Draw objects in perspective
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw objects in perspective to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Draw objects in perspective with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Draw objects in perspective
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/draw-objects-in-perspective.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-perspective-grid-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-perspective-grid-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-perspective-grid-options-html
  feature_name: Perspective grid options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Perspective grid options to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Perspective grid options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Perspective grid options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/perspective-grid-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-define-object-or-document-perspective-htm.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-define-object-or-document-perspective-htm.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-define-object-or-document-perspective-htm
  feature_name: Define and manage perspective grid preset
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Define and manage perspective grid preset to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Define and manage perspective grid preset with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Define and manage perspective grid preset
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/define-object-or-document-perspective.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-move-perspective-grids-and-adjust-vanishi.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-move-perspective-grids-and-adjust-vanishi.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-move-perspective-grids-and-adjust-vanishi
  feature_name: Move the perspective grid and adjust its vanishing points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move the perspective grid and adjust its vanishing points to create, edit, transform, or inspect vector geometry in Studio without relying on
    a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Move the perspective grid and adjust its vanishing points with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move the perspective grid and adjust its vanishing points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/move-perspective-grids-and-adjust-vanishing-points.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-adjust-grid-cell-size-and-grid-extent-htm.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-adjust-grid-cell-size-and-grid-extent-htm.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-adjust-grid-cell-size-and-grid-extent-htm
  feature_name: Adjust grid cell size and grid extent
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust grid cell size and grid extent to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Adjust grid cell size and grid extent with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adjust grid cell size and grid extent
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/adjust-grid-cell-size-and-grid-extent.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-adjust-horizon-heights-and-grid-planes-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-adjust-horizon-heights-and-grid-planes-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-adjust-horizon-heights-and-grid-planes-ht
  feature_name: Adjust horizon heights and grid planes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust horizon heights and grid planes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Adjust horizon heights and grid planes with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adjust horizon heights and grid planes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/adjust-horizon-heights-and-grid-planes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-modify-perspective-grid-settings-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-modify-perspective-grid-settings-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-reshape-transform-objects-modify-perspective-grid-settings-html
  feature_name: Adjust the perspective grid and the active plane widget
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust the perspective grid and the active plane widget to create, edit, transform, or inspect vector geometry in Studio without relying on a
    vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Adjust the perspective grid and the active plane widget with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adjust the perspective grid and the active plane widget
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/reshape-transform-objects/modify-perspective-grid-settings.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-trace-images-to-convert-raster-into-vector-a.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-trace-images-to-convert-raster-into-vector-a.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-trace-images-to-convert-raster-into-vector-a
  feature_name: Vectorize images using Image Trace
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Vectorize images using Image Trace to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Vectorize images using Image Trace with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Vectorize images using Image Trace
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/trace-images-to-convert-raster-into-vector-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-image-trace-results-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-image-trace-results-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-image-trace-results-html
  feature_name: Edit tracing results
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit tracing results to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Edit tracing results with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Edit tracing results
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/edit-image-trace-results.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-image-trace-presets-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-image-trace-presets-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-image-trace-presets-html
  feature_name: Save custom tracing presets
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save custom tracing presets to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Save custom tracing presets with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Save custom tracing presets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/save-image-trace-presets.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-mockups-for-images-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-mockups-for-images-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-mockups-for-images-html
  feature_name: Create mockups for images
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create mockups for images to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create mockups for images with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create mockups for images
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/create-mockups-for-images.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-mockups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-mockups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-mockups-html
  feature_name: Edit mockups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit mockups to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit mockups with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit mockups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/edit-mockups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-mockups-as-templates-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-mockups-as-templates-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-save-mockups-as-templates-html
  feature_name: Save mockups as templates
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Save mockups as templates to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Save mockups as templates with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Save mockups as templates
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/save-mockups-as-templates.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-and-place-symbols-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-and-place-symbols-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-and-place-symbols-html
  feature_name: Create and place symbols
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create and place symbols to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create and place symbols with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create and place symbols
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/create-and-place-symbols.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-symbols-panel-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-symbols-panel-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-symbols-panel-options-html
  feature_name: Symbols panel options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Symbols panel options to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Symbols panel options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Symbols panel options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/symbols-panel-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-symbols-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-symbols-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-edit-symbols-html
  feature_name: Edit symbols
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit symbols to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit symbols with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit symbols
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/edit-symbols.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-or-import-symbol-libraries-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-or-import-symbol-libraries-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-create-or-import-symbol-libraries-html
  feature_name: Create or import symbol libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create or import symbol libraries to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Create or import symbol libraries with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Create or import symbol libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/create-or-import-symbol-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-maintain-proportions-while-scaling-symbols-h.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-maintain-proportions-while-scaling-symbols-h.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-maintain-proportions-while-scaling-symbols-h
  feature_name: Maintain proportions while scaling symbols
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Maintain proportions while scaling symbols as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Maintain proportions while scaling symbols with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Maintain proportions while scaling symbols
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/maintain-proportions-while-scaling-symbols.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-transform-symbols-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-transform-symbols-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-objects-traces-mockups-symbols-transform-symbols-html
  feature_name: Transform symbols
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: manage_objects
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transform symbols to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Transform symbols with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Transform symbols
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-objects/traces-mockups-symbols/transform-symbols.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-about-rulers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-about-rulers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-about-rulers-html
  feature_name: About rulers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About rulers to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named About rulers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / About rulers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/about-rulers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-rulers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-rulers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-rulers-html
  feature_name: Use rulers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use rulers to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Use rulers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Use rulers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/use-rulers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-work-with-decimal-tabs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-work-with-decimal-tabs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-work-with-decimal-tabs-html
  feature_name: Work with decimal tabs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with decimal tabs to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Work with decimal tabs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Work with decimal tabs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/work-with-decimal-tabs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-manage-tab-stops-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-manage-tab-stops-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-manage-tab-stops-html
  feature_name: Manage tab stops
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage tab stops to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Manage tab stops with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Manage tab stops
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/manage-tab-stops.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-grids-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-grids-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-grids-html
  feature_name: Align graphic objects with grids
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align graphic objects with grids to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Align graphic objects with grids with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Align graphic objects with grids
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/align-graphic-objects-with-grids.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-guides-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-guides-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-guides-html
  feature_name: Align graphic objects with guides
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align graphic objects with guides to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Align graphic objects with guides with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Align graphic objects with guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/align-graphic-objects-with-guides.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-tabs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-tabs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-align-graphic-objects-with-tabs-html
  feature_name: Align graphic objects with Tabs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align graphic objects with Tabs to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Align graphic objects with Tabs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Align graphic objects with Tabs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/align-graphic-objects-with-tabs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-distance-guides-for-accurate-placement-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-distance-guides-for-accurate-placement-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-use-distance-guides-for-accurate-placement-html
  feature_name: Use Distance Guides for accurate placement
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use Distance Guides for accurate placement to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Use Distance Guides for accurate placement with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Use Distance Guides for accurate placement
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/use-distance-guides-for-accurate-placement.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-work-with-smart-guides-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-work-with-smart-guides-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-work-with-smart-guides-html
  feature_name: Work with Smart Guides
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with Smart Guides to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Work with Smart Guides with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Work with Smart Guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/work-with-smart-guides.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-smart-guides-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-smart-guides-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-grids-and-guides-smart-guides-options-html
  feature_name: Smart Guides options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Smart Guides options to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Smart Guides options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Smart Guides options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/grids-and-guides/smart-guides-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-measure-the-distance-between-two-points-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-measure-the-distance-between-two-points-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-measure-the-distance-between-two-points-html
  feature_name: Measure the distance between two points
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Measure the distance between two points to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Measure the distance between two points with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Measure the distance between two points
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/measure-the-distance-between-two-points.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-measure-the-area-of-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-measure-the-area-of-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-measure-the-area-of-objects-html
  feature_name: Measure the area of objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Measure the area of objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Measure the area of objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Measure the area of objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/measure-the-area-of-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-area-measurement-scenarios-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-area-measurement-scenarios-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-area-measurement-scenarios-html
  feature_name: Area measurement scenarios
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Area measurement scenarios to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Area measurement scenarios with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Area measurement scenarios
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/area-measurement-scenarios.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-about-dimension-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-about-dimension-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-about-dimension-objects-html
  feature_name: About Dimension objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About Dimension objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named About Dimension objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / About Dimension objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/about-dimension-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-linear-dimensions-of-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-linear-dimensions-of-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-linear-dimensions-of-objects-html
  feature_name: Plot the linear dimensions of objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Plot the linear dimensions of objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Plot the linear dimensions of objects with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Plot the linear dimensions of objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/plot-linear-dimensions-of-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-angular-dimensions-of-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-angular-dimensions-of-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-angular-dimensions-of-objects-html
  feature_name: Plot the angular dimensions of objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Plot the angular dimensions of objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Plot the angular dimensions of objects with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Plot the angular dimensions of objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/plot-angular-dimensions-of-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-radial-dimensions-of-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-radial-dimensions-of-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-plot-radial-dimensions-of-objects-html
  feature_name: Plot radial dimensions of objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Plot radial dimensions of objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Plot radial dimensions of objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Plot radial dimensions of objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/plot-radial-dimensions-of-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-dimension-tool-options-and-settings-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-dimension-tool-options-and-settings-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-dimension-tool-options-and-settings-html
  feature_name: Dimension tool options and settings
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Dimension tool options and settings to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Dimension tool options and settings with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Dimension tool options and settings
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/dimension-tool-options-and-settings.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-snap-to-perpendicular-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-snap-to-perpendicular-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-snap-to-perpendicular-html
  feature_name: Snap a line perpendicular to a path in Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Snap a line perpendicular to a path in Illustrator to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Snap a line perpendicular to a path in Illustrator with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Snap a line perpendicular to a path in Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/snap-to-perpendicular.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-snap-to-tangent-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-snap-to-tangent-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-measure-and-align-plot-and-measure-snap-to-tangent-html
  feature_name: Snap a line tangent to a curve in Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Snap a line tangent to a curve in Illustrator to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named
    tool surface.
  user_goal: A Studio operator can perform the source workflow named Snap a line tangent to a curve in Illustrator with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Snap a line tangent to a curve in Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/measure-and-align/plot-and-measure/snap-to-tangent.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-about-fills-and-strokes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-about-fills-and-strokes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-about-fills-and-strokes-html
  feature_name: About fills and strokes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About fills and strokes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named About fills and strokes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / About fills and strokes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/about-fills-and-strokes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-fill-and-stroke-controls-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-fill-and-stroke-controls-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-fill-and-stroke-controls-html
  feature_name: Fill and stroke controls
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Fill and stroke controls as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Fill and stroke controls with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Fill and stroke controls
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/fill-and-stroke-controls.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-apply-fill-colors-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-apply-fill-colors-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-apply-fill-colors-html
  feature_name: Apply fill colors
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply fill colors as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Apply fill colors with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply fill colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/apply-fill-colors.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-apply-stroke-colors-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-apply-stroke-colors-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-apply-stroke-colors-html
  feature_name: Apply stroke colors
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply stroke colors as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Apply stroke colors with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply stroke colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/apply-stroke-colors.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-create-multiple-fills-and-strokes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-create-multiple-fills-and-strokes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-create-multiple-fills-and-strokes-html
  feature_name: Create multiple fills and strokes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create multiple fills and strokes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Create multiple fills and strokes with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create multiple fills and strokes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/create-multiple-fills-and-strokes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-convert-strokes-to-compound-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-convert-strokes-to-compound-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-convert-strokes-to-compound-paths-html
  feature_name: Convert strokes to compound paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert strokes to compound paths as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Convert strokes to compound paths with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Convert strokes to compound paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/convert-strokes-to-compound-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-remove-fills-or-strokes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-remove-fills-or-strokes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-remove-fills-or-strokes-html
  feature_name: Remove fills or strokes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove fills or strokes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Remove fills or strokes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Remove fills or strokes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/remove-fills-or-strokes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-select-objects-with-same-fill-and-stroke-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-select-objects-with-same-fill-and-stroke-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-select-objects-with-same-fill-and-stroke-html
  feature_name: Select objects with same fill and stroke
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select objects with same fill and stroke as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Select objects with same fill and stroke with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Select objects with same fill and stroke
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/select-objects-with-same-fill-and-stroke.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-paint-tools-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-paint-tools-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-paint-tools-overview-html
  feature_name: Paint tools overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paint tools overview as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Paint tools overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Paint tools overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/paint-tools-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-merge-paths-using-the-blob-brush-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-merge-paths-using-the-blob-brush-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-merge-paths-using-the-blob-brush-tool-html
  feature_name: Merge paths using the Blob Brush tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Merge paths using the Blob Brush tool as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Merge paths using the Blob Brush tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Merge paths using the Blob Brush tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/merge-paths-using-the-blob-brush-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-blob-brush-options-and-best-practices-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-blob-brush-options-and-best-practices-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-blob-brush-options-and-best-practices-html
  feature_name: Blob brush options and best practices
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blob brush options and best practices as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Blob brush options and best practices with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Blob brush options and best practices
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/blob-brush-options-and-best-practices.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-about-live-paint-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-about-live-paint-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-about-live-paint-html
  feature_name: About Live Paint
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About Live Paint as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named About Live Paint with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / About Live Paint
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/about-live-paint.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-create-live-paint-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-create-live-paint-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-create-live-paint-groups-html
  feature_name: Create Live Paint groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create Live Paint groups as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create Live Paint groups with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create Live Paint groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/create-live-paint-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-paint-with-the-live-paint-bucket-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-paint-with-the-live-paint-bucket-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-paint-with-the-live-paint-bucket-tool-html
  feature_name: Paint with the Live Paint Bucket tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paint with the Live Paint Bucket tool as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Paint with the Live Paint Bucket tool with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Paint with the Live Paint Bucket tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/paint-with-the-live-paint-bucket-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-modify-live-paint-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-modify-live-paint-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-modify-live-paint-groups-html
  feature_name: Modify Live Paint groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify Live Paint groups as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Modify Live Paint groups with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Modify Live Paint groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/modify-live-paint-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-find-and-close-gaps-in-live-paint-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-find-and-close-gaps-in-live-paint-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-learn-painting-basics-find-and-close-gaps-in-live-paint-groups-html
  feature_name: Find and close gaps in Live Paint groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find and close gaps in Live Paint groups as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Find and close gaps in Live Paint groups with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Find and close gaps in Live Paint groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/learn-painting-basics/find-and-close-gaps-in-live-paint-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-about-brushes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-about-brushes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-about-brushes-html
  feature_name: About brushes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About brushes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named About brushes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / About brushes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/about-brushes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-brushes-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-brushes-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-brushes-panel-overview-html
  feature_name: Brushes panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Brushes panel overview as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Brushes panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Brushes panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/brushes-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-draw-paths-with-brush-strokes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-draw-paths-with-brush-strokes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-draw-paths-with-brush-strokes-html
  feature_name: Draw paths with brush strokes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Draw paths with brush strokes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in
    the core.
  user_goal: A Studio operator can perform the source workflow named Draw paths with brush strokes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Draw paths with brush strokes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/draw-paths-with-brush-strokes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-apply-brush-strokes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-apply-brush-strokes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-apply-brush-strokes-html
  feature_name: Apply brush strokes to paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply brush strokes to paths as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Apply brush strokes to paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply brush strokes to paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/apply-brush-strokes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-remove-brush-strokes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-remove-brush-strokes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-remove-brush-strokes-html
  feature_name: Remove brush strokes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove brush strokes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Remove brush strokes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Remove brush strokes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/remove-brush-strokes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-add-arrowheads-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-add-arrowheads-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-add-arrowheads-html
  feature_name: Add arrowheads to paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add arrowheads to paths as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Add arrowheads to paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Add arrowheads to paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/add-arrowheads.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-customize-arrowheads-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-customize-arrowheads-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-customize-arrowheads-html
  feature_name: Customize arrowheads
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Customize arrowheads as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Customize arrowheads with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Customize arrowheads
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/customize-arrowheads.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-paintbrush-tool-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-paintbrush-tool-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-paintbrush-tool-options-html
  feature_name: Paintbrush tool options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paintbrush tool options as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Paintbrush tool options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Paintbrush tool options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/paintbrush-tool-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-new-brush-libraries-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-new-brush-libraries-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-new-brush-libraries-html
  feature_name: Create brush libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create brush libraries as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create brush libraries with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create brush libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/create-new-brush-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-import-brushes-from-brush-libraries-or-other.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-import-brushes-from-brush-libraries-or-other.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-import-brushes-from-brush-libraries-or-other
  feature_name: Import brushes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import brushes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Import brushes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Import brushes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/import-brushes-from-brush-libraries-or-other-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-brushes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-brushes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-brushes-html
  feature_name: Create brushes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create brushes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create brushes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create brushes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/create-brushes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-dotted-or-dashed-lines-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-dotted-or-dashed-lines-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-create-dotted-or-dashed-lines-html
  feature_name: Create dotted or dashed lines
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create dotted or dashed lines as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in
    the core.
  user_goal: A Studio operator can perform the source workflow named Create dotted or dashed lines with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create dotted or dashed lines
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/create-dotted-or-dashed-lines.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-modify-brushes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-modify-brushes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-modify-brushes-html
  feature_name: Modify brushes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify brushes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Modify brushes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Modify brushes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/modify-brushes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-change-the-caps-or-joins-of-a-line-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-change-the-caps-or-joins-of-a-line-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-change-the-caps-or-joins-of-a-line-html
  feature_name: Change the caps or joins of a line
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change the caps or joins of a line as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Change the caps or joins of a line with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Change the caps or joins of a line
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/change-the-caps-or-joins-of-a-line.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-convert-brush-strokes-to-outlines-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-convert-brush-strokes-to-outlines-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-apply-and-edit-strokes-convert-brush-strokes-to-outlines-html
  feature_name: Convert brush strokes to outlines
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert brush strokes to outlines as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Convert brush strokes to outlines with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Convert brush strokes to outlines
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/apply-and-edit-strokes/convert-brush-strokes-to-outlines.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-gradients-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-gradients-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-gradients-overview-html
  feature_name: Gradients overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Gradients overview as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Gradients overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Gradients overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/gradients-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-a-predefined-gradient-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-a-predefined-gradient-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-a-predefined-gradient-html
  feature_name: Apply predefined gradients
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply predefined gradients as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Apply predefined gradients with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply predefined gradients
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/apply-a-predefined-gradient.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-a-linear-gradient-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-a-linear-gradient-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-a-linear-gradient-html
  feature_name: Apply linear gradients
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply linear gradients as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Apply linear gradients with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply linear gradients
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/create-and-apply-a-linear-gradient.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-radial-gradients-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-radial-gradients-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-radial-gradients-html
  feature_name: Apply radial gradients
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply radial gradients as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Apply radial gradients with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply radial gradients
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/create-and-apply-radial-gradients.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-freeform-gradients-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-freeform-gradients-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-create-and-apply-freeform-gradients-html
  feature_name: Apply freeform gradients
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply freeform gradients as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Apply freeform gradients with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply freeform gradients
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/create-and-apply-freeform-gradients.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-gradients-on-stroke-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-gradients-on-stroke-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-gradients-on-stroke-html
  feature_name: Apply gradients on stroke
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply gradients on stroke as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Apply gradients on stroke with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply gradients on stroke
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/apply-gradients-on-stroke.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-gradients-across-multiple-objects-h.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-gradients-across-multiple-objects-h.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-apply-gradients-across-multiple-objects-h
  feature_name: Apply gradients across multiple objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply gradients across multiple objects as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Apply gradients across multiple objects with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Apply gradients across multiple objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/apply-gradients-across-multiple-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-control-dither-in-gradients-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-control-dither-in-gradients-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-control-dither-in-gradients-html
  feature_name: Control dither in gradients
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Control dither in gradients as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Control dither in gradients with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Control dither in gradients
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/control-dither-in-gradients.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-control-perceptual-interpolation-in-gradi.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-control-perceptual-interpolation-in-gradi.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-gradients-control-perceptual-interpolation-in-gradi
  feature_name: Control perceptual interpolation in gradients
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Control perceptual interpolation in gradients as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud
    dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Control perceptual interpolation in gradients with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Control perceptual interpolation in gradients
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-gradients/control-perceptual-interpolation-in-gradients.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-mesh-objects-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-mesh-objects-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-mesh-objects-overview-html
  feature_name: Mesh objects overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Mesh objects overview as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Mesh objects overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Mesh objects overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-meshes/mesh-objects-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-create-mesh-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-create-mesh-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-create-mesh-objects-html
  feature_name: Create mesh objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create mesh objects as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create mesh objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create mesh objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-meshes/create-mesh-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-edit-mesh-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-edit-mesh-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-edit-mesh-objects-html
  feature_name: Edit mesh objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit mesh objects as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Edit mesh objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Edit mesh objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-meshes/edit-mesh-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-set-transparency-for-gradient-meshes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-set-transparency-for-gradient-meshes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-set-transparency-for-gradient-meshes-html
  feature_name: Set transparency for gradient meshes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set transparency for gradient meshes as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Set transparency for gradient meshes with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Set transparency for gradient meshes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-meshes/set-transparency-for-gradient-meshes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-convert-mesh-objects-to-path-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-convert-mesh-objects-to-path-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-convert-mesh-objects-to-path-objects-html
  feature_name: Convert mesh objects to path objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert mesh objects to path objects as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Convert mesh objects to path objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Convert mesh objects to path objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-meshes/convert-mesh-objects-to-path-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-convert-a-gradient-filled-object-to-a-mesh-o.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-convert-a-gradient-filled-object-to-a-mesh-o.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-meshes-convert-a-gradient-filled-object-to-a-mesh-o
  feature_name: Convert a gradient-filled object to a mesh object
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert a gradient-filled object to a mesh object as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no
    cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Convert a gradient-filled object to a mesh object with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Convert a gradient-filled object to a mesh object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-meshes/convert-a-gradient-filled-object-to-a-mesh-object.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-patterns-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-patterns-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-patterns-overview-html
  feature_name: Pattern overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Pattern overview as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Pattern overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Pattern overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/patterns-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-patterns-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-patterns-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-patterns-html
  feature_name: Create patterns
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create patterns as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create patterns with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create patterns
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/create-patterns.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-edit-patterns-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-edit-patterns-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-edit-patterns-html
  feature_name: Edit patterns
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit patterns as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Edit patterns with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Edit patterns
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/edit-patterns.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-repeat-patterns-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-repeat-patterns-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-repeat-patterns-overview-html
  feature_name: Repeat patterns overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Repeat patterns overview as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Repeat patterns overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Repeat patterns overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/repeat-patterns-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-radial-repeats-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-radial-repeats-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-radial-repeats-html
  feature_name: Create radial repeats
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create radial repeats as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create radial repeats with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create radial repeats
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/create-radial-repeats.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-radial-repeat-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-radial-repeat-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-radial-repeat-options-html
  feature_name: Set radial repeat options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set radial repeat options as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Set radial repeat options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Set radial repeat options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/set-radial-repeat-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-grid-repeats-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-grid-repeats-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-grid-repeats-html
  feature_name: Create and modify a grid repeat
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create and modify a grid repeat as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in
    the core.
  user_goal: A Studio operator can perform the source workflow named Create and modify a grid repeat with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create and modify a grid repeat
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/create-grid-repeats.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-grid-repeat-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-grid-repeat-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-grid-repeat-options-html
  feature_name: Set grid repeat options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set grid repeat options as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Set grid repeat options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Set grid repeat options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/set-grid-repeat-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-mirror-repeats-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-mirror-repeats-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-create-mirror-repeats-html
  feature_name: Create mirror repeats
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create mirror repeats as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Create mirror repeats with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Create mirror repeats
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/create-mirror-repeats.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-mirror-repeat-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-mirror-repeat-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-paint-and-fill-create-and-edit-patterns-set-mirror-repeat-options-html
  feature_name: Set mirror repeat options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set mirror repeat options as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the
    core.
  user_goal: A Studio operator can perform the source workflow named Set mirror repeat options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Set mirror repeat options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/paint-and-fill/create-and-edit-patterns/set-mirror-repeat-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-color-guide-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-color-guide-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-color-guide-panel-overview-html
  feature_name: Color Guide panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Color Guide panel overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Color Guide panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Color Guide panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/learn-color-basics/color-guide-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-create-color-groups-in-the-color-guide-panel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-create-color-groups-in-the-color-guide-panel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-create-color-groups-in-the-color-guide-panel-html
  feature_name: Create swatch groups in the Color Guide panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create swatch groups in the Color Guide panel to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create swatch groups in the Color Guide panel with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create swatch groups in the Color Guide panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/learn-color-basics/create-color-groups-in-the-color-guide-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-edit-colors-dialog-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-edit-colors-dialog-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-learn-color-basics-edit-colors-dialog-overview-html
  feature_name: Advanced recolor options overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Advanced recolor options overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Advanced recolor options overview with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Advanced recolor options overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/learn-color-basics/edit-colors-dialog-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-about-selecting-colors-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-about-selecting-colors-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-about-selecting-colors-html
  feature_name: About selecting colors
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About selecting colors to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named About selecting colors with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / About selecting colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/about-selecting-colors.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-color-picker-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-color-picker-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-color-picker-overview-html
  feature_name: Color Picker overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Color Picker overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Color Picker overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Color Picker overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/color-picker-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-using-the-color-picker-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-using-the-color-picker-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-using-the-color-picker-html
  feature_name: Select colors using the Color Picker
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select colors using the Color Picker to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Select colors using the Color Picker with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Select colors using the Color Picker
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/select-colors-using-the-color-picker.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-color-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-color-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-color-panel-overview-html
  feature_name: Color panel options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Color panel options to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Color panel options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Color panel options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/color-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-using-the-color-panel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-using-the-color-panel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-using-the-color-panel-html
  feature_name: Select colors using the Color panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select colors using the Color panel to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Select colors using the Color panel with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Select colors using the Color panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/select-colors-using-the-color-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-convert-color-modes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-convert-color-modes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-convert-color-modes-html
  feature_name: Convert color modes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Convert color modes to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Convert color modes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Convert color modes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/convert-color-modes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-make-colors-printable-or-web-safe-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-make-colors-printable-or-web-safe-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-make-colors-printable-or-web-safe-html
  feature_name: Make colors printable or web safe
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Make colors printable or web safe as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency
    in the core.
  user_goal: A Studio operator can perform the source workflow named Make colors printable or web safe with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Make colors printable or web safe
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/make-colors-printable-or-web-safe.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-change-colors-to-their-inverse-or-complemen.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-change-colors-to-their-inverse-or-complemen.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-change-colors-to-their-inverse-or-complemen
  feature_name: Change colors to their inverse or complement
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change colors to their inverse or complement to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Change colors to their inverse or complement with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Change colors to their inverse or complement
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/change-colors-to-their-inverse-or-complement.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-get-precise-spot-colors-using-lab-values-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-get-precise-spot-colors-using-lab-values-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-get-precise-spot-colors-using-lab-values-ht
  feature_name: Get precise spot colors using lab values
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get precise spot colors using lab values to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Get precise spot colors using lab values with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Get precise spot colors using lab values
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/get-precise-spot-colors-using-lab-values.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-adjust-saturation-of-multiple-colors-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-adjust-saturation-of-multiple-colors-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-adjust-saturation-of-multiple-colors-html
  feature_name: Adjust saturation of multiple colors
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust saturation of multiple colors to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Adjust saturation of multiple colors with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Adjust saturation of multiple colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/adjust-saturation-of-multiple-colors.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-adjust-color-balance-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-adjust-color-balance-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-adjust-color-balance-html
  feature_name: Adjust color balance
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust color balance to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Adjust color balance with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Adjust color balance
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/adjust-color-balance.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-blend-colors-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-blend-colors-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-blend-colors-html
  feature_name: Blend colors
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blend colors to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Blend colors with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Blend colors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/blend-colors.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-change-color-tints-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-change-color-tints-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-change-color-tints-html
  feature_name: Change color tints
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change color tints to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Change color tints with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Change color tints
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/change-color-tints.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-select-and-adjust-colors-select-colors-html
  feature_name: Select colors using the Eyedropper and Color Picker tool
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select colors using the Eyedropper and Color Picker tool to control fills, color, gradients, effects, blends, profiles, or appearance state in
    Studio.
  user_goal: A Studio operator can perform the source workflow named Select colors using the Eyedropper and Color Picker tool with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Select colors using the Eyedropper and Color Picker tool
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/select-and-adjust-colors/select-colors.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-about-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-about-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-about-swatches-html
  feature_name: About Swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About Swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named About Swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / About Swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/about-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-swatches-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-swatches-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-swatches-panel-overview-html
  feature_name: Swatches panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Swatches panel overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Swatches panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Swatches panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/swatches-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-add-colors-to-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-add-colors-to-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-add-colors-to-swatches-html
  feature_name: Add colors to swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add colors to swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Add colors to swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Add colors to swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/add-colors-to-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-swatches-from-the-color-guide-panel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-swatches-from-the-color-guide-panel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-swatches-from-the-color-guide-panel-html
  feature_name: Create swatches from the Color Guide panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create swatches from the Color Guide panel to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create swatches from the Color Guide panel with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create swatches from the Color Guide panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/create-swatches-from-the-color-guide-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-process-color-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-process-color-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-process-color-swatches-html
  feature_name: Create process color swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create process color swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create process color swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create process color swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/create-process-color-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-spot-color-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-spot-color-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-spot-color-swatches-html
  feature_name: Create spot-color swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create spot-color swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create spot-color swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create spot-color swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/create-spot-color-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-gradient-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-gradient-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-gradient-swatches-html
  feature_name: Create gradient swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create gradient swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create gradient swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create gradient swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/create-gradient-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-reorder-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-reorder-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-reorder-swatches-html
  feature_name: Reorder swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reorder swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Reorder swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Reorder swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/reorder-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-duplicate-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-duplicate-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-duplicate-swatches-html
  feature_name: Duplicate swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Duplicate swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Duplicate swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Duplicate swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/duplicate-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-replace-merge-or-delete-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-replace-merge-or-delete-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-replace-merge-or-delete-swatches-html
  feature_name: Replace, merge, or delete swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: local_first_collaboration_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace, merge, or delete swatches to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Replace, merge, or delete swatches with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Replace, merge, or delete swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/replace-merge-or-delete-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-group-swatches-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-group-swatches-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-group-swatches-html
  feature_name: Group swatches
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Group swatches to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Group swatches with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Group swatches
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/group-swatches.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-move-colors-into-swatch-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-move-colors-into-swatch-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-move-colors-into-swatch-groups-html
  feature_name: Move colors into swatch groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move colors into swatch groups to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Move colors into swatch groups with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Move colors into swatch groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/move-colors-into-swatch-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-and-open-swatch-libraries-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-and-open-swatch-libraries-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-create-and-open-swatch-libraries-html
  feature_name: Create and open swatch libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create and open swatch libraries to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Create and open swatch libraries with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create and open swatch libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/create-and-open-swatch-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-edit-swatch-libraries-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-edit-swatch-libraries-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-edit-swatch-libraries-html
  feature_name: Edit swatch libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit swatch libraries to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Edit swatch libraries with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Edit swatch libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/edit-swatch-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-move-swatches-from-swatch-libraries-to-the-swatches-pan.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-move-swatches-from-swatch-libraries-to-the-swatches-pan.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-move-swatches-from-swatch-libraries-to-the-swatches-pan
  feature_name: Move swatches from swatch libraries to the swatches panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move swatches from swatch libraries to the swatches panel to control fills, color, gradients, effects, blends, profiles, or appearance state in
    Studio.
  user_goal: A Studio operator can perform the source workflow named Move swatches from swatch libraries to the swatches panel with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Move swatches from swatch libraries to the swatches panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/move-swatches-from-swatch-libraries-to-the-swatches-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-share-swatches-between-applications-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-use-swatches-share-swatches-between-applications-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-use-swatches-share-swatches-between-applications-html
  feature_name: Share swatches between applications
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_first_collaboration_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Share swatches between applications to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Share swatches between applications with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Share swatches between applications
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/use-swatches/share-swatches-between-applications.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-recolor-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-recolor-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-recolor-artwork-html
  feature_name: Recolor artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Recolor artwork to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Recolor artwork with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Recolor artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/recolor-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-recolor-options-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-recolor-options-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-recolor-options-overview-html
  feature_name: Recolor options overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Recolor options overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Recolor options overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Recolor options overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/recolor-options-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-colors-using-edit-colors-dialog-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-colors-using-edit-colors-dialog-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-colors-using-edit-colors-dialog-html
  feature_name: Edit colors using Edit Colors dialog
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit colors using Edit Colors dialog to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Edit colors using Edit Colors dialog with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Edit colors using Edit Colors dialog
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/edit-colors-using-edit-colors-dialog.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-change-colors-in-color-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-change-colors-in-color-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-change-colors-in-color-groups-html
  feature_name: Modify colors in swatch groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify colors in swatch groups to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Modify colors in swatch groups with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Modify colors in swatch groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/change-colors-in-color-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-apply-local-or-global-changes-to-color-properties-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-apply-local-or-global-changes-to-color-properties-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-apply-local-or-global-changes-to-color-properties-html
  feature_name: Apply local or global changes to color properties
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply local or global changes to color properties to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Apply local or global changes to color properties with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Apply local or global changes to color properties
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/apply-local-or-global-changes-to-color-properties.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-assign-new-colors-to-selected-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-assign-new-colors-to-selected-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-assign-new-colors-to-selected-artwork-html
  feature_name: Assign new colors to selected artworks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Assign new colors to selected artworks to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Assign new colors to selected artworks with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Assign new colors to selected artworks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/assign-new-colors-to-selected-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-view-original-colors-in-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-view-original-colors-in-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-view-original-colors-in-artwork-html
  feature_name: View original colors in artworks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View original colors in artworks to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named View original colors in artworks with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / View original colors in artworks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/view-original-colors-in-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-limit-colors-in-artworks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-limit-colors-in-artworks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-limit-colors-in-artworks-html
  feature_name: Limit colors in artworks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Limit colors in artworks to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Limit colors in artworks with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Limit colors in artworks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/limit-colors-in-artworks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-color-reduction-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-color-reduction-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-color-reduction-options-html
  feature_name: Color reduction options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Color reduction options to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Color reduction options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Color reduction options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/color-reduction-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-color-groups-using-the-color-wheel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-color-groups-using-the-color-wheel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-color-groups-using-the-color-wheel-html
  feature_name: Edit swatch groups using the color wheel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit swatch groups using the color wheel to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Edit swatch groups using the color wheel with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Edit swatch groups using the color wheel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/edit-color-groups-using-the-color-wheel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-add-or-remove-colors-from-color-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-add-or-remove-colors-from-color-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-add-or-remove-colors-from-color-groups-html
  feature_name: Add or remove colors from swatch groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add or remove colors from swatch groups to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Add or remove colors from swatch groups with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Add or remove colors from swatch groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/add-or-remove-colors-from-color-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-an-individual-color-in-color-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-an-individual-color-in-color-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-edit-an-individual-color-in-color-groups-html
  feature_name: Edit an individual color in color groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit an individual color in color groups to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Edit an individual color in color groups with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Edit an individual color in color groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/edit-an-individual-color-in-color-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-delete-color-groups-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-modify-colors-delete-color-groups-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-modify-colors-delete-color-groups-html
  feature_name: Delete swatch groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete swatch groups to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Delete swatch groups with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Delete swatch groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/modify-colors/delete-color-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-transparency-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-transparency-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-transparency-panel-overview-html
  feature_name: Transparency panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transparency panel overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Transparency panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Transparency panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/transparency-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-change-the-opacity-of-artworks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-change-the-opacity-of-artworks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-change-the-opacity-of-artworks-html
  feature_name: Adjust opacity
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust opacity to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Adjust opacity with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Adjust opacity
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/change-the-opacity-of-artworks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-about-opacity-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-about-opacity-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-about-opacity-masks-html
  feature_name: Opacity mask overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Opacity mask overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Opacity mask overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Opacity mask overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/about-opacity-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-transparency-using-opacity-ma.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-transparency-using-opacity-ma.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-transparency-using-opacity-ma
  feature_name: Adjust transparency using opacity masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust transparency using opacity masks to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Adjust transparency using opacity masks with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Adjust transparency using opacity masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/create-transparency-using-opacity-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-invert-or-clip-opacity-masks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-invert-or-clip-opacity-masks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-invert-or-clip-opacity-masks-html
  feature_name: Clip and invert opacity masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Clip and invert opacity masks to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Clip and invert opacity masks with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Clip and invert opacity masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/invert-or-clip-opacity-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-deactivate-or-remove-opacity-masks-h.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-deactivate-or-remove-opacity-masks-h.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-deactivate-or-remove-opacity-masks-h
  feature_name: Deactivate, reactivate, or remove opacity masks
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Deactivate, reactivate, or remove opacity masks to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Deactivate, reactivate, or remove opacity masks with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Deactivate, reactivate, or remove opacity masks
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/deactivate-or-remove-opacity-masks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-use-transparency-to-define-knockout.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-use-transparency-to-define-knockout.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-use-transparency-to-define-knockout
  feature_name: Create knockout shapes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create knockout shapes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Create knockout shapes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create knockout shapes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/use-transparency-to-define-knockout-shapes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-transparency-knockout-groups.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-transparency-knockout-groups.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-transparency-knockout-groups
  feature_name: Create transparency knockout groups
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create transparency knockout groups to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create transparency knockout groups with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create transparency knockout groups
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/create-transparency-knockout-groups.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-view-transparency-in-artworks-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-view-transparency-in-artworks-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-view-transparency-in-artworks-html
  feature_name: View transparency in the artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use View transparency in the artwork to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named View transparency in the artwork with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / View transparency in the artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/view-transparency-in-artworks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blended-objects-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blended-objects-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blended-objects-overview-html
  feature_name: Blended objects overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blended objects overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Blended objects overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Blended objects overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/blended-objects-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blending-mode-types-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blending-mode-types-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blending-mode-types-html
  feature_name: Blending mode types
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blending mode types to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Blending mode types with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Blending mode types
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/blending-mode-types.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-change-the-blending-mode-in-artworks.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-change-the-blending-mode-in-artworks.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-change-the-blending-mode-in-artworks
  feature_name: Change and manage blending modes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change and manage blending modes to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Change and manage blending modes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Change and manage blending modes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/change-the-blending-mode-in-artworks.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-object-blends-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-object-blends-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-create-object-blends-html
  feature_name: Create blends
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create blends to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Create blends with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Create blends
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/create-object-blends.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blend-options-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blend-options-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-blend-options-overview-html
  feature_name: Blend Options overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blend Options overview to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Blend Options overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Blend Options overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/blend-options-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-modify-spine-of-blended-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-modify-spine-of-blended-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-modify-spine-of-blended-objects-html
  feature_name: Modify the spine of blended objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify the spine of blended objects to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Modify the spine of blended objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Modify the spine of blended objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/modify-spine-of-blended-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-reverse-stacking-order-in-blended-ob.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-reverse-stacking-order-in-blended-ob.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-reverse-stacking-order-in-blended-ob
  feature_name: Reverse the stacking order in blended objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reverse the stacking order in blended objects to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Reverse the stacking order in blended objects with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Reverse the stacking order in blended objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/reverse-stacking-order-in-blended-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-release-or-expand-blended-objects-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-release-or-expand-blended-objects-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-colors-apply-transparency-and-blending-release-or-expand-blended-objects-ht
  feature_name: Release or expand blended objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Release or expand blended objects to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Release or expand blended objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Release or expand blended objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-colors/apply-transparency-and-blending/release-or-expand-blended-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-or-remove-placeholder-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-or-remove-placeholder-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-or-remove-placeholder-text-html
  feature_name: Add or remove placeholder text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add or remove placeholder text to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Add or remove placeholder text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Add or remove placeholder text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/add-or-remove-placeholder-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-wrap-or-unwrap-text-around-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-wrap-or-unwrap-text-around-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-wrap-or-unwrap-text-around-objects-html
  feature_name: Wrap or unwrap text around objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Wrap or unwrap text around objects to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Wrap or unwrap text around objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Wrap or unwrap text around objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/wrap-or-unwrap-text-around-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-resize-the-text-area-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-resize-the-text-area-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-resize-the-text-area-html
  feature_name: Resize the text area
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resize the text area to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Resize the text area with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Resize the text area
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/resize-the-text-area.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-rows-and-columns-to-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-rows-and-columns-to-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-rows-and-columns-to-text-html
  feature_name: Add rows and columns to text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add rows and columns to text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Add rows and columns to text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add rows and columns to text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/add-rows-and-columns-to-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-margins-to-text-and-adjust-the-first-baseline.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-margins-to-text-and-adjust-the-first-baseline.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-margins-to-text-and-adjust-the-first-baseline
  feature_name: Add margins to text and adjust the first baseline
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add margins to text and adjust the first baseline to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Add margins to text and adjust the first baseline with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add margins to text and adjust the first baseline
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/add-margins-to-text-and-adjust-the-first-baseline.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-create-text-threads-between-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-create-text-threads-between-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-create-text-threads-between-objects-html
  feature_name: Create text threads between objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create text threads between objects to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Create text threads between objects with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Create text threads between objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/create-text-threads-between-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-remove-or-break-text-threads-between-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-remove-or-break-text-threads-between-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-remove-or-break-text-threads-between-objects-html
  feature_name: Remove or break text threads between objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Remove or break text threads between objects to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Remove or break text threads between objects with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Remove or break text threads between objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/remove-or-break-text-threads-between-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-text-to-vector-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-text-to-vector-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-text-to-vector-artwork-html
  feature_name: Add text to vector artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add text to vector artwork to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Add text to vector artwork with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add text to vector artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/add-text-to-vector-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-text-illustrator-text-tools-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-text-illustrator-text-tools-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-add-manage-text-add-text-illustrator-text-tools-html
  feature_name: Add text in Illustrator with Type, Area Type, and Touch Type tools
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add text in Illustrator with Type, Area Type, and Touch Type tools to author, style, shape, inspect, or export text behavior with explicit font
    dependencies.
  user_goal: A Studio operator can perform the source workflow named Add text in Illustrator with Type, Area Type, and Touch Type tools with Handshake-native commands,
    local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add text in Illustrator with Type, Area Type, and Touch Type tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/add-manage-text/add-text-illustrator-text-tools.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-font-options-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-font-options-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-font-options-overview-html
  feature_name: Font browser overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Font browser overview to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Font browser overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Font browser overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/font-options-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-apply-fonts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-apply-fonts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-apply-fonts-html
  feature_name: Find and apply fonts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find and apply fonts to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Find and apply fonts with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Find and apply fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/find-and-apply-fonts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-organize-fonts-using-creative-cloud-libraries-h.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-organize-fonts-using-creative-cloud-libraries-h.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-organize-fonts-using-creative-cloud-libraries-h
  feature_name: Organize fonts using Creative Cloud Libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Organize fonts using Creative Cloud Libraries to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Organize fonts using Creative Cloud Libraries with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Organize fonts using Creative Cloud Libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/organize-fonts-using-creative-cloud-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-apply-the-fonts-organized-in-creative-cloud-lib.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-apply-the-fonts-organized-in-creative-cloud-lib.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-apply-the-fonts-organized-in-creative-cloud-lib
  feature_name: Apply the fonts organized in Creative Cloud Libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply the fonts organized in Creative Cloud Libraries to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Apply the fonts organized in Creative Cloud Libraries with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Apply the fonts organized in Creative Cloud Libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/apply-the-fonts-organized-in-creative-cloud-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-apply-and-adjust-variable-fonts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-apply-and-adjust-variable-fonts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-apply-and-adjust-variable-fonts-html
  feature_name: Find, apply, and adjust variable fonts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find, apply, and adjust variable fonts to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Find, apply, and adjust variable fonts with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Find, apply, and adjust variable fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/find-apply-and-adjust-variable-fonts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-supported-fonts-in-illustrator-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-supported-fonts-in-illustrator-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-supported-fonts-in-illustrator-html
  feature_name: Supported font file types in Adobe Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Supported font file types in Adobe Illustrator to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Supported font file types in Adobe Illustrator with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Supported font file types in Adobe Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/supported-fonts-in-illustrator.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-edit-fonts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-edit-fonts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-edit-fonts-html
  feature_name: Edit fonts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit fonts to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Edit fonts with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Edit fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/edit-fonts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-replace-fonts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-replace-fonts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-find-and-replace-fonts-html
  feature_name: Find and replace fonts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find and replace fonts to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Find and replace fonts with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Find and replace fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/find-and-replace-fonts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-preview-add-or-replace-missing-fonts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-preview-add-or-replace-missing-fonts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-preview-add-or-replace-missing-fonts-html
  feature_name: Preview, add, or replace missing fonts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Preview, add, or replace missing fonts to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Preview, add, or replace missing fonts with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Preview, add, or replace missing fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/preview-add-or-replace-missing-fonts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-edit-text-without-replacing-missing-fonts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-edit-text-without-replacing-missing-fonts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-edit-text-without-replacing-missing-fonts-html
  feature_name: Edit text without replacing missing fonts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit text without replacing missing fonts to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Edit text without replacing missing fonts with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Edit text without replacing missing fonts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/edit-text-without-replacing-missing-fonts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-identify-fonts-using-retype-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-identify-fonts-using-retype-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-identify-fonts-using-retype-html
  feature_name: Identify fonts using Retype
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Identify fonts using Retype to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Identify fonts using Retype with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Identify fonts using Retype
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/identify-fonts-using-retype.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-apply-retype-suggested-fonts-to-live-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-apply-retype-suggested-fonts-to-live-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-apply-retype-suggested-fonts-to-live-text-html
  feature_name: Apply Retype-suggested fonts to live text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply Retype-suggested fonts to live text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Apply Retype-suggested fonts to live text with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Apply Retype-suggested fonts to live text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/apply-retype-suggested-fonts-to-live-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-about-adobe-asian-composers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-about-adobe-asian-composers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-about-adobe-asian-composers-html
  feature_name: About Adobe Asian composers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About Adobe Asian composers to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named About Adobe Asian composers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / About Adobe Asian composers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/about-adobe-asian-composers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-justify-text-using-kashida-and-hyphenation-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-justify-text-using-kashida-and-hyphenation-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-justify-text-using-kashida-and-hyphenation-html
  feature_name: Justify text using Kashida and hyphenation
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Justify text using Kashida and hyphenation to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Justify text using Kashida and hyphenation with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Justify text using Kashida and hyphenation
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/justify-text-using-kashida-and-hyphenation.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-format-japanese-text-mojikumi-kinsoku-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-format-japanese-text-mojikumi-kinsoku-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-format-japanese-text-mojikumi-kinsoku-html
  feature_name: Format Japanese text in Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Format Japanese text in Illustrator to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Format Japanese text in Illustrator with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Format Japanese text in Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/format-japanese-text-mojikumi-kinsoku.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-and-kinsoku-settings-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-and-kinsoku-settings-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-and-kinsoku-settings-html
  feature_name: Mojikumi and Kinsoku settings in Illustrator
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Mojikumi and Kinsoku settings in Illustrator to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Mojikumi and Kinsoku settings in Illustrator with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Mojikumi and Kinsoku settings in Illustrator
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/mojikumi-and-kinsoku-settings.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-kinsoku-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-kinsoku-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-fonts-and-scripts-mojikumi-kinsoku-overview-html
  feature_name: Mojikumi and Kinsoku overview for Japanese text formatting
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Mojikumi and Kinsoku overview for Japanese text formatting to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Mojikumi and Kinsoku overview for Japanese text formatting with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Mojikumi and Kinsoku overview for Japanese text formatting
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/fonts-and-scripts/mojikumi-kinsoku-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-character-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-character-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-character-panel-overview-html
  feature_name: Character panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Character panel overview to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Character panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Character panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/character-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-transform-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-transform-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-transform-text-html
  feature_name: Transform text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Transform text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Transform text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Transform text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/transform-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-underline-or-strike-through-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-underline-or-strike-through-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-underline-or-strike-through-text-html
  feature_name: Underline or strikethrough text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Underline or strikethrough text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Underline or strikethrough text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Underline or strikethrough text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/underline-or-strike-through-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-change-case-and-capitalization-styles-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-change-case-and-capitalization-styles-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-change-case-and-capitalization-styles-html
  feature_name: Change case and capitalization styles
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change case and capitalization styles to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Change case and capitalization styles with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Change case and capitalization styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/change-case-and-capitalization-styles.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-kerning-and-tracking-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-kerning-and-tracking-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-kerning-and-tracking-html
  feature_name: Adjust kerning and tracking
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust kerning and tracking to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Adjust kerning and tracking with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Adjust kerning and tracking
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/adjust-kerning-and-tracking.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-vary-font-height-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-vary-font-height-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-vary-font-height-html
  feature_name: Vary font height
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Vary font height to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Vary font height with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Vary font height
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/vary-font-height.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-turn-fractional-character-widths-off-or-on-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-turn-fractional-character-widths-off-or-on-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-turn-fractional-character-widths-off-or-on-html
  feature_name: Turn fractional character widths off or on
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Turn fractional character widths off or on to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Turn fractional character widths off or on with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Turn fractional character widths off or on
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/turn-fractional-character-widths-off-or-on.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-paragraph-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-paragraph-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-paragraph-panel-overview-html
  feature_name: Paragraph panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paragraph panel overview to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Paragraph panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Paragraph panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/paragraph-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-align-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-align-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-align-text-html
  feature_name: Align text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Align text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Align text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/align-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-word-and-letterspacing-in-justified-text.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-word-and-letterspacing-in-justified-text.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-word-and-letterspacing-in-justified-text
  feature_name: Adjust word and letterspacing in justified text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust word and letterspacing in justified text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Adjust word and letterspacing in justified text with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Adjust word and letterspacing in justified text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/adjust-word-and-letterspacing-in-justified-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-indent-text-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-indent-text-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-indent-text-html
  feature_name: Indent text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Indent text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Indent text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Indent text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/indent-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-paragraph-spacing-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-paragraph-spacing-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-paragraph-spacing-html
  feature_name: Adjust paragraph spacing
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust paragraph spacing to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Adjust paragraph spacing with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Adjust paragraph spacing
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/adjust-paragraph-spacing.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-create-bulleted-or-numbered-lists-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-create-bulleted-or-numbered-lists-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-create-bulleted-or-numbered-lists-html
  feature_name: Create bulleted or numbered lists
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create bulleted or numbered lists to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Create bulleted or numbered lists with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Create bulleted or numbered lists
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/create-bulleted-or-numbered-lists.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-move-or-flip-text-on-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-move-or-flip-text-on-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-move-or-flip-text-on-paths-html
  feature_name: Move or flip text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move or flip text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Move or flip text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Move or flip text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/move-or-flip-text-on-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-text-alignment-and-spacing-on-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-text-alignment-and-spacing-on-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-adjust-text-alignment-and-spacing-on-paths-html
  feature_name: Adjust text alignment and spacing on paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust text alignment and spacing on paths to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Adjust text alignment and spacing on paths with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Adjust text alignment and spacing on paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/adjust-text-alignment-and-spacing-on-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-apply-effects-to-text-on-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-apply-effects-to-text-on-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-edit-format-text-apply-effects-to-text-on-paths-html
  feature_name: Apply effects to text on paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply effects to text on paths to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Apply effects to text on paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Apply effects to text on paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/edit-format-text/apply-effects-to-text-on-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-about-character-sets-and-alternate-glyp.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-about-character-sets-and-alternate-glyp.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-about-character-sets-and-alternate-glyp
  feature_name: About character sets and alternate glyphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About character sets and alternate glyphs to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named About character sets and alternate glyphs with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / About character sets and alternate glyphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/about-character-sets-and-alternate-glyphs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-insert-special-characters-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-insert-special-characters-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-insert-special-characters-html
  feature_name: Insert special characters
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Insert special characters to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Insert special characters with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Insert special characters
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/insert-special-characters.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-glyphs-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-glyphs-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-glyphs-panel-overview-html
  feature_name: Glyphs panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Glyphs panel overview to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Glyphs panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Glyphs panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/glyphs-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-replace-characters-with-alternate-glyph.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-replace-characters-with-alternate-glyph.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-replace-characters-with-alternate-glyph
  feature_name: Replace characters with alternate glyphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Replace characters with alternate glyphs to preserve compatibility with existing creative file and asset workflows through explicit import/export
    diagnostics.
  user_goal: A Studio operator can perform the source workflow named Replace characters with alternate glyphs with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Replace characters with alternate glyphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/replace-characters-with-alternate-glyphs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-opentype-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-opentype-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-opentype-panel-overview-html
  feature_name: OpenType panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use OpenType panel overview to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named OpenType panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / OpenType panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/opentype-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-use-ligatures-and-contextual-alternates.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-use-ligatures-and-contextual-alternates.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-use-ligatures-and-contextual-alternates
  feature_name: Use ligatures and contextual alternates
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use ligatures and contextual alternates to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Use ligatures and contextual alternates with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Use ligatures and contextual alternates
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/use-ligatures-and-contextual-alternates.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-use-swashes-titling-alternates-or-styli.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-use-swashes-titling-alternates-or-styli.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-use-swashes-titling-alternates-or-styli
  feature_name: Use swashes, titling alternates, or stylistic alternates
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use swashes, titling alternates, or stylistic alternates to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Use swashes, titling alternates, or stylistic alternates with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Use swashes, titling alternates, or stylistic alternates
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/use-swashes-titling-alternates-or-stylistic-alternates.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-add-stylistic-sets-to-selected-text-htm.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-add-stylistic-sets-to-selected-text-htm.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-add-stylistic-sets-to-selected-text-htm
  feature_name: Add stylistic sets to selected text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add stylistic sets to selected text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Add stylistic sets to selected text with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Add stylistic sets to selected text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/add-stylistic-sets-to-selected-text.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-insert-white-space-and-break-characters.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-insert-white-space-and-break-characters.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-insert-white-space-and-break-characters
  feature_name: Insert white space and break characters
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Insert white space and break characters to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Insert white space and break characters with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Insert white space and break characters
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/insert-white-space-and-break-characters.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-to-glyph-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-to-glyph-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-to-glyph-options-html
  feature_name: Snap to Glyph options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Snap to Glyph options to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Snap to Glyph options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Snap to Glyph options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/snap-to-glyph-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-with-glyph-guides-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-with-glyph-guides-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-with-glyph-guides-html
  feature_name: Snap with glyph guides
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Snap with glyph guides to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Snap with glyph guides with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Snap with glyph guides
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/snap-with-glyph-guides.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-to-glyph-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-to-glyph-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-design-with-text-special-characters-glyphs-snap-to-glyph-html
  feature_name: Snap glyph to angles, anchor points, or text area
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Snap glyph to angles, anchor points, or text area to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Snap glyph to angles, anchor points, or text area with Handshake-native commands, local state,
    receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Snap glyph to angles, anchor points, or text area
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/design-with-text/special-characters-glyphs/snap-to-glyph.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-introduction-to-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-introduction-to-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-introduction-to-artboards-html
  feature_name: Introduction to artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Introduction to artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Introduction to artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Introduction to artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/introduction-to-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-add-new-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-add-new-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-add-new-artboards-html
  feature_name: Create and add new artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create and add new artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Create and add new artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Create and add new artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/add-new-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-select-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-select-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-select-artboards-html
  feature_name: Select artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Select artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Select artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/select-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-duplicate-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-duplicate-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-duplicate-artboards-html
  feature_name: Duplicate artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Duplicate artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Duplicate artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Duplicate artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/duplicate-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-resize-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-resize-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-resize-artboards-html
  feature_name: Resize artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Resize artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Resize artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Resize artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/resize-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-scale-artwork-with-artboard-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-scale-artwork-with-artboard-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-scale-artwork-with-artboard-html
  feature_name: Scale artwork with artboard
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale artwork with artboard to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Scale artwork with artboard with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Scale artwork with artboard
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/scale-artwork-with-artboard.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-rename-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-rename-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-rename-artboards-html
  feature_name: Rename artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rename artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Rename artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Rename artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/rename-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-delete-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-delete-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-add-edit-artboards-delete-artboards-html
  feature_name: Delete artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Delete artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Delete artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/add-edit-artboards/delete-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-use-artboard-context-menu-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-use-artboard-context-menu-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-use-artboard-context-menu-html
  feature_name: Use the artboard context menu
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use the artboard context menu to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Use the artboard context menu with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Use the artboard context menu
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/use-artboard-context-menu.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-cut-copy-and-paste-artboards-htm.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-cut-copy-and-paste-artboards-htm.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-cut-copy-and-paste-artboards-htm
  feature_name: Cut, copy, and paste artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Cut, copy, and paste artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Cut, copy, and paste artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Cut, copy, and paste artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/cut-copy-and-paste-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-move-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-move-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-move-artboards-html
  feature_name: Move artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Move artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Move artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/move-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-rearrange-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-rearrange-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-rearrange-artboards-html
  feature_name: Rearrange artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rearrange artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Rearrange artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Rearrange artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/rearrange-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-reorder-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-reorder-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-reorder-artboards-html
  feature_name: Reorder artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reorder artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Reorder artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Reorder artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/reorder-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-align-and-distribute-artboards-h.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-align-and-distribute-artboards-h.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-align-and-distribute-artboards-h
  feature_name: Align and distribute artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Align and distribute artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Align and distribute artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Align and distribute artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/align-and-distribute-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-lock-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-lock-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-lock-artboards-html
  feature_name: Lock artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Lock artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Lock artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Lock artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/lock-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-modify-display-settings-of-artbo.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-modify-display-settings-of-artbo.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-modify-display-settings-of-artbo
  feature_name: Modify display settings of artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Modify display settings of artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Modify display settings of artboards with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Modify display settings of artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/modify-display-settings-of-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-set-artboard-views-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-set-artboard-views-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-set-artboard-views-html
  feature_name: Set artboard views
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set artboard views to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Set artboard views with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Set artboard views
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/set-artboard-views.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-add-background-color-to-artboard.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-add-background-color-to-artboard.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-add-background-color-to-artboard
  feature_name: Apply colors to artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply colors to artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Apply colors to artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Apply colors to artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/add-background-color-to-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-set-video-display-options-for-ar.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-set-video-display-options-for-ar.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-set-video-display-options-for-ar
  feature_name: Set video display options for artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set video display options for artboards to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document
    graph.
  user_goal: A Studio operator can perform the source workflow named Set video display options for artboards with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Set video display options for artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/set-video-display-options-for-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-export-artboards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-export-artboards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-create-manage-artboards-organize-manage-artboards-export-artboards-html
  feature_name: Export selected artboards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export selected artboards to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Export selected artboards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Export selected artboards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/create-manage-artboards/organize-manage-artboards/export-artboards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-layers-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-layers-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-layers-overview-html
  feature_name: Layers overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Layers overview to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Layers overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Layers overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/layers-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-layers-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-layers-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-layers-panel-overview-html
  feature_name: Layers panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Layers panel overview to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Layers panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Layers panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/layers-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-change-the-display-of-the-layers-panel-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-change-the-display-of-the-layers-panel-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-change-the-display-of-the-layers-panel-ht
  feature_name: Change the display of the Layers panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change the display of the Layers panel to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Change the display of the Layers panel with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Change the display of the Layers panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/change-the-display-of-the-layers-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-add-layers-and-sublayers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-add-layers-and-sublayers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-add-layers-and-sublayers-html
  feature_name: Add layers and sublayers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add layers and sublayers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add layers and sublayers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add layers and sublayers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/add-layers-and-sublayers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-set-layer-and-sub-layer-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-set-layer-and-sub-layer-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-set-layer-and-sub-layer-options-html
  feature_name: Set layer and sublayer options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set layer and sublayer options to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Set layer and sublayer options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Set layer and sublayer options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/set-layer-and-sub-layer-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-move-objects-to-different-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-move-objects-to-different-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-move-objects-to-different-layers-html
  feature_name: Move objects to different layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Move objects to different layers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Move objects to different layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Move objects to different layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/move-objects-to-different-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-merge-and-flatten-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-merge-and-flatten-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-merge-and-flatten-layers-html
  feature_name: Merge and flatten layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioCollaborationSession
  primitive_domain: collaboration
  provider_posture: local_first_collaboration_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Merge and flatten layers to reproduce collaborative workflow behavior through local-first CRDT/EventLedger state, attribution, and recoverable
    receipts.
  user_goal: A Studio operator can perform the source workflow named Merge and flatten layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioCollaborationSession / Merge and flatten layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.collaboration.v0
  verification_refs:
  - needs_fixture.collaboration.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/merge-and-flatten-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-release-objects-to-separate-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-release-objects-to-separate-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-release-objects-to-separate-layers-html
  feature_name: Release objects to separate layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Release objects to separate layers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Release objects to separate layers with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Release objects to separate layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/release-objects-to-separate-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-locate-objects-in-the-layers-panel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-locate-objects-in-the-layers-panel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-locate-objects-in-the-layers-panel-html
  feature_name: Locate objects in the Layers panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Locate objects in the Layers panel to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Locate objects in the Layers panel with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Locate objects in the Layers panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/locate-objects-in-the-layers-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-find-and-filter-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-find-and-filter-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-create-and-organize-layers-find-and-filter-layers-html
  feature_name: Find and filter layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Find and filter layers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Find and filter layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Find and filter layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/create-and-organize-layers/find-and-filter-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-lock-or-unlock-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-lock-or-unlock-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-lock-or-unlock-layers-html
  feature_name: Lock or unlock layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Lock or unlock layers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Lock or unlock layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Lock or unlock layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/lock-and-hide-layers/lock-or-unlock-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-hide-or-show-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-hide-or-show-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-hide-or-show-layers-html
  feature_name: Hide or show layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Hide or show layers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Hide or show layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Hide or show layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/lock-and-hide-layers/hide-or-show-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-delete-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-delete-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-lock-and-hide-layers-delete-layers-html
  feature_name: Delete layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete layers to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Delete layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Delete layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/lock-and-hide-layers/delete-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-apply-and-modify-layer-effects-apply-effects-to-layers-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-apply-and-modify-layer-effects-apply-effects-to-layers-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-apply-and-modify-layer-effects-apply-effects-to-layers-html
  feature_name: Apply and modify effects to the layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply and modify effects to the layers to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Apply and modify effects to the layers with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Apply and modify effects to the layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/apply-and-modify-layer-effects/apply-effects-to-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-manage-layers-apply-and-modify-layer-effects-modify-or-delete-effects-in-layers-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-manage-layers-apply-and-modify-layer-effects-modify-or-delete-effects-in-layers-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-manage-layers-apply-and-modify-layer-effects-modify-or-delete-effects-in-layers-ht
  feature_name: Delete effects in layers
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete effects in layers to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Delete effects in layers with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Delete effects in layers
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/manage-layers/apply-and-modify-layer-effects/modify-or-delete-effects-in-layers.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-create-drop-shadows-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-create-drop-shadows-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-create-drop-shadows-html
  feature_name: Create drop shadows
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create drop shadows to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Create drop shadows with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Create drop shadows
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-filter-effects/create-drop-shadows.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-create-an-inner-or-outer-glow-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-create-an-inner-or-outer-glow-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-create-an-inner-or-outer-glow-html
  feature_name: Create an inner or outer glow
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create an inner or outer glow to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Create an inner or outer glow with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Create an inner or outer glow
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-filter-effects/create-an-inner-or-outer-glow.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-feather-object-edges-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-feather-object-edges-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-feather-object-edges-html
  feature_name: Feather object edges
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Feather object edges to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Feather object edges with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Feather object edges
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-filter-effects/feather-object-edges.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-apply-svg-filter-effects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-apply-svg-filter-effects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-apply-svg-filter-effects-html
  feature_name: Apply SVG filter effects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply SVG filter effects to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Apply SVG filter effects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Apply SVG filter effects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-filter-effects/apply-svg-filter-effects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-work-with-svg-interactivity-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-work-with-svg-interactivity-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-filter-effects-work-with-svg-interactivity-html
  feature_name: Work with SVG Interactivity
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with SVG Interactivity to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Work with SVG Interactivity with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Work with SVG Interactivity
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-filter-effects/work-with-svg-interactivity.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-vector-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-vector-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-vector-artwork-html
  feature_name: Create 3D vector artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create 3D vector artwork to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Create 3D vector artwork with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Create 3D vector artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/create-3d-vector-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-3d-materials-panel-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-3d-materials-panel-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-3d-materials-panel-options-html
  feature_name: 3D and Materials panel options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use 3D and Materials panel options to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named 3D and Materials panel options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / 3D and Materials panel options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/3d-materials-panel-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-map-artwork-on-3d-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-map-artwork-on-3d-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-map-artwork-on-3d-objects-html
  feature_name: Map artworks on 3D object
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Map artworks on 3D object to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Map artworks on 3D object with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Map artworks on 3D object
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/map-artwork-on-3d-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-add-artwork-to-the-3d-panel-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-add-artwork-to-the-3d-panel-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-add-artwork-to-the-3d-panel-html
  feature_name: Add artworks to the 3D panel
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add artworks to the 3D panel to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Add artworks to the 3D panel with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Add artworks to the 3D panel
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/add-artwork-to-the-3d-panel.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-render-3d-vector-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-render-3d-vector-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-render-3d-vector-artwork-html
  feature_name: Render 3D vector artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Render 3D vector artwork to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Render 3D vector artwork with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Render 3D vector artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/render-3d-vector-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-export-3d-vector-artwork-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-export-3d-vector-artwork-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-export-3d-vector-artwork-html
  feature_name: Export 3D vector artwork
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export 3D vector artwork to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Export 3D vector artwork with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Export 3D vector artwork
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/export-3d-vector-artwork.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-text-effects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-text-effects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-text-effects-html
  feature_name: Create 3D text effects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create 3D text effects to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Create 3D text effects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Create 3D text effects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/create-3d-text-effects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-create-3d-objects-html
  feature_name: Create 3D objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create 3D objects to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Create 3D objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Create 3D objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/create-3d-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-rotate-objects-in-3d-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-rotate-objects-in-3d-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-rotate-objects-in-3d-html
  feature_name: Rotate objects in 3D
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate objects in 3D to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Rotate objects in 3D with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Rotate objects in 3D
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/rotate-objects-in-3d.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-add-custom-bevel-paths-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-add-custom-bevel-paths-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-create-3d-graphics-add-custom-bevel-paths-html
  feature_name: Add custom bevel paths
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add custom bevel paths to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Add custom bevel paths with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Add custom bevel paths
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/create-3d-graphics/add-custom-bevel-paths.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-graphic-styles-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-graphic-styles-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-graphic-styles-panel-overview-html
  feature_name: Graphic Styles panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Graphic Styles panel overview to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Graphic Styles panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Graphic Styles panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-graphic-styles/graphic-styles-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-apply-graphic-styles-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-apply-graphic-styles-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-apply-graphic-styles-html
  feature_name: Set graphic appearance styles
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set graphic appearance styles to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Set graphic appearance styles with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Set graphic appearance styles
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-graphic-styles/apply-graphic-styles.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-work-with-graphic-style-libraries-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-work-with-graphic-style-libraries-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-special-effects-styles-apply-graphic-styles-work-with-graphic-style-libraries-html
  feature_name: Work with Graphic Style Libraries
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with Graphic Style Libraries to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Work with Graphic Style Libraries with Handshake-native commands, local state, receipts, and
    recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Work with Graphic Style Libraries
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/special-effects-styles/apply-graphic-styles/work-with-graphic-style-libraries.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-actions-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-actions-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-actions-panel-overview-html
  feature_name: Actions panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Actions panel overview to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Actions panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Actions panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/actions-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-record-actions-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-record-actions-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-record-actions-html
  feature_name: Create new actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create new actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Create new actions with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create new actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/record-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-insert-non-recordable-tasks-into-actions.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-insert-non-recordable-tasks-into-actions.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-insert-non-recordable-tasks-into-actions
  feature_name: Insert non-recordable tasks into actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Insert non-recordable tasks into actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Insert non-recordable tasks into actions with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Insert non-recordable tasks into actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/insert-non-recordable-tasks-into-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-insert-stops-in-actions-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-insert-stops-in-actions-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-insert-stops-in-actions-html
  feature_name: Insert stops in actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Insert stops in actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Insert stops in actions with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Insert stops in actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/insert-stops-in-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-exclude-commands-from-actions-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-exclude-commands-from-actions-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-exclude-commands-from-actions-html
  feature_name: Exclude commands from actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Exclude commands from actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Exclude commands from actions with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Exclude commands from actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/exclude-commands-from-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-specify-playback-speed-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-specify-playback-speed-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-specify-playback-speed-html
  feature_name: Specify playback speed
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Specify playback speed to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Specify playback speed with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Specify playback speed
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/specify-playback-speed.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-add-commands-to-actions-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-add-commands-to-actions-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-add-commands-to-actions-html
  feature_name: Add commands to actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add commands to actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add commands to actions with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add commands to actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/add-commands-to-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-re-record-actions-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-re-record-actions-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-re-record-actions-html
  feature_name: Re-record actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Re-record actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Re-record actions with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Re-record actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/re-record-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-manage-a-set-of-actions-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-manage-a-set-of-actions-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-manage-a-set-of-actions-html
  feature_name: Manage a set of actions
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Manage a set of actions to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Manage a set of actions with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Manage a set of actions
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/manage-a-set-of-actions.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-play-actions-on-a-batch-of-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-play-actions-on-a-batch-of-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-play-actions-on-a-batch-of-files-html
  feature_name: Play actions on a batch of files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Play actions on a batch of files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Play actions on a batch of files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Play actions on a batch of files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/play-actions-on-a-batch-of-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-batch-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-batch-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-batch-options-html
  feature_name: Batch options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Batch options to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Batch options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Batch options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/batch-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-install-and-run-scripts-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-install-and-run-scripts-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-install-and-run-scripts-html
  feature_name: Install and run scripts
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Install and run scripts to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Install and run scripts with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Install and run scripts
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/install-and-run-scripts.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-merge-data-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-merge-data-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-merge-data-html
  feature_name: Merge data
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioCollaborationSession
  primitive_domain: collaboration
  provider_posture: local_first_collaboration_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Merge data to reproduce collaborative workflow behavior through local-first CRDT/EventLedger state, attribution, and recoverable receipts.
  user_goal: A Studio operator can perform the source workflow named Merge data with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioCollaborationSession / Merge data
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.collaboration.v0
  verification_refs:
  - needs_fixture.collaboration.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/merge-data.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-set-up-data-source-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-set-up-data-source-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-set-up-data-source-files-html
  feature_name: Set up data source files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Set up data source files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Set up data source files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Set up data source files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/set-up-data-source-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-import-data-source-files-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-import-data-source-files-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-import-data-source-files-html
  feature_name: Import data source files
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: import
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Import data source files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Import data source files with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Import data source files
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/import-data-source-files.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-variable-panel-overview-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-variable-panel-overview-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-variable-panel-overview-html
  feature_name: Variable panel overview
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Variable panel overview to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Variable panel overview with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Variable panel overview
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/variable-panel-overview.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-work-with-variables-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-work-with-variables-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-work-with-variables-html
  feature_name: Work with variables
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioStyleRegistry
  primitive_domain: style_system
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with variables to define, apply, publish, or update reusable styles, components, variables, and libraries inside Studio.
  user_goal: A Studio operator can perform the source workflow named Work with variables with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioStyleRegistry / Work with variables
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.style-system.v0
  verification_refs:
  - needs_fixture.style-system.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/work-with-variables.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-edit-dynamic-objects-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-edit-dynamic-objects-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-automate-actions-edit-dynamic-objects-html
  feature_name: Edit dynamic objects
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Edit dynamic objects to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Edit dynamic objects with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Edit dynamic objects
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/automate-actions/edit-dynamic-objects.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-data-sets-and-label-options-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-data-sets-and-label-options-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-data-sets-and-label-options-html
  feature_name: Data sets and label options
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Data sets and label options to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Data sets and label options with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Data sets and label options
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/data-sets-and-label-options.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-create-graphs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-create-graphs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-create-graphs-html
  feature_name: Create graphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Create graphs to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Create graphs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Create graphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/create-graphs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-graph-data-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-graph-data-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-graph-data-html
  feature_name: Add graph data
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add graph data to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add graph data with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add graph data
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/add-graph-data.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-graph-labels-and-data-sets-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-graph-labels-and-data-sets-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-graph-labels-and-data-sets-html
  feature_name: Add graph labels and data sets
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add graph labels and data sets to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add graph labels and data sets with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add graph labels and data sets
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/add-graph-labels-and-data-sets.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-adjust-column-width-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-adjust-column-width-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-adjust-column-width-html
  feature_name: Adjust decimal digits and column width
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Adjust decimal digits and column width to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Adjust decimal digits and column width with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Adjust decimal digits and column width
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/adjust-column-width.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-columns-bars-and-lines-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-columns-bars-and-lines-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-columns-bars-and-lines-html
  feature_name: Format columns, bars, and lines
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Format columns, bars, and lines to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Format columns, bars, and lines with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Format columns, bars, and lines
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/format-columns-bars-and-lines.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-select-parts-of-a-graph-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-select-parts-of-a-graph-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-select-parts-of-a-graph-html
  feature_name: Select parts of a graph
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Select parts of a graph as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Select parts of a graph with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Select parts of a graph
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/select-parts-of-a-graph.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-graph-types-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-graph-types-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-graph-types-html
  feature_name: Change graph types
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change graph types to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Change graph types with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Change graph types
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/change-graph-types.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-graph-value-axes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-graph-value-axes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-graph-value-axes-html
  feature_name: Change graph value axes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change graph value axes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Change graph value axes with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Change graph value axes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/change-graph-value-axes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-assign-different-scales-to-value-axes-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-assign-different-scales-to-value-axes-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-assign-different-scales-to-value-axes-html
  feature_name: Assign different scales to value axes
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Assign different scales to value axes to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Assign different scales to value axes with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Assign different scales to value axes
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/assign-different-scales-to-value-axes.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-the-position-of-legends-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-the-position-of-legends-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-change-the-position-of-legends-html
  feature_name: Change the position of legend in graphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Change the position of legend in graphs to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool
    surface.
  user_goal: A Studio operator can perform the source workflow named Change the position of legend in graphs with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Change the position of legend in graphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/change-the-position-of-legends.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-combine-different-graph-types-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-combine-different-graph-types-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-combine-different-graph-types-html
  feature_name: Combine different graph types
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Combine different graph types to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Combine different graph types with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Combine different graph types
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/combine-different-graph-types.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-drop-shadows-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-drop-shadows-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-add-drop-shadows-html
  feature_name: Add drop shadows to graphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add drop shadows to graphs to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Add drop shadows to graphs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Add drop shadows to graphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/add-drop-shadows.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-apply-marker-designs-to-graphs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-apply-marker-designs-to-graphs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-apply-marker-designs-to-graphs-html
  feature_name: Apply marker designs to graphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Apply marker designs to graphs to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Apply marker designs to graphs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Apply marker designs to graphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/apply-marker-designs-to-graphs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-pie-graphs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-pie-graphs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-pie-graphs-html
  feature_name: Format pie graphs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Format pie graphs to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Format pie graphs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Format pie graphs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/format-pie-graphs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-the-text-in-graphs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-the-text-in-graphs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-format-the-text-in-graphs-html
  feature_name: Format graph text
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Format graph text to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Format graph text with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Format graph text
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/format-the-text-in-graphs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-reuse-graph-designs-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-reuse-graph-designs-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-automate-visualize-data-visualize-data-reuse-graph-designs-html
  feature_name: Reuse graph designs
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reuse graph designs to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Reuse graph designs with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Reuse graph designs
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/automate-visualize-data/visualize-data/reuse-graph-designs.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-to-cloud-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-to-cloud-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-to-cloud-html
  feature_name: Export to Adobe cloud storage
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export to Adobe cloud storage to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Export to Adobe cloud storage with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Export to Adobe cloud storage
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/save-and-export/export-files-to-different-formats/export-to-cloud.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-for-screens-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-for-screens-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-save-and-export-export-files-to-different-formats-export-for-screens-html
  feature_name: Export for screens
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export for screens to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Export for screens with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Export for screens
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/save-and-export/export-files-to-different-formats/export-for-screens.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-save-and-export-export-to-other-apps-export-assets-to-firefly-boards-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-save-and-export-export-to-other-apps-export-assets-to-firefly-boards-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-save-and-export-export-to-other-apps-export-assets-to-firefly-boards-html
  feature_name: Export assets to Firefly Boards
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: export
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Export assets to Firefly Boards as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in
    the core.
  user_goal: A Studio operator can perform the source workflow named Export assets to Firefly Boards with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Export assets to Firefly Boards
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/save-and-export/export-to-other-apps/export-assets-to-firefly-boards.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-about-using-ai-tools-with-illustrator-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-about-using-ai-tools-with-illustrator-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-about-using-ai-tools-with-illustrator-html
  feature_name: About using desktop AI tools with Adobe Illustrator (Beta)
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use About using desktop AI tools with Adobe Illustrator (Beta) as a provider-neutral or local-model-assisted Studio workflow with explicit receipts
    and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named About using desktop AI tools with Adobe Illustrator (Beta) with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / About using desktop AI tools with Adobe Illustrator (Beta)
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/connect-with-other-apps-and-tools/about-using-ai-tools-with-illustrator.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-connect-illustrator-to-ai-tools-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-connect-illustrator-to-ai-tools-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-connect-illustrator-to-ai-tools-html
  feature_name: Connect Adobe Illustrator (Beta) to AI tools
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Connect Adobe Illustrator (Beta) to AI tools as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud
    dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Connect Adobe Illustrator (Beta) to AI tools with Handshake-native commands, local state, receipts,
    and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Connect Adobe Illustrator (Beta) to AI tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/connect-with-other-apps-and-tools/connect-illustrator-to-ai-tools.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-work-with-illustrator-documents-from-ai-tools-ht.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-work-with-illustrator-documents-from-ai-tools-ht.v0
  source_feature_id: illustrator.desktop.leaf.desktop-connect-with-other-apps-and-tools-work-with-illustrator-documents-from-ai-tools-ht
  feature_name: Work with Adobe Illustrator (Beta) documents from AI tools
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Work with Adobe Illustrator (Beta) documents from AI tools as a provider-neutral or local-model-assisted Studio workflow with explicit receipts
    and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Work with Adobe Illustrator (Beta) documents from AI tools with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Work with Adobe Illustrator (Beta) documents from AI tools
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/connect-with-other-apps-and-tools/work-with-illustrator-documents-from-ai-tools.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-ai-assistant-about-using-ai-assistant-in-illustrator-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-ai-assistant-about-using-ai-assistant-in-illustrator-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-ai-assistant-about-using-ai-assistant-in-illustrator-html
  feature_name: Get started with AI Assistant in Adobe Illustrator (Beta)
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Get started with AI Assistant in Adobe Illustrator (Beta) as a provider-neutral or local-model-assisted Studio workflow with explicit receipts
    and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Get started with AI Assistant in Adobe Illustrator (Beta) with Handshake-native commands, local
    state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Get started with AI Assistant in Adobe Illustrator (Beta)
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-ai-assistant/about-using-ai-assistant-in-illustrator.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.desktop-use-ai-assistant-automate-tasks-using-ai-assistant-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.desktop-use-ai-assistant-automate-tasks-using-ai-assistant-html.v0
  source_feature_id: illustrator.desktop.leaf.desktop-use-ai-assistant-automate-tasks-using-ai-assistant-html
  feature_name: Use AI Assistant to complete production tasks in Illustrator (Beta)
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: illustrator
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Use AI Assistant to complete production tasks in Illustrator (Beta) as a provider-neutral or local-model-assisted Studio workflow with explicit
    receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Use AI Assistant to complete production tasks in Illustrator (Beta) with Handshake-native commands,
    local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Use AI Assistant to complete production tasks in Illustrator (Beta)
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: adobe-illustrator-desktop-jina.md
    path: _source_snapshots/adobe-illustrator-desktop-jina.md
    url: https://helpx.adobe.com/illustrator/desktop/use-ai-assistant/automate-tasks-using-ai-assistant.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-selection-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-selection-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-selection-tool-html
  feature_name: Selection
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Selection as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Selection with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/selection-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-direct-selection-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-direct-selection-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-direct-selection-tool-html
  feature_name: Direct Selection
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Direct Selection as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Direct Selection with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Direct Selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/direct-selection-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-group-selection-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-group-selection-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-group-selection-tool-html
  feature_name: Group Selection
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioSelectionSet
  primitive_domain: selection
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Group Selection as a source-backed Studio feature candidate with local-first Rust behavior.
  user_goal: A Studio operator can perform the source workflow named Group Selection with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioSelectionSet / Group Selection
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.selection.v0
  verification_refs:
  - needs_fixture.selection.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/group-selection-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-magic-wand-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-magic-wand-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-magic-wand-tool-html
  feature_name: Magic Wand
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Magic Wand to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Magic Wand with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Magic Wand
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/magic-wand-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-lasso-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-lasso-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-lasso-tool-html
  feature_name: Lasso
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Lasso to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Lasso with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Lasso
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/lasso-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-artboard-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-artboard-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-artboard-tool-html
  feature_name: Artboard
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioPageSpread
  primitive_domain: page_layout
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Artboard to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph.
  user_goal: A Studio operator can perform the source workflow named Artboard with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioPageSpread / Artboard
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.page-layout.v0
  verification_refs:
  - needs_fixture.page-layout.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/artboard-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-hand-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-hand-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-hand-tool-html
  feature_name: Hand
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Hand to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Hand with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Hand
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/hand-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-rotate-view-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-rotate-view-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-rotate-view-tool-html
  feature_name: Rotate View
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate View to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Rotate View with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Rotate View
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/rotate-view-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-zoom-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-zoom-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-zoom-tool-html
  feature_name: Zoom
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioWorkspaceSurface
  primitive_domain: workspace
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Zoom to make workspace, preference, navigation, and diagnostic behavior predictable for operators and models.
  user_goal: A Studio operator can perform the source workflow named Zoom with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioWorkspaceSurface / Zoom
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.workspace.v0
  verification_refs:
  - needs_fixture.workspace.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/zoom-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-gradient-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-gradient-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-gradient-tool-html
  feature_name: Gradient
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Gradient to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Gradient with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Gradient
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/gradient-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-mesh-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-mesh-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-mesh-tool-html
  feature_name: Mesh
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Mesh to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Mesh with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Mesh
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/mesh-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-shape-builder-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-shape-builder-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-shape-builder-tool-html
  feature_name: Shape Builder
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Shape Builder to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Shape Builder with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Shape Builder
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/shape-builder-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-type-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-type-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-type-tool-html
  feature_name: Type
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Type to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Type with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Type
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/type-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-type-on-path-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-type-on-path-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-type-on-path-tool-html
  feature_name: Type on a Path
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Type on a Path to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Type on a Path with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Type on a Path
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/type-on-path-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-vertical-type-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-vertical-type-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-vertical-type-tool-html
  feature_name: Vertical Type
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioTextRunAndStory
  primitive_domain: typography
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Vertical Type to author, style, shape, inspect, or export text behavior with explicit font dependencies.
  user_goal: A Studio operator can perform the source workflow named Vertical Type with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioTextRunAndStory / Vertical Type
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.typography.v0
  verification_refs:
  - needs_fixture.typography.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/vertical-type-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-pen-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-pen-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-pen-tool-html
  feature_name: Pen
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Pen to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Pen with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Pen
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/pen-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-add-anchor-point-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-add-anchor-point-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-add-anchor-point-tool-html
  feature_name: Add Anchor Point
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Add Anchor Point to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Add Anchor Point with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Add Anchor Point
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/add-anchor-point-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-delete-anchor-point-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-delete-anchor-point-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-delete-anchor-point-tool-html
  feature_name: Delete Anchor Point
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Delete Anchor Point to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Delete Anchor Point with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Delete Anchor Point
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/delete-anchor-point-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-anchor-point-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-anchor-point-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-anchor-point-tool-html
  feature_name: Anchor Point
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Anchor Point to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Anchor Point with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Anchor Point
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/anchor-point-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-curvature-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-curvature-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-curvature-tool-html
  feature_name: Curvature
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Curvature to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Curvature with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Curvature
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/curvature-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-line-segment-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-line-segment-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-line-segment-tool-html
  feature_name: Line Segment
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Line Segment to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Line Segment with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Line Segment
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/line-segment-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-rectangle-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-rectangle-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-rectangle-tool-html
  feature_name: Rectangle
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rectangle to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rectangle with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rectangle
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/rectangle-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-rounded-rectangle-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-rounded-rectangle-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-rounded-rectangle-tool-html
  feature_name: Rounded Rectangle
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rounded Rectangle to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rounded Rectangle with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rounded Rectangle
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/rounded-rectangle-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-ellipse-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-ellipse-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-ellipse-tool-html
  feature_name: Ellipse
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Ellipse to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Ellipse with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Ellipse
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/ellipse-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-polygon-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-polygon-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-polygon-tool-html
  feature_name: Polygon
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Polygon to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Polygon with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Polygon
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/polygon-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-star-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-star-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-star-tool-html
  feature_name: Star
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Star to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Star with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Star
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/star-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-paintbrush-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-paintbrush-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-paintbrush-tool-html
  feature_name: Paintbrush
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioModelToolContract
  primitive_domain: ai
  provider_posture: provider_adapter_or_local_model_candidate
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Paintbrush as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core.
  user_goal: A Studio operator can perform the source workflow named Paintbrush with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioModelToolContract / Paintbrush
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.ai.v0
  verification_refs:
  - needs_fixture.ai.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/paintbrush-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-blob-brush-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-blob-brush-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-blob-brush-tool-html
  feature_name: Blob Brush
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blob Brush to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Blob Brush with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Blob Brush
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/blob-brush-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-pencil-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-pencil-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-pencil-tool-html
  feature_name: Pencil
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Pencil to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Pencil with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Pencil
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/pencil-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-shaper-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-shaper-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-shaper-tool-html
  feature_name: Shaper
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Shaper to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Shaper with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Shaper
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/shaper-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-slice-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-slice-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-slice-tool-html
  feature_name: Slice
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Slice to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Slice with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Slice
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/slice-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-rotate-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-rotate-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-rotate-tool-html
  feature_name: Rotate
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Rotate to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Rotate with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Rotate
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/rotate-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-reflect-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-reflect-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-reflect-tool-html
  feature_name: Reflect
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Reflect to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Reflect with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Reflect
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/reflect-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-scale-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-scale-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-scale-tool-html
  feature_name: Scale
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scale to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Scale with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Scale
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/scale-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-shear-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-shear-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-shear-tool-html
  feature_name: Shear
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Shear to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Shear with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Shear
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/shear-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-width-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-width-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-width-tool-html
  feature_name: Width
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Width to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Width with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Width
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/width-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-free-transform-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-free-transform-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-free-transform-tool-html
  feature_name: Free Transform
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Free Transform to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Free Transform with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Free Transform
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/free-transform-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-eyedropper-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-eyedropper-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-eyedropper-tool-html
  feature_name: Eyedropper
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Eyedropper to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Eyedropper with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Eyedropper
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/eyedropper-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-blend-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-blend-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-blend-tool-html
  feature_name: Blend
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioColorPipeline
  primitive_domain: color
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Blend to control fills, color, gradients, effects, blends, profiles, or appearance state in Studio.
  user_goal: A Studio operator can perform the source workflow named Blend with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioColorPipeline / Blend
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.color.v0
  verification_refs:
  - needs_fixture.color.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/blend-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-eraser-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-eraser-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-eraser-tool-html
  feature_name: Eraser
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Eraser to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Eraser with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Eraser
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/eraser-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-scissors-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-scissors-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-scissors-tool-html
  feature_name: Scissors
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Scissors to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Scissors with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Scissors
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/scissors-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.using-tool-techniques-dimension-tool-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.using-tool-techniques-dimension-tool-html.v0
  source_feature_id: illustrator.desktop.leaf.using-tool-techniques-dimension-tool-html
  feature_name: Dimension
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: tool_techniques
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioVectorPathGraph
  primitive_domain: vector
  provider_posture: local_primitive
  file_format_compatibility: not_applicable
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Dimension to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface.
  user_goal: A Studio operator can perform the source workflow named Dimension with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioVectorPathGraph / Dimension
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.vector.v0
  verification_refs:
  - needs_fixture.vector.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/using/tool-techniques/dimension-tool.html
- source_distilled_feature_id: osd.illustrator.illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html.v1
  source_ids:
  - ROWS-S01
  - ROWS-S02
  feature_use_card_id: fuc.illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html.v0
  source_feature_id: illustrator.desktop.leaf.kb-supported-file-formats-illustrator-html
  feature_name: Supported file formats
  source_apps:
  - Illustrator desktop
  source_inventory: 22-illustrator-leaf-index.md
  source_category: supported_file_formats
  source_domain_ledger: 36-illustrator-source-distilled-domain-ledger.md
  feature_kind: source_help_leaf_or_category_feature
  studio_surface: StudioFileIO
  primitive_domain: file_io
  provider_posture: compatibility_shim
  file_format_compatibility: fixture_required
  naming_posture: handshake_native_name_with_vendor_source_refs
  app_behavior: Use Supported file formats to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics.
  user_goal: A Studio operator can perform the source workflow named Supported file formats with Handshake-native commands, local state, receipts, and recovery.
  implementation_readiness: needs_command_contract_promotion
  manual_topic_candidate: Studio / StudioFileIO / Supported file formats
  manual_required_when: same_change_as_product_behavior_implementation
  command_contract_refs:
  - needs_contract.file-io.v0
  verification_refs:
  - needs_fixture.file-io.v0
  source_confidence: online_source_distilled_from_feature_use_card
  source_refs:
  - label: illustrator-tools-jina.md
    path: _source_snapshots/illustrator-tools-jina.md
    url: https://helpx.adobe.com/illustrator/kb/supported-file-formats-illustrator.html
```

</topic>

<topic id="sources" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Sources for this generated row ledger.">

### [SFR-ILLUSTRATOR-SOURCE-DISTILLED-FEATURE-ROWS.sources] Sources

```yaml
sources:
- id: ROWS-S01
  path: 24-illustrator-feature-use-cards.md
  note: Generated Feature Use Cards used as row source.
- id: ROWS-S02
  path: 36-illustrator-source-distilled-domain-ledger.md
  note: Online-source-distilled domain ledger used as row context.
- id: ROWS-S03
  path: 33-online-source-distilled-feature-ledger.md
  note: Canonical source-distilled merge record.
```

</topic>
