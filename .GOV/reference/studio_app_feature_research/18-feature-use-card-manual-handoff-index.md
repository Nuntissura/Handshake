---
file_id: "sfr-feature-use-card-manual-handoff-index"
file_kind: "studio_user_manual_handoff_index"
updated_at: "2026-07-05"
status: "generated_from_feature_use_cards"
total_feature_use_cards: 2730
---

<topic id="manual-handoff-summary" status="current" version="0.2" updated_at="2026-07-05" ingestable="true" summary="Counts and manual handoff posture for generated Feature Use Cards.">

# Feature Use Card Manual Handoff Index

This index groups all generated Feature Use Cards by intended Studio manual surface. It does not create product manual authority by itself. When a feature is implemented, the same product change must add or update the internal Studio UserManual topic and link the command receipt, diagnostics, and verification proof.

```yaml
total_feature_use_cards: 2730
app_counts:
  affinity: 1032
  figma: 200
  illustrator: 515
  indesign: 542
  photoshop: 441
manual_entry_status:
  planning_only: 2730
  implemented_manual_topics: 0
coverage_status: "all_current_stable_leaf_feature_ids_have_generated_feature_use_cards_for_photoshop_affinity_indesign_illustrator_figma"
residual_gap: "Generated cards are TOC-inferred until exact source pages/app behavior are inspected during command-contract promotion."
source_count_note: "Affinity desktop has 1,035 raw rows and 1,032 stable unique feature IDs after duplicate key-features rows are collapsed; Illustrator has 515 generated cards from 532 parsed leaves; Figma has 200 generated cards from current Design/Make/import/export/API snapshots plus verified category/source-agent evidence."
```

</topic>

<topic id="manual-topic-groups" status="current" version="0.2" updated_at="2026-07-05" ingestable="true" summary="Manual topic groups and source card files.">

```yaml
manual_topic_groups:
  - studio_surface: "StudioActionGraph"
    feature_use_card_count: 2
    app_counts:
      figma: 2
    source_card_files: ["25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioCollaborationSession"
    feature_use_card_count: 3
    app_counts:
      figma: 1
      illustrator: 2
    source_card_files: ["24-illustrator-feature-use-cards.md", "25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioColorPipeline"
    feature_use_card_count: 164
    app_counts:
      affinity: 19
      illustrator: 65
      indesign: 58
      photoshop: 22
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioExportRecipe"
    feature_use_card_count: 139
    app_counts:
      indesign: 78
      photoshop: 61
    source_card_files: ["15-photoshop-feature-use-cards.md", "17-indesign-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioFileIO"
    feature_use_card_count: 220
    app_counts:
      figma: 174
      illustrator: 46
    source_card_files: ["24-illustrator-feature-use-cards.md", "25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioInteractiveDocumentSurface"
    feature_use_card_count: 219
    app_counts:
      figma: 2
      illustrator: 1
      indesign: 147
      photoshop: 69
    source_card_files: ["15-photoshop-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md", "25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioLayerGraph"
    feature_use_card_count: 213
    app_counts:
      affinity: 112
      indesign: 16
      photoshop: 85
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioModelToolContract"
    feature_use_card_count: 134
    app_counts:
      figma: 18
      illustrator: 93
      indesign: 5
      photoshop: 18
    source_card_files: ["15-photoshop-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md", "25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioPageSpread"
    feature_use_card_count: 661
    app_counts:
      affinity: 549
      figma: 2
      illustrator: 28
      indesign: 68
      photoshop: 14
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md", "25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioRasterPipeline"
    feature_use_card_count: 175
    app_counts:
      affinity: 87
      indesign: 12
      photoshop: 76
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioRawDevelopRecipe"
    feature_use_card_count: 24
    app_counts:
      affinity: 24
    source_card_files: ["16-affinity-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioSelectionSet"
    feature_use_card_count: 112
    app_counts:
      affinity: 41
      illustrator: 19
      indesign: 3
      photoshop: 49
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioStyleRegistry"
    feature_use_card_count: 17
    app_counts:
      illustrator: 17
    source_card_files: ["24-illustrator-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioTableFrame"
    feature_use_card_count: 10
    app_counts:
      indesign: 10
    source_card_files: ["17-indesign-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioTextRunAndStory"
    feature_use_card_count: 308
    app_counts:
      affinity: 110
      illustrator: 57
      indesign: 117
      photoshop: 24
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioVectorPathGraph"
    feature_use_card_count: 319
    app_counts:
      affinity: 90
      figma: 1
      illustrator: 177
      indesign: 28
      photoshop: 23
    source_card_files: ["15-photoshop-feature-use-cards.md", "16-affinity-feature-use-cards.md", "17-indesign-feature-use-cards.md", "24-illustrator-feature-use-cards.md", "25-figma-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
  - studio_surface: "StudioWorkspaceSurface"
    feature_use_card_count: 10
    app_counts:
      illustrator: 10
    source_card_files: ["24-illustrator-feature-use-cards.md"]
    required_user_manual_topic_status: "create_or_update_when_feature_is_implemented"
```

</topic>

<topic id="handoff-rules" status="current" version="0.2" updated_at="2026-07-05" ingestable="true" summary="Promotion rules from planning cards into the internal Studio UserManual.">

```yaml
handoff_rules:
  - id: "FUC-HANDOFF-001"
    rule: "Do not treat a generated Feature Use Card as implemented behavior."
    required_before_manual_promotion: [exact_source_page_or_app_behavior_inspection, handshake_native_name, typed_rust_command_contract, fixtures_or_interaction_tests, command_receipt_schema, diagnostics, undo_replay_behavior]
  - id: "FUC-HANDOFF-002"
    rule: "Every implemented Studio feature must add or update an internal Studio UserManual topic in the same change."
    required_manual_content: [purpose, when_to_use, workflow, options, expected_result, mistakes, edge_cases, recovery_steps, command_id, diagnostics, examples]
  - id: "FUC-HANDOFF-003"
    rule: "Vendor product names stay in source references, migration notes, and compatibility fixtures only."
    required_product_posture: "Handshake-native names for shipped tools, panels, command IDs, and manual topics."
  - id: "FUC-HANDOFF-004"
    rule: "File compatibility features must target existing formats and include round-trip expectations, unsupported-feature diagnostics, and recovery behavior."
    applies_to_surfaces: [StudioFileIO, StudioExportRecipe, StudioPreflightProfile]
```

</topic>

<topic id="sources" status="current" version="0.2" updated_at="2026-07-05" ingestable="true" summary="Source card files used to generate this handoff index.">

```yaml
sources:
  - { id: "FUC-SRC-PHOTOSHOP", path: "15-photoshop-feature-use-cards.md", note: "Generated photoshop Feature Use Cards." }
  - { id: "FUC-SRC-AFFINITY", path: "16-affinity-feature-use-cards.md", note: "Generated affinity Feature Use Cards." }
  - { id: "FUC-SRC-INDESIGN", path: "17-indesign-feature-use-cards.md", note: "Generated indesign Feature Use Cards." }
  - { id: "FUC-SRC-ILLUSTRATOR", path: "24-illustrator-feature-use-cards.md", note: "Generated illustrator Feature Use Cards." }
  - { id: "FUC-SRC-FIGMA", path: "25-figma-feature-use-cards.md", note: "Generated figma Feature Use Cards." }
```

</topic>
