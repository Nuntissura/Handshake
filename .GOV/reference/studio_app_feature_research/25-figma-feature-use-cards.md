---
file_id: "25-figma-feature-use-cards"
file_kind: "studio_app_feature_use_cards"
updated_at: "2026-07-05"
app_family: "Figma"
source_inventory: "23-figma-leaf-index.md"
card_count: 200
status: "generated_full_coverage_from_leaf_inventory"
---

<topic id="feature-use-card-coverage" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Generated Feature Use Cards for stable source-backed leaves.">

# Figma Feature Use Cards

```yaml
feature_use_card_count: 200
explanation_status: "toc_inferred_from_verified_help_inventory"
manual_entry_status: "planning_only_until_implemented"
local_first_posture: "built_in_no_cloud_required_rust_forward"
```

</topic>

<topic id="generated-feature-use-cards" status="current" version="0.1" updated_at="2026-07-05" ingestable="true" summary="Machine-readable generated Feature Use Card records.">

```yaml
feature_use_cards:
  - feature_use_card_id: "fuc.figma.platform.leaf.developers.v0"
    source_feature_id: "figma.platform.leaf.developers"
    feature_name: "Developer docs Work with APIs, embeds, and more"
    source_apps: ["Figma Developer Platform"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_developer_platform"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Developer docs Work with APIs, embeds, and more to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Developer docs Work with APIs, embeds, and more with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_developer_platform"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Developer docs Work with APIs, embeds, and more with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Developer Platform"
        feature_id: "figma.platform.leaf.developers"
        feature_name: "Developer docs Work with APIs, embeds, and more"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Developer docs Work with APIs, embeds, and more"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Developer docs Work with APIs, embeds, and more inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://www.figma.com/developers"
  - feature_use_card_id: "fuc.figma.platform.leaf.15297425105303-explore-design-files.v0"
    source_feature_id: "figma.platform.leaf.15297425105303-explore-design-files"
    feature_name: "Explore design files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Explore design files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Explore design files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Explore design files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15297425105303-explore-design-files"
        feature_name: "Explore design files"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Explore design files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Explore design files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15297425105303-Explore-design-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.37998629035799-work-with-the-figma-agent-in-design-files.v0"
    source_feature_id: "figma.platform.leaf.37998629035799-work-with-the-figma-agent-in-design-files"
    feature_name: "Work with the Figma agent in design files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Work with the Figma agent in design files as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Work with the Figma agent in design files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Work with the Figma agent in design files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.37998629035799-work-with-the-figma-agent-in-design-files"
        feature_name: "Work with the Figma agent in design files"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Work with the Figma agent in design files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Work with the Figma agent in design files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/37998629035799-Work-with-the-Figma-agent-in-design-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
    feature_name: "Use AI tools in Figma Design"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use AI tools in Figma Design as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Use AI tools in Figma Design with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Use AI tools in Figma Design with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
        feature_name: "Use AI tools in Figma Design"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Use AI tools in Figma Design"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use AI tools in Figma Design inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/23870272542231-Use-AI-tools-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041064814-change-the-background-color-of-the-canvas.v0"
    source_feature_id: "figma.platform.leaf.360041064814-change-the-background-color-of-the-canvas"
    feature_name: "Change the background color of the canvas"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Change the background color of the canvas to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Change the background color of the canvas with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Change the background color of the canvas with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041064814-change-the-background-color-of-the-canvas"
        feature_name: "Change the background color of the canvas"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Change the background color of the canvas"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Change the background color of the canvas inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041064814-Change-the-background-color-of-the-canvas"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041064174-access-design-tools-from-the-toolbar.v0"
    source_feature_id: "figma.platform.leaf.360041064174-access-design-tools-from-the-toolbar"
    feature_name: "Access design tools from the toolbar"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Access design tools from the toolbar to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Access design tools from the toolbar with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Access design tools from the toolbar with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041064174-access-design-tools-from-the-toolbar"
        feature_name: "Access design tools from the toolbar"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Access design tools from the toolbar"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Access design tools from the toolbar inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041064174-Access-design-tools-from-the-toolbar"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039831974-explore-the-navigation-bar-and-left-sidebar.v0"
    source_feature_id: "figma.platform.leaf.360039831974-explore-the-navigation-bar-and-left-sidebar"
    feature_name: "Explore the navigation bar and left sidebar"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Explore the navigation bar and left sidebar to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Explore the navigation bar and left sidebar with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Explore the navigation bar and left sidebar with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039831974-explore-the-navigation-bar-and-left-sidebar"
        feature_name: "Explore the navigation bar and left sidebar"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Explore the navigation bar and left sidebar"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Explore the navigation bar and left sidebar inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039831974-Explore-the-navigation-bar-and-left-sidebar"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039832014-design-prototype-and-explore-layer-properties-in-the-right-sidebar.v0"
    source_feature_id: "figma.platform.leaf.360039832014-design-prototype-and-explore-layer-properties-in-the-right-sidebar"
    feature_name: "Design, prototype, and explore layer properties in the right sidebar"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Design, prototype, and explore layer properties in the right sidebar to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Design, prototype, and explore layer properties in the right sidebar with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Design, prototype, and explore layer properties in the right sidebar with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039832014-design-prototype-and-explore-layer-properties-in-the-right-sidebar"
        feature_name: "Design, prototype, and explore layer properties in the right sidebar"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Design, prototype, and explore layer properties in the right sidebar"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Design, prototype, and explore layer properties in the right sidebar inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar"
  - feature_use_card_id: "fuc.figma.platform.leaf.41414918021271-hide-or-minimize-the-ui.v0"
    source_feature_id: "figma.platform.leaf.41414918021271-hide-or-minimize-the-ui"
    feature_name: "Hide or minimize the UI"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Hide or minimize the UI to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Hide or minimize the UI with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Hide or minimize the UI with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.41414918021271-hide-or-minimize-the-ui"
        feature_name: "Hide or minimize the UI"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Hide or minimize the UI"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Hide or minimize the UI inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/41414918021271-Hide-or-minimize-the-UI"
  - feature_use_card_id: "fuc.figma.platform.leaf.23570416033943-use-the-actions-menu-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.23570416033943-use-the-actions-menu-in-figma-design"
    feature_name: "Use the actions menu in Figma Design"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use the actions menu in Figma Design to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use the actions menu in Figma Design with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use the actions menu in Figma Design with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.23570416033943-use-the-actions-menu-in-figma-design"
        feature_name: "Use the actions menu in Figma Design"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use the actions menu in Figma Design"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use the actions menu in Figma Design inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/23570416033943-Use-the-actions-menu-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040328653-use-figma-products-with-a-keyboard.v0"
    source_feature_id: "figma.platform.leaf.360040328653-use-figma-products-with-a-keyboard"
    feature_name: "Use Figma products with a keyboard"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use Figma products with a keyboard to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use Figma products with a keyboard with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use Figma products with a keyboard with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040328653-use-figma-products-with-a-keyboard"
        feature_name: "Use Figma products with a keyboard"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use Figma products with a keyboard"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use Figma products with a keyboard inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040328653-Use-Figma-products-with-a-keyboard"
  - feature_use_card_id: "fuc.figma.platform.leaf.4404575206295-set-small-and-big-nudge-values.v0"
    source_feature_id: "figma.platform.leaf.4404575206295-set-small-and-big-nudge-values"
    feature_name: "Set small and big nudge values"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Set small and big nudge values to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Set small and big nudge values with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Set small and big nudge values with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4404575206295-set-small-and-big-nudge-values"
        feature_name: "Set small and big nudge values"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Set small and big nudge values"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Set small and big nudge values inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4404575206295-Set-small-and-big-nudge-values"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041065034-adjust-your-zoom-and-view-options.v0"
    source_feature_id: "figma.platform.leaf.360041065034-adjust-your-zoom-and-view-options"
    feature_name: "Adjust your zoom and view options"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Adjust your zoom and view options to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Adjust your zoom and view options with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Adjust your zoom and view options with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041065034-adjust-your-zoom-and-view-options"
        feature_name: "Adjust your zoom and view options"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Adjust your zoom and view options"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Adjust your zoom and view options inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041065034-Adjust-your-zoom-and-view-options"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038511413-set-custom-thumbnails-for-files.v0"
    source_feature_id: "figma.platform.leaf.360038511413-set-custom-thumbnails-for-files"
    feature_name: "Set custom thumbnails for files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Set custom thumbnails for files as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Set custom thumbnails for files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Set custom thumbnails for files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038511413-set-custom-thumbnails-for-files"
        feature_name: "Set custom thumbnails for files"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Set custom thumbnails for files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Set custom thumbnails for files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038511413-Set-custom-thumbnails-for-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040449713-add-guides-to-the-canvas-or-frames.v0"
    source_feature_id: "figma.platform.leaf.360040449713-add-guides-to-the-canvas-or-frames"
    feature_name: "Add guides to the canvas or frames"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add guides to the canvas or frames to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add guides to the canvas or frames with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add guides to the canvas or frames with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040449713-add-guides-to-the-canvas-or-frames"
        feature_name: "Add guides to the canvas or frames"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add guides to the canvas or frames"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add guides to the canvas or frames inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040449713-Add-guides-to-the-canvas-or-frames"
  - feature_use_card_id: "fuc.figma.platform.leaf.5724448965527-view-layer-outlines-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.5724448965527-view-layer-outlines-in-figma-design"
    feature_name: "View layer outlines in Figma Design"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use View layer outlines in Figma Design to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named View layer outlines in Figma Design with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute View layer outlines in Figma Design with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5724448965527-view-layer-outlines-in-figma-design"
        feature_name: "View layer outlines in Figma Design"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / View layer outlines in Figma Design"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover View layer outlines in Figma Design inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5724448965527-View-layer-outlines-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.9141292269847-find-and-replace-in-figma.v0"
    source_feature_id: "figma.platform.leaf.9141292269847-find-and-replace-in-figma"
    feature_name: "Find and replace in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Find and replace in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Find and replace in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Find and replace in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.9141292269847-find-and-replace-in-figma"
        feature_name: "Find and replace in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Find and replace in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Find and replace in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/9141292269847-Find-and-replace-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.26584819173271-layers-101-get-started-with-layers.v0"
    source_feature_id: "figma.platform.leaf.26584819173271-layers-101-get-started-with-layers"
    feature_name: "Layers 101: Get started with layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Layers 101: Get started with layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Layers 101: Get started with layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Layers 101: Get started with layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.26584819173271-layers-101-get-started-with-layers"
        feature_name: "Layers 101: Get started with layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Layers 101: Get started with layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Layers 101: Get started with layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/26584819173271-Layers-101-Get-started-with-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.26620239826199-layers-101-explore-layer-types.v0"
    source_feature_id: "figma.platform.leaf.26620239826199-layers-101-explore-layer-types"
    feature_name: "Layers 101: Explore layer types"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Layers 101: Explore layer types to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Layers 101: Explore layer types with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Layers 101: Explore layer types with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.26620239826199-layers-101-explore-layer-types"
        feature_name: "Layers 101: Explore layer types"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Layers 101: Explore layer types"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Layers 101: Explore layer types inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/26620239826199-Layers-101-Explore-layer-types"
  - feature_use_card_id: "fuc.figma.platform.leaf.26610806345623-layers-101-combine-layers.v0"
    source_feature_id: "figma.platform.leaf.26610806345623-layers-101-combine-layers"
    feature_name: "Layers 101: Combine layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Layers 101: Combine layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Layers 101: Combine layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Layers 101: Combine layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.26610806345623-layers-101-combine-layers"
        feature_name: "Layers 101: Combine layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Layers 101: Combine layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Layers 101: Combine layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/26610806345623-Layers-101-Combine-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041539473-frames-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.360041539473-frames-in-figma-design"
    feature_name: "Frames in Figma Design"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Frames in Figma Design to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Frames in Figma Design with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Frames in Figma Design with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041539473-frames-in-figma-design"
        feature_name: "Frames in Figma Design"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Frames in Figma Design"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Frames in Figma Design inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041539473-Frames-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.4402723791511-sketch-on-the-canvas-with-the-pencil-tool.v0"
    source_feature_id: "figma.platform.leaf.4402723791511-sketch-on-the-canvas-with-the-pencil-tool"
    feature_name: "Sketch on the canvas with the pencil tool"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Sketch on the canvas with the pencil tool to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Sketch on the canvas with the pencil tool with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Sketch on the canvas with the pencil tool with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4402723791511-sketch-on-the-canvas-with-the-pencil-tool"
        feature_name: "Sketch on the canvas with the pencil tool"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Sketch on the canvas with the pencil tool"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Sketch on the canvas with the pencil tool inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4402723791511-Sketch-on-the-canvas-with-the-pencil-tool"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040450133-shape-tools.v0"
    source_feature_id: "figma.platform.leaf.360040450133-shape-tools"
    feature_name: "Shape tools"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Shape tools to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Shape tools with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Shape tools with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040450133-shape-tools"
        feature_name: "Shape tools"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Shape tools"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Shape tools inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040450133-Shape-tools"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039832054-the-difference-between-frames-and-groups.v0"
    source_feature_id: "figma.platform.leaf.360039832054-the-difference-between-frames-and-groups"
    feature_name: "The difference between frames and groups"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use The difference between frames and groups to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named The difference between frames and groups with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute The difference between frames and groups with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039832054-the-difference-between-frames-and-groups"
        feature_name: "The difference between frames and groups"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / The difference between frames and groups"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover The difference between frames and groups inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039832054-The-difference-between-frames-and-groups"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040450173-arc-tool-create-arcs-semi-circles-and-rings.v0"
    source_feature_id: "figma.platform.leaf.360040450173-arc-tool-create-arcs-semi-circles-and-rings"
    feature_name: "Arc tool: create arcs, semi-circles, and rings"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Arc tool: create arcs, semi-circles, and rings to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Arc tool: create arcs, semi-circles, and rings with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Arc tool: create arcs, semi-circles, and rings with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040450173-arc-tool-create-arcs-semi-circles-and-rings"
        feature_name: "Arc tool: create arcs, semi-circles, and rings"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Arc tool: create arcs, semi-circles, and rings"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Arc tool: create arcs, semi-circles, and rings inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040450173-Arc-tool-create-arcs-semi-circles-and-rings"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040450253-masks.v0"
    source_feature_id: "figma.platform.leaf.360040450253-masks"
    feature_name: "Masks"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Masks to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Masks with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Masks with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040450253-masks"
        feature_name: "Masks"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Masks"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Masks inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040450253-Masks"
  - feature_use_card_id: "fuc.figma.platform.leaf.40826832449303-turn-webpages-into-editable-design-layers.v0"
    source_feature_id: "figma.platform.leaf.40826832449303-turn-webpages-into-editable-design-layers"
    feature_name: "Turn webpages into editable design layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Turn webpages into editable design layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Turn webpages into editable design layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Turn webpages into editable design layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.40826832449303-turn-webpages-into-editable-design-layers"
        feature_name: "Turn webpages into editable design layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Turn webpages into editable design layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Turn webpages into editable design layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/40826832449303-Turn-webpages-into-editable-design-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.21635177948567-edit-objects-on-the-canvas-in-bulk.v0"
    source_feature_id: "figma.platform.leaf.21635177948567-edit-objects-on-the-canvas-in-bulk"
    feature_name: "Edit objects on the canvas in bulk"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Edit objects on the canvas in bulk to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Edit objects on the canvas in bulk with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Edit objects on the canvas in bulk with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.21635177948567-edit-objects-on-the-canvas-in-bulk"
        feature_name: "Edit objects on the canvas in bulk"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Edit objects on the canvas in bulk"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Edit objects on the canvas in bulk inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/21635177948567-Edit-objects-on-the-canvas-in-bulk"
  - feature_use_card_id: "fuc.figma.platform.leaf.21523793229463-identify-matching-objects.v0"
    source_feature_id: "figma.platform.leaf.21523793229463-identify-matching-objects"
    feature_name: "Identify matching objects"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Identify matching objects to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Identify matching objects with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Identify matching objects with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.21523793229463-identify-matching-objects"
        feature_name: "Identify matching objects"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Identify matching objects"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Identify matching objects inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/21523793229463-Identify-matching-objects"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039959014-parent-child-and-sibling-relationships.v0"
    source_feature_id: "figma.platform.leaf.360039959014-parent-child-and-sibling-relationships"
    feature_name: "Parent, child, and sibling relationships"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Parent, child, and sibling relationships to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Parent, child, and sibling relationships with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Parent, child, and sibling relationships with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039959014-parent-child-and-sibling-relationships"
        feature_name: "Parent, child, and sibling relationships"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Parent, child, and sibling relationships"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Parent, child, and sibling relationships inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039959014-Parent-child-and-sibling-relationships"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040449873-select-layers-and-objects.v0"
    source_feature_id: "figma.platform.leaf.360040449873-select-layers-and-objects"
    feature_name: "Select layers and objects"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Select layers and objects to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Select layers and objects with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Select layers and objects with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040449873-select-layers-and-objects"
        feature_name: "Select layers and objects"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Select layers and objects"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Select layers and objects inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040449873-Select-layers-and-objects"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039956914-adjust-alignment-rotation-position-and-dimensions.v0"
    source_feature_id: "figma.platform.leaf.360039956914-adjust-alignment-rotation-position-and-dimensions"
    feature_name: "Adjust alignment, rotation, position, and dimensions"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Adjust alignment, rotation, position, and dimensions to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Adjust alignment, rotation, position, and dimensions with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Adjust alignment, rotation, position, and dimensions with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039956914-adjust-alignment-rotation-position-and-dimensions"
        feature_name: "Adjust alignment, rotation, position, and dimensions"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Adjust alignment, rotation, position, and dimensions"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Adjust alignment, rotation, position, and dimensions inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039956914-Adjust-alignment-rotation-position-and-dimensions"
  - feature_use_card_id: "fuc.figma.platform.leaf.4409078832791-copy-and-paste-objects.v0"
    source_feature_id: "figma.platform.leaf.4409078832791-copy-and-paste-objects"
    feature_name: "Copy and paste objects"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Copy and paste objects to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Copy and paste objects with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Copy and paste objects with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4409078832791-copy-and-paste-objects"
        feature_name: "Copy and paste objects"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Copy and paste objects"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Copy and paste objects inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4409078832791-Copy-and-paste-objects"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040451453-scale-layers-while-maintaining-proportions.v0"
    source_feature_id: "figma.platform.leaf.360040451453-scale-layers-while-maintaining-proportions"
    feature_name: "Scale layers while maintaining proportions"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Scale layers while maintaining proportions as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Scale layers while maintaining proportions with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Scale layers while maintaining proportions with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040451453-scale-layers-while-maintaining-proportions"
        feature_name: "Scale layers while maintaining proportions"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Scale layers while maintaining proportions"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Scale layers while maintaining proportions inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040451453-Scale-layers-while-maintaining-proportions"
  - feature_use_card_id: "fuc.figma.platform.leaf.9771500257687-organize-your-canvas-with-sections.v0"
    source_feature_id: "figma.platform.leaf.9771500257687-organize-your-canvas-with-sections"
    feature_name: "Organize your canvas with sections"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Organize your canvas with sections to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Organize your canvas with sections with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Organize your canvas with sections with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.9771500257687-organize-your-canvas-with-sections"
        feature_name: "Organize your canvas with sections"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Organize your canvas with sections"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Organize your canvas with sections inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/9771500257687-Organize-your-canvas-with-sections"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039956974-measure-distances-between-layers.v0"
    source_feature_id: "figma.platform.leaf.360039956974-measure-distances-between-layers"
    feature_name: "Measure distances between layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Measure distances between layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Measure distances between layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Measure distances between layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039956974-measure-distances-between-layers"
        feature_name: "Measure distances between layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Measure distances between layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Measure distances between layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039956974-Measure-distances-between-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041596573-lock-and-unlock-layers.v0"
    source_feature_id: "figma.platform.leaf.360041596573-lock-and-unlock-layers"
    feature_name: "Lock and unlock layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Lock and unlock layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Lock and unlock layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Lock and unlock layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041596573-lock-and-unlock-layers"
        feature_name: "Lock and unlock layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Lock and unlock layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Lock and unlock layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041596573-Lock-and-unlock-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041112614-toggle-visibility-to-hide-layers.v0"
    source_feature_id: "figma.platform.leaf.360041112614-toggle-visibility-to-hide-layers"
    feature_name: "Toggle visibility to hide layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Toggle visibility to hide layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Toggle visibility to hide layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Toggle visibility to hide layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041112614-toggle-visibility-to-hide-layers"
        feature_name: "Toggle visibility to hide layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Toggle visibility to hide layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Toggle visibility to hide layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041112614-Toggle-visibility-to-hide-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039958934-rename-layers.v0"
    source_feature_id: "figma.platform.leaf.360039958934-rename-layers"
    feature_name: "Rename Layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Rename Layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Rename Layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Rename Layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039958934-rename-layers"
        feature_name: "Rename Layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Rename Layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Rename Layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039958934-Rename-Layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.4412765442967-copy-and-paste-properties-between-layers.v0"
    source_feature_id: "figma.platform.leaf.4412765442967-copy-and-paste-properties-between-layers"
    feature_name: "Copy and paste properties between layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Copy and paste properties between layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Copy and paste properties between layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Copy and paste properties between layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4412765442967-copy-and-paste-properties-between-layers"
        feature_name: "Copy and paste properties between layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Copy and paste properties between layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Copy and paste properties between layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4412765442967-Copy-and-paste-properties-between-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040450233-arrange-layers-with-smart-selection.v0"
    source_feature_id: "figma.platform.leaf.360040450233-arrange-layers-with-smart-selection"
    feature_name: "Arrange layers with Smart selection"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Arrange layers with Smart selection to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Arrange layers with Smart selection with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Arrange layers with Smart selection with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040450233-arrange-layers-with-smart-selection"
        feature_name: "Arrange layers with Smart selection"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Arrange layers with Smart selection"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Arrange layers with Smart selection inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040450233-Arrange-layers-with-Smart-selection"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize.v0"
    source_feature_id: "figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize"
    feature_name: "Apply constraints to define how layers resize"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Apply constraints to define how layers resize as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Apply constraints to define how layers resize with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Apply constraints to define how layers resize with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize"
        feature_name: "Apply constraints to define how layers resize"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Apply constraints to define how layers resize"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Apply constraints to define how layers resize inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039957734-Apply-constraints-to-define-how-layers-resize"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040450513-create-layout-guides.v0"
    source_feature_id: "figma.platform.leaf.360040450513-create-layout-guides"
    feature_name: "Create layout guides"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create layout guides to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create layout guides with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create layout guides with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040450513-create-layout-guides"
        feature_name: "Create layout guides"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create layout guides"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create layout guides inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040450513-Create-layout-guides"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039957934-combine-layout-guides-and-constraints.v0"
    source_feature_id: "figma.platform.leaf.360039957934-combine-layout-guides-and-constraints"
    feature_name: "Combine layout guides and constraints"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Combine layout guides and constraints as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Combine layout guides and constraints with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Combine layout guides and constraints with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039957934-combine-layout-guides-and-constraints"
        feature_name: "Combine layout guides and constraints"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Combine layout guides and constraints"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Combine layout guides and constraints inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039957934-Combine-layout-guides-and-constraints"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040450213-vector-networks.v0"
    source_feature_id: "figma.platform.leaf.360040450213-vector-networks"
    feature_name: "Vector networks"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Vector networks to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Vector networks with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Vector networks with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040450213-vector-networks"
        feature_name: "Vector networks"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Vector networks"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Vector networks inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040450213-Vector-networks"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039957634-edit-vector-layers.v0"
    source_feature_id: "figma.platform.leaf.360039957634-edit-vector-layers"
    feature_name: "Edit vector layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Edit vector layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Edit vector layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Edit vector layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039957634-edit-vector-layers"
        feature_name: "Edit vector layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Edit vector layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Edit vector layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039957634-Edit-vector-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.31616004109847-create-custom-shapes-with-the-shape-builder-tool.v0"
    source_feature_id: "figma.platform.leaf.31616004109847-create-custom-shapes-with-the-shape-builder-tool"
    feature_name: "Create custom shapes with the shape builder tool"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create custom shapes with the shape builder tool to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create custom shapes with the shape builder tool with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create custom shapes with the shape builder tool with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.31616004109847-create-custom-shapes-with-the-shape-builder-tool"
        feature_name: "Create custom shapes with the shape builder tool"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create custom shapes with the shape builder tool"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create custom shapes with the shape builder tool inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/31616004109847-Create-custom-shapes-with-the-shape-builder-tool"
  - feature_use_card_id: "fuc.figma.platform.leaf.33052305733015-convert-strokes-to-vector-paths.v0"
    source_feature_id: "figma.platform.leaf.33052305733015-convert-strokes-to-vector-paths"
    feature_name: "Convert strokes to vector paths"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Convert strokes to vector paths to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Convert strokes to vector paths with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Convert strokes to vector paths with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.33052305733015-convert-strokes-to-vector-paths"
        feature_name: "Convert strokes to vector paths"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Convert strokes to vector paths"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Convert strokes to vector paths inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/33052305733015-Convert-strokes-to-vector-paths"
  - feature_use_card_id: "fuc.figma.platform.leaf.360047239073-convert-text-to-vector-paths.v0"
    source_feature_id: "figma.platform.leaf.360047239073-convert-text-to-vector-paths"
    feature_name: "Convert text to vector paths"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Convert text to vector paths to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Convert text to vector paths with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Convert text to vector paths with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360047239073-convert-text-to-vector-paths"
        feature_name: "Convert text to vector paths"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Convert text to vector paths"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Convert text to vector paths inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360047239073-Convert-text-to-vector-paths"
  - feature_use_card_id: "fuc.figma.platform.leaf.33792861450263-offset-a-vector-path.v0"
    source_feature_id: "figma.platform.leaf.33792861450263-offset-a-vector-path"
    feature_name: "Offset a vector path"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Offset a vector path to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Offset a vector path with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Offset a vector path with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.33792861450263-offset-a-vector-path"
        feature_name: "Offset a vector path"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Offset a vector path"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Offset a vector path inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/33792861450263-Offset-a-vector-path"
  - feature_use_card_id: "fuc.figma.platform.leaf.33792593975575-simplify-a-vector-path.v0"
    source_feature_id: "figma.platform.leaf.33792593975575-simplify-a-vector-path"
    feature_name: "Simplify a vector path"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Simplify a vector path to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Simplify a vector path with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Simplify a vector path with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.33792593975575-simplify-a-vector-path"
        feature_name: "Simplify a vector path"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Simplify a vector path"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Simplify a vector path inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/33792593975575-Simplify-a-vector-path"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039957534-boolean-operations.v0"
    source_feature_id: "figma.platform.leaf.360039957534-boolean-operations"
    feature_name: "Boolean operations"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Boolean operations to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Boolean operations with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Boolean operations with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039957534-boolean-operations"
        feature_name: "Boolean operations"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Boolean operations"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Boolean operations inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039957534-Boolean-operations"
  - feature_use_card_id: "fuc.figma.platform.leaf.30101373312279-flatten-layers.v0"
    source_feature_id: "figma.platform.leaf.30101373312279-flatten-layers"
    feature_name: "Flatten layers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Flatten layers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Flatten layers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Flatten layers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.30101373312279-flatten-layers"
        feature_name: "Flatten layers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Flatten layers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Flatten layers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/30101373312279-Flatten-layers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039956434-guide-to-text-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.360039956434-guide-to-text-in-figma-design"
    feature_name: "Guide to text in Figma Design"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to text in Figma Design to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to text in Figma Design with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to text in Figma Design with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039956434-guide-to-text-in-figma-design"
        feature_name: "Guide to text in Figma Design"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to text in Figma Design"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to text in Figma Design inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039956434-Guide-to-text-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039956634-explore-text-properties.v0"
    source_feature_id: "figma.platform.leaf.360039956634-explore-text-properties"
    feature_name: "Explore text properties"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Explore text properties to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Explore text properties with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Explore text properties with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039956634-explore-text-properties"
        feature_name: "Explore text properties"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Explore text properties"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Explore text properties inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039956894-add-a-font-to-figma.v0"
    source_feature_id: "figma.platform.leaf.360039956894-add-a-font-to-figma"
    feature_name: "Add a font to Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add a font to Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add a font to Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add a font to Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039956894-add-a-font-to-figma"
        feature_name: "Add a font to Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add a font to Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add a font to Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039956894-Add-a-font-to-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041308034-browse-and-apply-fonts.v0"
    source_feature_id: "figma.platform.leaf.360041308034-browse-and-apply-fonts"
    feature_name: "Browse and apply fonts"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Browse and apply fonts to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Browse and apply fonts with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Browse and apply fonts with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041308034-browse-and-apply-fonts"
        feature_name: "Browse and apply fonts"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Browse and apply fonts"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Browse and apply fonts inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041308034-Browse-and-apply-fonts"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039957034-create-and-apply-text-styles.v0"
    source_feature_id: "figma.platform.leaf.360039957034-create-and-apply-text-styles"
    feature_name: "Create and apply text styles"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create and apply text styles to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create and apply text styles with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create and apply text styles with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039957034-create-and-apply-text-styles"
        feature_name: "Create and apply text styles"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create and apply text styles"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create and apply text styles inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039957034-Create-and-apply-text-styles"
  - feature_use_card_id: "fuc.figma.platform.leaf.27378154668951-adjust-text-dimensions-and-resizing.v0"
    source_feature_id: "figma.platform.leaf.27378154668951-adjust-text-dimensions-and-resizing"
    feature_name: "Adjust text dimensions and resizing"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Adjust text dimensions and resizing to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Adjust text dimensions and resizing with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Adjust text dimensions and resizing with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.27378154668951-adjust-text-dimensions-and-resizing"
        feature_name: "Adjust text dimensions and resizing"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Adjust text dimensions and resizing"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Adjust text dimensions and resizing inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/27378154668951-Adjust-text-dimensions-and-resizing"
  - feature_use_card_id: "fuc.figma.platform.leaf.360045942953-add-links-to-text.v0"
    source_feature_id: "figma.platform.leaf.360045942953-add-links-to-text"
    feature_name: "Add links to text"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add links to text to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add links to text with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add links to text with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360045942953-add-links-to-text"
        feature_name: "Add links to text"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add links to text"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add links to text inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360045942953-Add-links-to-text"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039957174-add-emojis-and-smart-symbols-to-text.v0"
    source_feature_id: "figma.platform.leaf.360039957174-add-emojis-and-smart-symbols-to-text"
    feature_name: "Add emojis and smart symbols to text"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add emojis and smart symbols to text to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add emojis and smart symbols to text with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add emojis and smart symbols to text with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039957174-add-emojis-and-smart-symbols-to-text"
        feature_name: "Add emojis and smart symbols to text"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add emojis and smart symbols to text"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add emojis and smart symbols to text inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039957174-Add-emojis-and-smart-symbols-to-text"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040449773-create-bulleted-and-numbered-lists.v0"
    source_feature_id: "figma.platform.leaf.360040449773-create-bulleted-and-numbered-lists"
    feature_name: "Create bulleted and numbered lists"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create bulleted and numbered lists to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create bulleted and numbered lists with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create bulleted and numbered lists with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040449773-create-bulleted-and-numbered-lists"
        feature_name: "Create bulleted and numbered lists"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create bulleted and numbered lists"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create bulleted and numbered lists inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040449773-Create-bulleted-and-numbered-lists"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040449513-use-icon-fonts.v0"
    source_feature_id: "figma.platform.leaf.360040449513-use-icon-fonts"
    feature_name: "Use icon fonts"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use icon fonts to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use icon fonts with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use icon fonts with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040449513-use-icon-fonts"
        feature_name: "Use icon fonts"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use icon fonts"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use icon fonts inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040449513-Use-icon-fonts"
  - feature_use_card_id: "fuc.figma.platform.leaf.4913951097367-use-opentype-features.v0"
    source_feature_id: "figma.platform.leaf.4913951097367-use-opentype-features"
    feature_name: "Use OpenType features"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use OpenType features to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use OpenType features with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use OpenType features with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4913951097367-use-opentype-features"
        feature_name: "Use OpenType features"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use OpenType features"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use OpenType features inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4913951097367-Use-OpenType-features"
  - feature_use_card_id: "fuc.figma.platform.leaf.5579502031511-use-variable-fonts.v0"
    source_feature_id: "figma.platform.leaf.5579502031511-use-variable-fonts"
    feature_name: "Use variable fonts"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use variable fonts to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use variable fonts with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use variable fonts with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5579502031511-use-variable-fonts"
        feature_name: "Use variable fonts"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use variable fonts"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use variable fonts inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5579502031511-Use-variable-fonts"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040449673-add-text-in-chinese-japanese-and-korean.v0"
    source_feature_id: "figma.platform.leaf.360040449673-add-text-in-chinese-japanese-and-korean"
    feature_name: "Add text in Chinese, Japanese, and Korean"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add text in Chinese, Japanese, and Korean to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add text in Chinese, Japanese, and Korean with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add text in Chinese, Japanese, and Korean with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040449673-add-text-in-chinese-japanese-and-korean"
        feature_name: "Add text in Chinese, Japanese, and Korean"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add text in Chinese, Japanese, and Korean"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add text in Chinese, Japanese, and Korean inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040449673-Add-text-in-Chinese-Japanese-and-Korean"
  - feature_use_card_id: "fuc.figma.platform.leaf.4972283635863-add-right-to-left-text.v0"
    source_feature_id: "figma.platform.leaf.4972283635863-add-right-to-left-text"
    feature_name: "Add right-to-left text"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add right-to-left text to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add right-to-left text with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add right-to-left text with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4972283635863-add-right-to-left-text"
        feature_name: "Add right-to-left text"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add right-to-left text"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add right-to-left text inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4972283635863-Add-right-to-left-text"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041003694-guide-to-fills.v0"
    source_feature_id: "figma.platform.leaf.360041003694-guide-to-fills"
    feature_name: "Guide to fills"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to fills to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to fills with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to fills with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041003694-guide-to-fills"
        feature_name: "Guide to fills"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to fills"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to fills inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041003694-Guide-to-fills"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041003774-update-fills-using-the-color-picker.v0"
    source_feature_id: "figma.platform.leaf.360041003774-update-fills-using-the-color-picker"
    feature_name: "Update fills using the color picker"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Update fills using the color picker to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Update fills using the color picker with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Update fills using the color picker with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041003774-update-fills-using-the-color-picker"
        feature_name: "Update fills using the color picker"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Update fills using the color picker"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Update fills using the color picker inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041003774-Update-fills-using-the-color-picker"
  - feature_use_card_id: "fuc.figma.platform.leaf.34208860210199-use-gradients-as-a-fill-or-stroke.v0"
    source_feature_id: "figma.platform.leaf.34208860210199-use-gradients-as-a-fill-or-stroke"
    feature_name: "Use gradients as a fill or stroke"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use gradients as a fill or stroke to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use gradients as a fill or stroke with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use gradients as a fill or stroke with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.34208860210199-use-gradients-as-a-fill-or-stroke"
        feature_name: "Use gradients as a fill or stroke"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use gradients as a fill or stroke"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use gradients as a fill or stroke inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/34208860210199-Use-gradients-as-a-fill-or-stroke"
  - feature_use_card_id: "fuc.figma.platform.leaf.31616030150167-use-patterns-as-a-fill-or-stroke.v0"
    source_feature_id: "figma.platform.leaf.31616030150167-use-patterns-as-a-fill-or-stroke"
    feature_name: "Use patterns as a fill or stroke"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use patterns as a fill or stroke to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use patterns as a fill or stroke with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use patterns as a fill or stroke with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.31616030150167-use-patterns-as-a-fill-or-stroke"
        feature_name: "Use patterns as a fill or stroke"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use patterns as a fill or stroke"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use patterns as a fill or stroke inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/31616030150167-Use-patterns-as-a-fill-or-stroke"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040667874-apply-blend-modes-to-layers-fills-and-effects.v0"
    source_feature_id: "figma.platform.leaf.360040667874-apply-blend-modes-to-layers-fills-and-effects"
    feature_name: "Apply blend modes to layers, fills, and effects"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Apply blend modes to layers, fills, and effects to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Apply blend modes to layers, fills, and effects with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Apply blend modes to layers, fills, and effects with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040667874-apply-blend-modes-to-layers-fills-and-effects"
        feature_name: "Apply blend modes to layers, fills, and effects"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Apply blend modes to layers, fills, and effects"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Apply blend modes to layers, fills, and effects inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040667874-Apply-blend-modes-to-layers-fills-and-effects"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040028034-add-images-and-videos-to-designs.v0"
    source_feature_id: "figma.platform.leaf.360040028034-add-images-and-videos-to-designs"
    feature_name: "Add images and videos to designs"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add images and videos to designs to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add images and videos to designs with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add images and videos to designs with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040028034-add-images-and-videos-to-designs"
        feature_name: "Add images and videos to designs"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add images and videos to designs"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add images and videos to designs inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040028034-Add-images-and-videos-to-designs"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041098433-adjust-the-properties-of-an-image.v0"
    source_feature_id: "figma.platform.leaf.360041098433-adjust-the-properties-of-an-image"
    feature_name: "Adjust the properties of an image"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Adjust the properties of an image to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Adjust the properties of an image with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Adjust the properties of an image with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041098433-adjust-the-properties-of-an-image"
        feature_name: "Adjust the properties of an image"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Adjust the properties of an image"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Adjust the properties of an image inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041098433-Adjust-the-properties-of-an-image"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040675194-crop-an-image.v0"
    source_feature_id: "figma.platform.leaf.360040675194-crop-an-image"
    feature_name: "Crop an image"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Crop an image to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Crop an image with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Crop an image with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040675194-crop-an-image"
        feature_name: "Crop an image"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Crop an image"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Crop an image inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040675194-Crop-an-image"
  - feature_use_card_id: "fuc.figma.platform.leaf.27643269375767-sample-colors-with-the-eyedropper-tool.v0"
    source_feature_id: "figma.platform.leaf.27643269375767-sample-colors-with-the-eyedropper-tool"
    feature_name: "Sample colors with the eyedropper tool"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Sample colors with the eyedropper tool to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Sample colors with the eyedropper tool with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Sample colors with the eyedropper tool with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.27643269375767-sample-colors-with-the-eyedropper-tool"
        feature_name: "Sample colors with the eyedropper tool"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Sample colors with the eyedropper tool"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Sample colors with the eyedropper tool inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/27643269375767-Sample-colors-with-the-eyedropper-tool"
  - feature_use_card_id: "fuc.figma.platform.leaf.360042553434-view-and-adjust-colors-in-a-mixed-selection.v0"
    source_feature_id: "figma.platform.leaf.360042553434-view-and-adjust-colors-in-a-mixed-selection"
    feature_name: "View and adjust colors in a mixed selection"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use View and adjust colors in a mixed selection to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named View and adjust colors in a mixed selection with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute View and adjust colors in a mixed selection with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360042553434-view-and-adjust-colors-in-a-mixed-selection"
        feature_name: "View and adjust colors in a mixed selection"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / View and adjust colors in a mixed selection"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover View and adjust colors in a mixed selection inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360042553434-View-and-adjust-colors-in-a-mixed-selection"
  - feature_use_card_id: "fuc.figma.platform.leaf.360043042113-about-color-models.v0"
    source_feature_id: "figma.platform.leaf.360043042113-about-color-models"
    feature_name: "About color models"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use About color models as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named About color models with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute About color models with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360043042113-about-color-models"
        feature_name: "About color models"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / About color models"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover About color models inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360043042113-About-color-models"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038662654-guide-to-components-in-figma.v0"
    source_feature_id: "figma.platform.leaf.360038662654-guide-to-components-in-figma"
    feature_name: "Guide to components in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to components in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to components in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to components in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038662654-guide-to-components-in-figma"
        feature_name: "Guide to components in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to components in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to components in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038662654-Guide-to-components-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.38607529833751-migrate-a-library-to-using-slots.v0"
    source_feature_id: "figma.platform.leaf.38607529833751-migrate-a-library-to-using-slots"
    feature_name: "Migrate a library to using slots"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Migrate a library to using slots to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Migrate a library to using slots with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Migrate a library to using slots with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.38607529833751-migrate-a-library-to-using-slots"
        feature_name: "Migrate a library to using slots"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Migrate a library to using slots"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Migrate a library to using slots inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/38607529833751-Migrate-a-library-to-using-slots"
  - feature_use_card_id: "fuc.figma.platform.leaf.38231200344599-use-slots-to-build-flexible-components-in-figma.v0"
    source_feature_id: "figma.platform.leaf.38231200344599-use-slots-to-build-flexible-components-in-figma"
    feature_name: "Use slots to build flexible components in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use slots to build flexible components in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use slots to build flexible components in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use slots to build flexible components in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.38231200344599-use-slots-to-build-flexible-components-in-figma"
        feature_name: "Use slots to build flexible components in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use slots to build flexible components in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use slots to build flexible components in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/38231200344599-Use-slots-to-build-flexible-components-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038663154-create-components-to-reuse-in-designs.v0"
    source_feature_id: "figma.platform.leaf.360038663154-create-components-to-reuse-in-designs"
    feature_name: "Create components to reuse in designs"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create components to reuse in designs to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create components to reuse in designs with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create components to reuse in designs with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038663154-create-components-to-reuse-in-designs"
        feature_name: "Create components to reuse in designs"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create components to reuse in designs"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create components to reuse in designs inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038663154-Create-components-to-reuse-in-designs"
  - feature_use_card_id: "fuc.figma.platform.leaf.360056440594-create-and-use-variants.v0"
    source_feature_id: "figma.platform.leaf.360056440594-create-and-use-variants"
    feature_name: "Create and use variants"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create and use variants to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create and use variants with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create and use variants with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360056440594-create-and-use-variants"
        feature_name: "Create and use variants"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create and use variants"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create and use variants inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360056440594-Create-and-use-variants"
  - feature_use_card_id: "fuc.figma.platform.leaf.360061175334-create-interactive-components-with-variants.v0"
    source_feature_id: "figma.platform.leaf.360061175334-create-interactive-components-with-variants"
    feature_name: "Create interactive components with variants"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create interactive components with variants to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create interactive components with variants with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create interactive components with variants with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360061175334-create-interactive-components-with-variants"
        feature_name: "Create interactive components with variants"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create interactive components with variants"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create interactive components with variants inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360061175334-Create-interactive-components-with-variants"
  - feature_use_card_id: "fuc.figma.platform.leaf.38741465279895-the-difference-between-slots-instance-swaps-and-variants.v0"
    source_feature_id: "figma.platform.leaf.38741465279895-the-difference-between-slots-instance-swaps-and-variants"
    feature_name: "The difference between slots, instance swaps, and variants"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use The difference between slots, instance swaps, and variants to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named The difference between slots, instance swaps, and variants with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute The difference between slots, instance swaps, and variants with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.38741465279895-the-difference-between-slots-instance-swaps-and-variants"
        feature_name: "The difference between slots, instance swaps, and variants"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / The difference between slots, instance swaps, and variants"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover The difference between slots, instance swaps, and variants inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/38741465279895-The-difference-between-slots-instance-swaps-and-variants"
  - feature_use_card_id: "fuc.figma.platform.leaf.5579474826519-explore-component-properties.v0"
    source_feature_id: "figma.platform.leaf.5579474826519-explore-component-properties"
    feature_name: "Explore component properties"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Explore component properties to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Explore component properties with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Explore component properties with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5579474826519-explore-component-properties"
        feature_name: "Explore component properties"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Explore component properties"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Explore component properties inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5579474826519-Explore-component-properties"
  - feature_use_card_id: "fuc.figma.platform.leaf.41307940738967-create-and-use-animated-components.v0"
    source_feature_id: "figma.platform.leaf.41307940738967-create-and-use-animated-components"
    feature_name: "Create and use animated components"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create and use animated components to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create and use animated components with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create and use animated components with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.41307940738967-create-and-use-animated-components"
        feature_name: "Create and use animated components"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create and use animated components"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create and use animated components inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/41307940738967-Create-and-use-animated-components"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038663994-name-and-organize-components.v0"
    source_feature_id: "figma.platform.leaf.360038663994-name-and-organize-components"
    feature_name: "Name and organize components"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Name and organize components to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Name and organize components with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Name and organize components with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038663994-name-and-organize-components"
        feature_name: "Name and organize components"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Name and organize components"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Name and organize components inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038663994-Name-and-organize-components"
  - feature_use_card_id: "fuc.figma.platform.leaf.15339657135383-guide-to-variables-in-figma.v0"
    source_feature_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
    feature_name: "Guide to variables in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to variables in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to variables in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to variables in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
        feature_name: "Guide to variables in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to variables in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to variables in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15339657135383-Guide-to-variables-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.14506821864087-overview-of-variables-collections-and-modes.v0"
    source_feature_id: "figma.platform.leaf.14506821864087-overview-of-variables-collections-and-modes"
    feature_name: "Overview of variables, collections, and modes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Overview of variables, collections, and modes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Overview of variables, collections, and modes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Overview of variables, collections, and modes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.14506821864087-overview-of-variables-collections-and-modes"
        feature_name: "Overview of variables, collections, and modes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Overview of variables, collections, and modes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Overview of variables, collections, and modes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/14506821864087-Overview-of-variables-collections-and-modes"
  - feature_use_card_id: "fuc.figma.platform.leaf.15145852043927-create-and-manage-variables-and-collections.v0"
    source_feature_id: "figma.platform.leaf.15145852043927-create-and-manage-variables-and-collections"
    feature_name: "Create and manage variables and collections"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create and manage variables and collections to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create and manage variables and collections with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create and manage variables and collections with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15145852043927-create-and-manage-variables-and-collections"
        feature_name: "Create and manage variables and collections"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create and manage variables and collections"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create and manage variables and collections inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15145852043927-Create-and-manage-variables-and-collections"
  - feature_use_card_id: "fuc.figma.platform.leaf.15343107263511-apply-variables-to-designs.v0"
    source_feature_id: "figma.platform.leaf.15343107263511-apply-variables-to-designs"
    feature_name: "Apply variables to designs"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Apply variables to designs to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Apply variables to designs with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Apply variables to designs with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15343107263511-apply-variables-to-designs"
        feature_name: "Apply variables to designs"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Apply variables to designs"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Apply variables to designs inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15343107263511-Apply-variables-to-designs"
  - feature_use_card_id: "fuc.figma.platform.leaf.15343816063383-modes-for-variables.v0"
    source_feature_id: "figma.platform.leaf.15343816063383-modes-for-variables"
    feature_name: "Modes for variables"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Modes for variables to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Modes for variables with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Modes for variables with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15343816063383-modes-for-variables"
        feature_name: "Modes for variables"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Modes for variables"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Modes for variables inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15343816063383-Modes-for-variables"
  - feature_use_card_id: "fuc.figma.platform.leaf.36346281624471-extend-a-variable-collection.v0"
    source_feature_id: "figma.platform.leaf.36346281624471-extend-a-variable-collection"
    feature_name: "Extend a variable collection"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Extend a variable collection to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Extend a variable collection with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Extend a variable collection with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.36346281624471-extend-a-variable-collection"
        feature_name: "Extend a variable collection"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Extend a variable collection"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Extend a variable collection inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/36346281624471-Extend-a-variable-collection"
  - feature_use_card_id: "fuc.figma.platform.leaf.15871097384471-the-difference-between-variables-and-styles.v0"
    source_feature_id: "figma.platform.leaf.15871097384471-the-difference-between-variables-and-styles"
    feature_name: "The difference between variables and styles"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use The difference between variables and styles to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named The difference between variables and styles with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute The difference between variables and styles with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15871097384471-the-difference-between-variables-and-styles"
        feature_name: "The difference between variables and styles"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / The difference between variables and styles"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover The difference between variables and styles inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15871097384471-The-difference-between-variables-and-styles"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041051154-guide-to-libraries-in-figma.v0"
    source_feature_id: "figma.platform.leaf.360041051154-guide-to-libraries-in-figma"
    feature_name: "Guide to libraries in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to libraries in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to libraries in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to libraries in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041051154-guide-to-libraries-in-figma"
        feature_name: "Guide to libraries in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to libraries in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to libraries in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041051154-Guide-to-libraries-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.7938814091287-add-descriptions-to-styles-components-and-variables.v0"
    source_feature_id: "figma.platform.leaf.7938814091287-add-descriptions-to-styles-components-and-variables"
    feature_name: "Add descriptions to styles, components, and variables"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add descriptions to styles, components, and variables to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add descriptions to styles, components, and variables with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add descriptions to styles, components, and variables with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.7938814091287-add-descriptions-to-styles-components-and-variables"
        feature_name: "Add descriptions to styles, components, and variables"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add descriptions to styles, components, and variables"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add descriptions to styles, components, and variables inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/7938814091287-Add-descriptions-to-styles-components-and-variables"
  - feature_use_card_id: "fuc.figma.platform.leaf.360025508373-publish-a-library.v0"
    source_feature_id: "figma.platform.leaf.360025508373-publish-a-library"
    feature_name: "Publish a library"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Publish a library to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Publish a library with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Publish a library with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360025508373-publish-a-library"
        feature_name: "Publish a library"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Publish a library"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Publish a library inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360025508373-Publish-a-library"
  - feature_use_card_id: "fuc.figma.platform.leaf.4404848314647-move-published-components.v0"
    source_feature_id: "figma.platform.leaf.4404848314647-move-published-components"
    feature_name: "Move published components"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Move published components to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Move published components with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Move published components with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4404848314647-move-published-components"
        feature_name: "Move published components"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Move published components"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Move published components inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4404848314647-Move-published-components"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038665934-edit-main-components.v0"
    source_feature_id: "figma.platform.leaf.360038665934-edit-main-components"
    feature_name: "Edit main components"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Edit main components as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Edit main components with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Edit main components with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038665934-edit-main-components"
        feature_name: "Edit main components"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Edit main components"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Edit main components inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038665934-Edit-main-components"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039238193-hide-styles-components-and-variables-when-publishing.v0"
    source_feature_id: "figma.platform.leaf.360039238193-hide-styles-components-and-variables-when-publishing"
    feature_name: "Hide styles, components, and variables when publishing"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Hide styles, components, and variables when publishing to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Hide styles, components, and variables when publishing with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Hide styles, components, and variables when publishing with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039238193-hide-styles-components-and-variables-when-publishing"
        feature_name: "Hide styles, components, and variables when publishing"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Hide styles, components, and variables when publishing"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Hide styles, components, and variables when publishing inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039238193-Hide-styles-components-and-variables-when-publishing"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039236853-unpublish-a-library.v0"
    source_feature_id: "figma.platform.leaf.360039236853-unpublish-a-library"
    feature_name: "Unpublish a library"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Unpublish a library to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Unpublish a library with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Unpublish a library with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039236853-unpublish-a-library"
        feature_name: "Unpublish a library"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Unpublish a library"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Unpublish a library inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039236853-Unpublish-a-library"
  - feature_use_card_id: "fuc.figma.platform.leaf.39592284074263-check-designs-in-figma.v0"
    source_feature_id: "figma.platform.leaf.39592284074263-check-designs-in-figma"
    feature_name: "Check designs in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Check designs in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Check designs in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Check designs in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.39592284074263-check-designs-in-figma"
        feature_name: "Check designs in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Check designs in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Check designs in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/39592284074263-Check-designs-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.24037833895831-get-started-with-apple-s-ui-kit.v0"
    source_feature_id: "figma.platform.leaf.24037833895831-get-started-with-apple-s-ui-kit"
    feature_name: "Get started with Apple's UI kit"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Get started with Apple's UI kit to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Get started with Apple's UI kit with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Get started with Apple's UI kit with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.24037833895831-get-started-with-apple-s-ui-kit"
        feature_name: "Get started with Apple's UI kit"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Get started with Apple's UI kit"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Get started with Apple's UI kit inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/24037833895831-Get-started-with-Apple-s-UI-kit"
  - feature_use_card_id: "fuc.figma.platform.leaf.24037724065943-start-designing-with-ui-kits.v0"
    source_feature_id: "figma.platform.leaf.24037724065943-start-designing-with-ui-kits"
    feature_name: "Start designing with UI kits"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Start designing with UI kits to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Start designing with UI kits with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Start designing with UI kits with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.24037724065943-start-designing-with-ui-kits"
        feature_name: "Start designing with UI kits"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Start designing with UI kits"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Start designing with UI kits inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/24037724065943-Start-designing-with-UI-kits"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040316193-apply-styles-to-layers-and-objects.v0"
    source_feature_id: "figma.platform.leaf.360040316193-apply-styles-to-layers-and-objects"
    feature_name: "Apply styles to layers and objects"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Apply styles to layers and objects to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Apply styles to layers and objects with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Apply styles to layers and objects with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040316193-apply-styles-to-layers-and-objects"
        feature_name: "Apply styles to layers and objects"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Apply styles to layers and objects"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Apply styles to layers and objects inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040316193-Apply-styles-to-layers-and-objects"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039234193-review-and-accept-library-updates.v0"
    source_feature_id: "figma.platform.leaf.360039234193-review-and-accept-library-updates"
    feature_name: "Review and accept library updates"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Review and accept library updates to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Review and accept library updates with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Review and accept library updates with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039234193-review-and-accept-library-updates"
        feature_name: "Review and accept library updates"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Review and accept library updates"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Review and accept library updates inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039234193-Review-and-accept-library-updates"
  - feature_use_card_id: "fuc.figma.platform.leaf.4404856784663-swap-libraries.v0"
    source_feature_id: "figma.platform.leaf.4404856784663-swap-libraries"
    feature_name: "Swap libraries"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Swap libraries to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Swap libraries with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Swap libraries with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4404856784663-swap-libraries"
        feature_name: "Swap libraries"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Swap libraries"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Swap libraries inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4404856784663-Swap-libraries"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039150173-create-and-insert-component-instances.v0"
    source_feature_id: "figma.platform.leaf.360039150173-create-and-insert-component-instances"
    feature_name: "Create and insert component instances"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create and insert component instances to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create and insert component instances with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create and insert component instances with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039150173-create-and-insert-component-instances"
        feature_name: "Create and insert component instances"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create and insert component instances"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create and insert component instances inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039150173-Create-and-insert-component-instances"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038665754-detach-an-instance-from-the-component.v0"
    source_feature_id: "figma.platform.leaf.360038665754-detach-an-instance-from-the-component"
    feature_name: "Detach an instance from the component"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Detach an instance from the component to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Detach an instance from the component with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Detach an instance from the component with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038665754-detach-an-instance-from-the-component"
        feature_name: "Detach an instance from the component"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Detach an instance from the component"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Detach an instance from the component inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038665754-Detach-an-instance-from-the-component"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039150733-apply-changes-to-instances.v0"
    source_feature_id: "figma.platform.leaf.360039150733-apply-changes-to-instances"
    feature_name: "Apply changes to instances"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Apply changes to instances to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Apply changes to instances with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Apply changes to instances with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039150733-apply-changes-to-instances"
        feature_name: "Apply changes to instances"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Apply changes to instances"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Apply changes to instances inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039150733-Apply-changes-to-instances"
  - feature_use_card_id: "fuc.figma.platform.leaf.8883757553943-edit-instances-with-component-properties.v0"
    source_feature_id: "figma.platform.leaf.8883757553943-edit-instances-with-component-properties"
    feature_name: "Edit instances with component properties"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Edit instances with component properties to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Edit instances with component properties with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Edit instances with component properties with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.8883757553943-edit-instances-with-component-properties"
        feature_name: "Edit instances with component properties"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Edit instances with component properties"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Edit instances with component properties inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/8883757553943-Edit-instances-with-component-properties"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040314193-guide-to-prototyping-in-figma.v0"
    source_feature_id: "figma.platform.leaf.360040314193-guide-to-prototyping-in-figma"
    feature_name: "Guide to prototyping in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to prototyping in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to prototyping in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to prototyping in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040314193-guide-to-prototyping-in-figma"
        feature_name: "Guide to prototyping in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to prototyping in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to prototyping in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040314193-Guide-to-prototyping-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.14397859494295-state-management-for-prototypes.v0"
    source_feature_id: "figma.platform.leaf.14397859494295-state-management-for-prototypes"
    feature_name: "State management for prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use State management for prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named State management for prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute State management for prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.14397859494295-state-management-for-prototypes"
        feature_name: "State management for prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / State management for prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover State management for prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/14397859494295-State-management-for-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041486873-use-animated-gifs-in-prototypes.v0"
    source_feature_id: "figma.platform.leaf.360041486873-use-animated-gifs-in-prototypes"
    feature_name: "Use animated GIFs in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use animated GIFs in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use animated GIFs in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use animated GIFs in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041486873-use-animated-gifs-in-prototypes"
        feature_name: "Use animated GIFs in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use animated GIFs in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use animated GIFs in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041486873-Use-animated-GIFs-in-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.8878274530455-use-videos-in-prototypes.v0"
    source_feature_id: "figma.platform.leaf.8878274530455-use-videos-in-prototypes"
    feature_name: "Use videos in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use videos in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use videos in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use videos in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.8878274530455-use-videos-in-prototypes"
        feature_name: "Use videos in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use videos in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use videos in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/8878274530455-Use-videos-in-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040035834-prototype-triggers.v0"
    source_feature_id: "figma.platform.leaf.360040035834-prototype-triggers"
    feature_name: "Prototype triggers"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Prototype triggers to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Prototype triggers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Prototype triggers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040035834-prototype-triggers"
        feature_name: "Prototype triggers"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Prototype triggers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Prototype triggers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040035834-Prototype-triggers"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040035874-prototype-actions.v0"
    source_feature_id: "figma.platform.leaf.360040035874-prototype-actions"
    feature_name: "Prototype actions"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Prototype actions to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Prototype actions with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Prototype actions with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040035874-prototype-actions"
        feature_name: "Prototype actions"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Prototype actions"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Prototype actions inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040035874-Prototype-actions"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040522373-prototype-animations.v0"
    source_feature_id: "figma.platform.leaf.360040522373-prototype-animations"
    feature_name: "Prototype animations"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Prototype animations to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Prototype animations with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Prototype animations with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040522373-prototype-animations"
        feature_name: "Prototype animations"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Prototype animations"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Prototype animations inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040522373-Prototype-animations"
  - feature_use_card_id: "fuc.figma.platform.leaf.360051748654-prototype-easing-and-spring-animations.v0"
    source_feature_id: "figma.platform.leaf.360051748654-prototype-easing-and-spring-animations"
    feature_name: "Prototype easing and spring animations"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Prototype easing and spring animations to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Prototype easing and spring animations with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Prototype easing and spring animations with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360051748654-prototype-easing-and-spring-animations"
        feature_name: "Prototype easing and spring animations"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Prototype easing and spring animations"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Prototype easing and spring animations inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360051748654-Prototype-easing-and-spring-animations"
  - feature_use_card_id: "fuc.figma.platform.leaf.16194160540567-use-sections-in-prototyping.v0"
    source_feature_id: "figma.platform.leaf.16194160540567-use-sections-in-prototyping"
    feature_name: "Use sections in prototyping"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use sections in prototyping to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use sections in prototyping with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use sections in prototyping with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.16194160540567-use-sections-in-prototyping"
        feature_name: "Use sections in prototyping"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use sections in prototyping"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use sections in prototyping inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/16194160540567-Use-sections-in-prototyping"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040315773-connect-your-prototype.v0"
    source_feature_id: "figma.platform.leaf.360040315773-connect-your-prototype"
    feature_name: "Connect your prototype"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Connect your prototype to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Connect your prototype with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Connect your prototype with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040315773-connect-your-prototype"
        feature_name: "Connect your prototype"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Connect your prototype"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Connect your prototype inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040315773-Connect-your-prototype"
  - feature_use_card_id: "fuc.figma.platform.leaf.4404380377367-add-prototype-connections-from-main-components.v0"
    source_feature_id: "figma.platform.leaf.4404380377367-add-prototype-connections-from-main-components"
    feature_name: "Add prototype connections from main components"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add prototype connections from main components as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Add prototype connections from main components with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Add prototype connections from main components with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4404380377367-add-prototype-connections-from-main-components"
        feature_name: "Add prototype connections from main components"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Add prototype connections from main components"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add prototype connections from main components inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4404380377367-Add-prototype-connections-from-main-components"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039823894-create-and-manage-prototype-flows.v0"
    source_feature_id: "figma.platform.leaf.360039823894-create-and-manage-prototype-flows"
    feature_name: "Create and manage prototype flows"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create and manage prototype flows to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create and manage prototype flows with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create and manage prototype flows with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039823894-create-and-manage-prototype-flows"
        feature_name: "Create and manage prototype flows"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create and manage prototype flows"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create and manage prototype flows inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039823894-Create-and-manage-prototype-flows"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039818254-create-overlays-in-your-prototypes.v0"
    source_feature_id: "figma.platform.leaf.360039818254-create-overlays-in-your-prototypes"
    feature_name: "Create overlays in your prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Create overlays in your prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Create overlays in your prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Create overlays in your prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039818254-create-overlays-in-your-prototypes"
        feature_name: "Create overlays in your prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Create overlays in your prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Create overlays in your prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039818254-Create-overlays-in-your-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.360051747774-preserve-scroll-position-in-prototypes.v0"
    source_feature_id: "figma.platform.leaf.360051747774-preserve-scroll-position-in-prototypes"
    feature_name: "Preserve scroll position in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Preserve scroll position in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Preserve scroll position in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Preserve scroll position in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360051747774-preserve-scroll-position-in-prototypes"
        feature_name: "Preserve scroll position in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Preserve scroll position in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Preserve scroll position in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360051747774-Preserve-scroll-position-in-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039818734-prototype-scroll-and-overflow-behavior.v0"
    source_feature_id: "figma.platform.leaf.360039818734-prototype-scroll-and-overflow-behavior"
    feature_name: "Prototype scroll and overflow behavior"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Prototype scroll and overflow behavior to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Prototype scroll and overflow behavior with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Prototype scroll and overflow behavior with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039818734-prototype-scroll-and-overflow-behavior"
        feature_name: "Prototype scroll and overflow behavior"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Prototype scroll and overflow behavior"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Prototype scroll and overflow behavior inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039818734-Prototype-scroll-and-overflow-behavior"
  - feature_use_card_id: "fuc.figma.platform.leaf.17146044893591-advanced-prototyping-examples.v0"
    source_feature_id: "figma.platform.leaf.17146044893591-advanced-prototyping-examples"
    feature_name: "Advanced prototyping examples"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Advanced prototyping examples to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Advanced prototyping examples with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Advanced prototyping examples with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.17146044893591-advanced-prototyping-examples"
        feature_name: "Advanced prototyping examples"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Advanced prototyping examples"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Advanced prototyping examples inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/17146044893591-Advanced-prototyping-examples"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039818874-smart-animate-layers-between-frames.v0"
    source_feature_id: "figma.platform.leaf.360039818874-smart-animate-layers-between-frames"
    feature_name: "Smart animate layers between frames"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Smart animate layers between frames to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Smart animate layers between frames with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Smart animate layers between frames with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039818874-smart-animate-layers-between-frames"
        feature_name: "Smart animate layers between frames"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Smart animate layers between frames"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Smart animate layers between frames inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039818874-Smart-animate-layers-between-frames"
  - feature_use_card_id: "fuc.figma.platform.leaf.15253268379799-variable-modes-in-prototypes.v0"
    source_feature_id: "figma.platform.leaf.15253268379799-variable-modes-in-prototypes"
    feature_name: "Variable modes in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Variable modes in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Variable modes in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Variable modes in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15253268379799-variable-modes-in-prototypes"
        feature_name: "Variable modes in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Variable modes in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Variable modes in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15253268379799-Variable-modes-in-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.15253220891799-multiple-actions-and-conditionals.v0"
    source_feature_id: "figma.platform.leaf.15253220891799-multiple-actions-and-conditionals"
    feature_name: "Multiple actions and conditionals"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Multiple actions and conditionals to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Multiple actions and conditionals with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Multiple actions and conditionals with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15253220891799-multiple-actions-and-conditionals"
        feature_name: "Multiple actions and conditionals"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Multiple actions and conditionals"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Multiple actions and conditionals inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15253220891799-Multiple-actions-and-conditionals"
  - feature_use_card_id: "fuc.figma.platform.leaf.15253194385943-use-expressions-in-prototypes.v0"
    source_feature_id: "figma.platform.leaf.15253194385943-use-expressions-in-prototypes"
    feature_name: "Use expressions in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use expressions in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use expressions in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use expressions in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.15253194385943-use-expressions-in-prototypes"
        feature_name: "Use expressions in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use expressions in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use expressions in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/15253194385943-Use-expressions-in-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.14506587589399-use-variables-in-prototypes.v0"
    source_feature_id: "figma.platform.leaf.14506587589399-use-variables-in-prototypes"
    feature_name: "Use variables in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use variables in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use variables in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use variables in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.14506587589399-use-variables-in-prototypes"
        feature_name: "Use variables in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use variables in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use variables in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/14506587589399-Use-variables-in-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.26463081577367-present-prototypes-offline.v0"
    source_feature_id: "figma.platform.leaf.26463081577367-present-prototypes-offline"
    feature_name: "Present prototypes offline"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Present prototypes offline to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Present prototypes offline with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Present prototypes offline with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.26463081577367-present-prototypes-offline"
        feature_name: "Present prototypes offline"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Present prototypes offline"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Present prototypes offline inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/26463081577367-Present-prototypes-offline"
  - feature_use_card_id: "fuc.figma.platform.leaf.21158597546391-set-prototype-device-and-background-settings.v0"
    source_feature_id: "figma.platform.leaf.21158597546391-set-prototype-device-and-background-settings"
    feature_name: "Set prototype device and background settings"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Set prototype device and background settings to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Set prototype device and background settings with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Set prototype device and background settings with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.21158597546391-set-prototype-device-and-background-settings"
        feature_name: "Set prototype device and background settings"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Set prototype device and background settings"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Set prototype device and background settings inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/21158597546391-Set-prototype-device-and-background-settings"
  - feature_use_card_id: "fuc.figma.platform.leaf.4411431245335-view-prototype-connections.v0"
    source_feature_id: "figma.platform.leaf.4411431245335-view-prototype-connections"
    feature_name: "View prototype connections"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use View prototype connections to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named View prototype connections with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute View prototype connections with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.4411431245335-view-prototype-connections"
        feature_name: "View prototype connections"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / View prototype connections"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover View prototype connections inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4411431245335-View-prototype-connections"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040318013-play-your-prototypes.v0"
    source_feature_id: "figma.platform.leaf.360040318013-play-your-prototypes"
    feature_name: "Play your prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Play your prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Play your prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Play your prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040318013-play-your-prototypes"
        feature_name: "Play your prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Play your prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Play your prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040318013-Play-your-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040321093-view-prototypes-on-a-mobile-device.v0"
    source_feature_id: "figma.platform.leaf.360040321093-view-prototypes-on-a-mobile-device"
    feature_name: "View prototypes on a mobile device"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use View prototypes on a mobile device to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named View prototypes on a mobile device with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute View prototypes on a mobile device with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040321093-view-prototypes-on-a-mobile-device"
        feature_name: "View prototypes on a mobile device"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / View prototypes on a mobile device"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover View prototypes on a mobile device inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040321093-View-prototypes-on-a-mobile-device"
  - feature_use_card_id: "fuc.figma.platform.leaf.7810391964695-accessible-prototypes-in-figma.v0"
    source_feature_id: "figma.platform.leaf.7810391964695-accessible-prototypes-in-figma"
    feature_name: "Accessible prototypes in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Accessible prototypes in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Accessible prototypes in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Accessible prototypes in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.7810391964695-accessible-prototypes-in-figma"
        feature_name: "Accessible prototypes in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Accessible prototypes in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Accessible prototypes in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/7810391964695-Accessible-prototypes-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.41307983648407-export-animations-from-figma.v0"
    source_feature_id: "figma.platform.leaf.41307983648407-export-animations-from-figma"
    feature_name: "Export animations from Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export animations from Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export animations from Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export animations from Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.41307983648407-export-animations-from-figma"
        feature_name: "Export animations from Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export animations from Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export animations from Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/41307983648407-Export-animations-from-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040027794-guide-to-imports-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.360040027794-guide-to-imports-in-figma-design"
    feature_name: "Guide to imports in Figma Design"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to imports in Figma Design to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to imports in Figma Design with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Guide to imports in Figma Design with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040027794-guide-to-imports-in-figma-design"
        feature_name: "Guide to imports in Figma Design"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to imports in Figma Design"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to imports in Figma Design inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040027794-Guide-to-imports-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041003114-import-files-to-the-file-browser.v0"
    source_feature_id: "figma.platform.leaf.360041003114-import-files-to-the-file-browser"
    feature_name: "Import files to the file browser"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Import files to the file browser to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Import files to the file browser with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Import files to the file browser with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041003114-import-files-to-the-file-browser"
        feature_name: "Import files to the file browser"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Import files to the file browser"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Import files to the file browser inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041003114-Import-files-to-the-file-browser"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040514273-import-sketch-files.v0"
    source_feature_id: "figma.platform.leaf.360040514273-import-sketch-files"
    feature_name: "Import Sketch files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Import Sketch files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Import Sketch files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Import Sketch files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040514273-import-sketch-files"
        feature_name: "Import Sketch files"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Import Sketch files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Import Sketch files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040514273-Import-Sketch-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040030374-copy-assets-between-design-tools.v0"
    source_feature_id: "figma.platform.leaf.360040030374-copy-assets-between-design-tools"
    feature_name: "Copy assets between design tools"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Copy assets between design tools to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Copy assets between design tools with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Copy assets between design tools with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040030374-copy-assets-between-design-tools"
        feature_name: "Copy assets between design tools"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Copy assets between design tools"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Copy assets between design tools inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040030374-Copy-assets-between-design-tools"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040028114-export-static-designs-from-figma.v0"
    source_feature_id: "figma.platform.leaf.360040028114-export-static-designs-from-figma"
    feature_name: "Export static designs from Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export static designs from Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export static designs from Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export static designs from Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040028114-export-static-designs-from-figma"
        feature_name: "Export static designs from Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export static designs from Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export static designs from Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040028114-Export-static-designs-from-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.13402894554519-export-formats-and-settings-for-static-designs.v0"
    source_feature_id: "figma.platform.leaf.13402894554519-export-formats-and-settings-for-static-designs"
    feature_name: "Export formats and settings for static designs"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export formats and settings for static designs to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export formats and settings for static designs with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export formats and settings for static designs with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.13402894554519-export-formats-and-settings-for-static-designs"
        feature_name: "Export formats and settings for static designs"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export formats and settings for static designs"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export formats and settings for static designs inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/13402894554519-Export-formats-and-settings-for-static-designs"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039825314-guide-to-comments-in-figma.v0"
    source_feature_id: "figma.platform.leaf.360039825314-guide-to-comments-in-figma"
    feature_name: "Guide to comments in Figma"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to comments in Figma to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to comments in Figma with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Guide to comments in Figma with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039825314-guide-to-comments-in-figma"
        feature_name: "Guide to comments in Figma"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to comments in Figma"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to comments in Figma inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039825314-Guide-to-comments-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041068574-add-comments-to-files.v0"
    source_feature_id: "figma.platform.leaf.360041068574-add-comments-to-files"
    feature_name: "Add comments to files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add comments to files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add comments to files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Add comments to files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041068574-add-comments-to-files"
        feature_name: "Add comments to files"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add comments to files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add comments to files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041068574-Add-comments-to-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041547593-view-and-manage-comments.v0"
    source_feature_id: "figma.platform.leaf.360041547593-view-and-manage-comments"
    feature_name: "View and manage comments"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use View and manage comments to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named View and manage comments with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute View and manage comments with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041547593-view-and-manage-comments"
        feature_name: "View and manage comments"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / View and manage comments"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover View and manage comments inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041547593-View-and-manage-comments"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041547853-move-or-edit-comments.v0"
    source_feature_id: "figma.platform.leaf.360041547853-move-or-edit-comments"
    feature_name: "Move or edit comments"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Move or edit comments to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Move or edit comments with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Move or edit comments with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041547853-move-or-edit-comments"
        feature_name: "Move or edit comments"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Move or edit comments"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Move or edit comments inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041547853-Move-or-edit-comments"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039824594-comment-on-prototypes.v0"
    source_feature_id: "figma.platform.leaf.360039824594-comment-on-prototypes"
    feature_name: "Comment on prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Comment on prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Comment on prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Comment on prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039824594-comment-on-prototypes"
        feature_name: "Comment on prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Comment on prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Comment on prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039824594-Comment-on-prototypes"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041547813-manage-email-notifications-for-comments-on-files.v0"
    source_feature_id: "figma.platform.leaf.360041547813-manage-email-notifications-for-comments-on-files"
    feature_name: "Manage email notifications for comments on files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Manage email notifications for comments on files as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Manage email notifications for comments on files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Manage email notifications for comments on files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041547813-manage-email-notifications-for-comments-on-files"
        feature_name: "Manage email notifications for comments on files"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Manage email notifications for comments on files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Manage email notifications for comments on files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041547813-Manage-email-notifications-for-comments-on-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.360063144053-guide-to-branching.v0"
    source_feature_id: "figma.platform.leaf.360063144053-guide-to-branching"
    feature_name: "Guide to branching"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to branching to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Guide to branching with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Guide to branching with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360063144053-guide-to-branching"
        feature_name: "Guide to branching"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Guide to branching"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to branching inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360063144053-Guide-to-branching"
  - feature_use_card_id: "fuc.figma.platform.leaf.5665697002263-share-a-branch.v0"
    source_feature_id: "figma.platform.leaf.5665697002263-share-a-branch"
    feature_name: "Share a branch"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Share a branch to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Share a branch with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Share a branch with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5665697002263-share-a-branch"
        feature_name: "Share a branch"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Share a branch"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Share a branch inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5665697002263-Share-a-branch"
  - feature_use_card_id: "fuc.figma.platform.leaf.5665728006423-get-updates-from-main-files.v0"
    source_feature_id: "figma.platform.leaf.5665728006423-get-updates-from-main-files"
    feature_name: "Get updates from main files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Get updates from main files as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Get updates from main files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Get updates from main files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5665728006423-get-updates-from-main-files"
        feature_name: "Get updates from main files"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Get updates from main files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Get updates from main files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5665728006423-Get-updates-from-main-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.5668839659415-view-and-manage-branches.v0"
    source_feature_id: "figma.platform.leaf.5668839659415-view-and-manage-branches"
    feature_name: "View and manage branches"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use View and manage branches to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named View and manage branches with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute View and manage branches with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5668839659415-view-and-manage-branches"
        feature_name: "View and manage branches"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / View and manage branches"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover View and manage branches inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5668839659415-View-and-manage-branches"
  - feature_use_card_id: "fuc.figma.platform.leaf.5691414603543-request-a-branch-review.v0"
    source_feature_id: "figma.platform.leaf.5691414603543-request-a-branch-review"
    feature_name: "Request a branch review"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Request a branch review to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Request a branch review with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Request a branch review with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5691414603543-request-a-branch-review"
        feature_name: "Request a branch review"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Request a branch review"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Request a branch review inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5691414603543-Request-a-branch-review"
  - feature_use_card_id: "fuc.figma.platform.leaf.5693123873687-review-branch-changes.v0"
    source_feature_id: "figma.platform.leaf.5693123873687-review-branch-changes"
    feature_name: "Review branch changes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Review branch changes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Review branch changes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Review branch changes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5693123873687-review-branch-changes"
        feature_name: "Review branch changes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Review branch changes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Review branch changes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5693123873687-Review-branch-changes"
  - feature_use_card_id: "fuc.figma.platform.leaf.5691189138839-merge-branch-into-main-file.v0"
    source_feature_id: "figma.platform.leaf.5691189138839-merge-branch-into-main-file"
    feature_name: "Merge branch into main file"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Merge branch into main file as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Merge branch into main file with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Merge branch into main file with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5691189138839-merge-branch-into-main-file"
        feature_name: "Merge branch into main file"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Merge branch into main file"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Merge branch into main file inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5691189138839-Merge-branch-into-main-file"
  - feature_use_card_id: "fuc.figma.platform.leaf.5691750511383-incomplete-merges-or-updates.v0"
    source_feature_id: "figma.platform.leaf.5691750511383-incomplete-merges-or-updates"
    feature_name: "Incomplete merges or updates"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Incomplete merges or updates to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Incomplete merges or updates with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute Incomplete merges or updates with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5691750511383-incomplete-merges-or-updates"
        feature_name: "Incomplete merges or updates"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Incomplete merges or updates"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Incomplete merges or updates inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5691750511383-Incomplete-merges-or-updates"
  - feature_use_card_id: "fuc.figma.platform.leaf.developers-2.v0"
    source_feature_id: "figma.platform.leaf.developers-2"
    feature_name: "Developers"
    source_apps: ["Figma Developer Platform"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_developer_platform"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Developers as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Developers with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_developer_platform"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Developers with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Developer Platform"
        feature_id: "figma.platform.leaf.developers-2"
        feature_name: "Developers"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Developers"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Developers inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-design-category-jina.md"
        url: "https://www.figma.com/developers/?utm_source=help-center&utm_medium=marketing_referral&utm_campaign=help_center"
  - feature_use_card_id: "fuc.figma.platform.leaf.35710574222487-beyond-the-basics-using-figma-make.v0"
    source_feature_id: "figma.platform.leaf.35710574222487-beyond-the-basics-using-figma-make"
    feature_name: "Go beyond the basics with Figma Make"
    source_apps: ["Figma Make"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_make"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Go beyond the basics with Figma Make as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Go beyond the basics with Figma Make with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_make"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Go beyond the basics with Figma Make with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Make"
        feature_id: "figma.platform.leaf.35710574222487-beyond-the-basics-using-figma-make"
        feature_name: "Go beyond the basics with Figma Make"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Go beyond the basics with Figma Make"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Go beyond the basics with Figma Make inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-make-category-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/35710574222487-Beyond-the-basics-Using-Figma-Make"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040028114.v0"
    source_feature_id: "figma.platform.leaf.360040028114"
    feature_name: "Export assets"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export assets to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export assets with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export assets with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040028114"
        feature_name: "Export assets"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export assets"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export assets inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-import-export-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040028114"
  - feature_use_card_id: "fuc.figma.platform.leaf.13402894554519-export-formats-and-settings.v0"
    source_feature_id: "figma.platform.leaf.13402894554519-export-formats-and-settings"
    feature_name: "Export settings"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export settings to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export settings with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export settings with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.13402894554519-export-formats-and-settings"
        feature_name: "Export settings"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export settings"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export settings inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-import-export-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/13402894554519-Export-formats-and-settings"
  - feature_use_card_id: "fuc.figma.platform.leaf.41307983648407.v0"
    source_feature_id: "figma.platform.leaf.41307983648407"
    feature_name: "Export animations"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export animations to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export animations with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export animations with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.41307983648407"
        feature_name: "Export animations"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export animations"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export animations inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-import-export-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/41307983648407"
  - feature_use_card_id: "fuc.figma.platform.leaf.8403626871063-save-a-local-copy-of-files.v0"
    source_feature_id: "figma.platform.leaf.8403626871063-save-a-local-copy-of-files"
    feature_name: "Export a design file"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Export a design file to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Export a design file with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Export a design file with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.8403626871063-save-a-local-copy-of-files"
        feature_name: "Export a design file"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Export a design file"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Export a design file inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-import-export-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/8403626871063-Save-a-local-copy-of-files"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040328273-plans-and-teams-in-figma.v0"
    source_feature_id: "figma.platform.leaf.360040328273-plans-and-teams-in-figma"
    feature_name: "all plans"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_first_collaboration_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use all plans to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named all plans with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_first_collaboration_primitive"
    expected_result: "Studio can represent or execute all plans with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040328273-plans-and-teams-in-figma"
        feature_name: "all plans"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / all plans"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover all plans inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040328273-Plans-and-teams-in-Figma"
  - feature_use_card_id: "fuc.figma.platform.leaf.19424714305943.v0"
    source_feature_id: "figma.platform.leaf.19424714305943"
    feature_name: "adjust your firewall settings"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use adjust your firewall settings to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named adjust your firewall settings with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute adjust your firewall settings with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.19424714305943"
        feature_name: "adjust your firewall settings"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / adjust your firewall settings"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover adjust your firewall settings inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/19424714305943"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040028034.v0"
    source_feature_id: "figma.platform.leaf.360040028034"
    feature_name: "Add images and videos to design files"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Add images and videos to design files to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Add images and videos to design files with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Add images and videos to design files with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040028034"
        feature_name: "Add images and videos to design files"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Add images and videos to design files"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Add images and videos to design files inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040028034"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041089973.v0"
    source_feature_id: "figma.platform.leaf.360041089973"
    feature_name: "Bulk add images and videos"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Bulk add images and videos to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Bulk add images and videos with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Bulk add images and videos with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041089973"
        feature_name: "Bulk add images and videos"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Bulk add images and videos"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Bulk add images and videos inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041089973"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041486873.v0"
    source_feature_id: "figma.platform.leaf.360041486873"
    feature_name: "Use animated GIFs in prototypes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Use animated GIFs in prototypes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Use animated GIFs in prototypes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Use animated GIFs in prototypes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041486873"
        feature_name: "Use animated GIFs in prototypes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Use animated GIFs in prototypes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Use animated GIFs in prototypes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041486873"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040030374.v0"
    source_feature_id: "figma.platform.leaf.360040030374"
    feature_name: "Copy assets between design tools"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Copy assets between design tools to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Copy assets between design tools with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Copy assets between design tools with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040030374"
        feature_name: "Copy assets between design tools"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Copy assets between design tools"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Copy assets between design tools inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040030374"
  - feature_use_card_id: "fuc.figma.platform.leaf.360041003114.v0"
    source_feature_id: "figma.platform.leaf.360041003114"
    feature_name: "Import files to the file browser"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Import files to the file browser to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Import files to the file browser with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Import files to the file browser with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360041003114"
        feature_name: "Import files to the file browser"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Import files to the file browser"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Import files to the file browser inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-imports-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360041003114"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040667874-use-blend-modes-to-create-unique-effects.v0"
    source_feature_id: "figma.platform.leaf.360040667874-use-blend-modes-to-create-unique-effects"
    feature_name: "blend modes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use blend modes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named blend modes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute blend modes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040667874-use-blend-modes-to-create-unique-effects"
        feature_name: "blend modes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / blend modes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover blend modes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-formats-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040667874-Use-blend-modes-to-create-unique-effects"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040027794.v0"
    source_feature_id: "figma.platform.leaf.360040027794"
    feature_name: "importing content into Figma ?"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use importing content into Figma ? to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named importing content into Figma ? with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute importing content into Figma ? with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040027794"
        feature_name: "importing content into Figma ?"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / importing content into Figma ?"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover importing content into Figma ? inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-formats-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040027794"
  - feature_use_card_id: "fuc.figma.platform.leaf.360039825114.v0"
    source_feature_id: "figma.platform.leaf.360039825114"
    feature_name: "Learn more about color profiles and color management ?"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Learn more about color profiles and color management ? to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Learn more about color profiles and color management ? with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Learn more about color profiles and color management ? with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360039825114"
        feature_name: "Learn more about color profiles and color management ?"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Learn more about color profiles and color management ?"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Learn more about color profiles and color management ? inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-formats-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360039825114"
  - feature_use_card_id: "fuc.figma.platform.leaf.360049283914.v0"
    source_feature_id: "figma.platform.leaf.360049283914"
    feature_name: "apply inside, center or outside strokes"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use apply inside, center or outside strokes to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named apply inside, center or outside strokes with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute apply inside, center or outside strokes with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360049283914"
        feature_name: "apply inside, center or outside strokes"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / apply inside, center or outside strokes"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover apply inside, center or outside strokes inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-formats-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360049283914"
  - feature_use_card_id: "fuc.figma.platform.leaf.360040045574.v0"
    source_feature_id: "figma.platform.leaf.360040045574"
    feature_name: "restricting copying and sharing on files ?"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use restricting copying and sharing on files ? to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named restricting copying and sharing on files ? with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute restricting copying and sharing on files ? with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360040045574"
        feature_name: "restricting copying and sharing on files ?"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / restricting copying and sharing on files ?"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover restricting copying and sharing on files ? inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-static-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360040045574"
  - feature_use_card_id: "fuc.figma.platform.leaf.22012921621015.v0"
    source_feature_id: "figma.platform.leaf.22012921621015"
    feature_name: "export or download assets in Dev Mode"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use export or download assets in Dev Mode to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named export or download assets in Dev Mode with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute export or download assets in Dev Mode with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.22012921621015"
        feature_name: "export or download assets in Dev Mode"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / export or download assets in Dev Mode"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover export or download assets in Dev Mode inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-static-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/22012921621015"
  - feature_use_card_id: "fuc.figma.platform.leaf.13402894554519.v0"
    source_feature_id: "figma.platform.leaf.13402894554519"
    feature_name: "Figma's export formats and settings ?"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Figma's export formats and settings ? to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Figma's export formats and settings ? with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Figma's export formats and settings ? with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.13402894554519"
        feature_name: "Figma's export formats and settings ?"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Figma's export formats and settings ?"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Figma's export formats and settings ? inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-export-static-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/13402894554519"
  - feature_use_card_id: "fuc.figma.platform.leaf.developers-apps.v0"
    source_feature_id: "figma.platform.leaf.developers-apps"
    feature_name: "My apps"
    source_apps: ["Figma Developer Platform"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_developer_platform"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use My apps to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named My apps with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_developer_platform"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute My apps with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Developer Platform"
        feature_id: "figma.platform.leaf.developers-apps"
        feature_name: "My apps"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / My apps"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover My apps inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-api-docs-jina.md"
        url: "https://www.figma.com/developers/apps"
  - feature_use_card_id: "fuc.figma.platform.leaf.release-notes.v0"
    source_feature_id: "figma.platform.leaf.release-notes"
    feature_name: "Skip to main content"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Skip to main content as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Skip to main content with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Skip to main content with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.release-notes"
        feature_name: "Skip to main content"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Skip to main content"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Skip to main content inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://www.figma.com/release-notes/"
  - feature_use_card_id: "fuc.figma.platform.leaf.39715554287255-search-the-web-in-figma-make.v0"
    source_feature_id: "figma.platform.leaf.39715554287255-search-the-web-in-figma-make"
    feature_name: "Learn more about web search"
    source_apps: ["Figma Make"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_make"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Learn more about web search as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named Learn more about web search with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_make"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Learn more about web search with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Make"
        feature_id: "figma.platform.leaf.39715554287255-search-the-web-in-figma-make"
        feature_name: "Learn more about web search"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / Learn more about web search"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Learn more about web search inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/39715554287255-Search-the-web-in-Figma-Make"
  - feature_use_card_id: "fuc.figma.platform.leaf.34932042346775-how-do-i-access-the-ai-agent-beta-in-figma-design.v0"
    source_feature_id: "figma.platform.leaf.34932042346775-how-do-i-access-the-ai-agent-beta-in-figma-design"
    feature_name: "access the agent beta"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use access the agent beta as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named access the agent beta with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute access the agent beta with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.34932042346775-how-do-i-access-the-ai-agent-beta-in-figma-design"
        feature_name: "access the agent beta"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / access the agent beta"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover access the agent beta inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/34932042346775-How-do-I-access-the-AI-agent-beta-in-Figma-Design"
  - feature_use_card_id: "fuc.figma.platform.leaf.31242876956183-manage-web-publishing-for-an-organization.v0"
    source_feature_id: "figma.platform.leaf.31242876956183-manage-web-publishing-for-an-organization"
    feature_name: "Learn more in the help center."
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Learn more in the help center. to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Learn more in the help center. with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Learn more in the help center. with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.31242876956183-manage-web-publishing-for-an-organization"
        feature_name: "Learn more in the help center."
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Learn more in the help center."
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Learn more in the help center. inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/31242876956183-Manage-web-publishing-for-an-organization"
  - feature_use_card_id: "fuc.figma.platform.leaf.5601429983767-guide-to-the-figma-desktop-app.v0"
    source_feature_id: "figma.platform.leaf.5601429983767-guide-to-the-figma-desktop-app"
    feature_name: "Learn more in the help center."
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Learn more in the help center. to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Learn more in the help center. with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Learn more in the help center. with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.5601429983767-guide-to-the-figma-desktop-app"
        feature_name: "Learn more in the help center."
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Learn more in the help center."
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Learn more in the help center. inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/5601429983767-Guide-to-the-Figma-desktop-app"
  - feature_use_card_id: "fuc.figma.platform.leaf.360038510833-create-a-community-profile.v0"
    source_feature_id: "figma.platform.leaf.360038510833-create-a-community-profile"
    feature_name: "Learn more about Community profiles"
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Learn more about Community profiles to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Learn more about Community profiles with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Learn more about Community profiles with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.360038510833-create-a-community-profile"
        feature_name: "Learn more about Community profiles"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Learn more about Community profiles"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Learn more about Community profiles inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/360038510833-Create-a-Community-profile"
  - feature_use_card_id: "fuc.figma.platform.leaf.40826832449303-capture-web-pages-to-layers-with-the-figma-chrome-extension.v0"
    source_feature_id: "figma.platform.leaf.40826832449303-capture-web-pages-to-layers-with-the-figma-chrome-extension"
    feature_name: "Learn more in the help center."
    source_apps: ["Figma Design"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_design"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Learn more in the help center. to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Learn more in the help center. with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_design"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Learn more in the help center. with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Design"
        feature_id: "figma.platform.leaf.40826832449303-capture-web-pages-to-layers-with-the-figma-chrome-extension"
        feature_name: "Learn more in the help center."
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Learn more in the help center."
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Learn more in the help center. inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-release-notes-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/40826832449303-Capture-web-pages-to-layers-with-the-Figma-Chrome-extension"
  - feature_use_card_id: "fuc.figma.figjam.leaf.guide-to-figjam.v0"
    source_feature_id: "figma.figjam.leaf.guide-to-figjam"
    feature_name: "Guide to FigJam"
    source_apps: ["FigJam"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figjam_canvas"
    studio_surface: "StudioPageSpread"
    primitive_domain: "page_layout"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Guide to FigJam to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph."
    user_goal: "A Studio operator can perform the source workflow named Guide to FigJam with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figjam_canvas"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_primitive"
    expected_result: "Studio can represent or execute Guide to FigJam with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "FigJam"
        feature_id: "figma.figjam.leaf.guide-to-figjam"
        feature_name: "Guide to FigJam"
    command_contract_refs: ["needs_contract.page-layout.v0"]
    verification_refs: ["needs_fixture.page-layout.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioPageSpread / Guide to FigJam"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Guide to FigJam inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-figjam-guide-to-figjam-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/1500004362321-Guide-to-FigJam"
  - feature_use_card_id: "fuc.figma.figjam.leaf.import-export.v0"
    source_feature_id: "figma.figjam.leaf.import-export"
    feature_name: "Import and export with FigJam"
    source_apps: ["FigJam"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figjam_import_export"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Import and export with FigJam to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Import and export with FigJam with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figjam_import_export"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Import and export with FigJam with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "FigJam"
        feature_id: "figma.figjam.leaf.import-export"
        feature_name: "Import and export with FigJam"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Import and export with FigJam"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Import and export with FigJam inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-figjam-import-export-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/1500007927941-Import-and-export-with-FigJam"
  - feature_use_card_id: "fuc.figma.figjam.leaf.spreadsheet-data.v0"
    source_feature_id: "figma.figjam.leaf.spreadsheet-data"
    feature_name: "Import spreadsheet data, images, and designs to FigJam"
    source_apps: ["FigJam"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figjam_import_export"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Import spreadsheet data, images, and designs to FigJam to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Import spreadsheet data, images, and designs to FigJam with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figjam_import_export"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Import spreadsheet data, images, and designs to FigJam with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "FigJam"
        feature_id: "figma.figjam.leaf.spreadsheet-data"
        feature_name: "Import spreadsheet data, images, and designs to FigJam"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Import spreadsheet data, images, and designs to FigJam"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Import spreadsheet data, images, and designs to FigJam inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-figjam-spreadsheet-data-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/4407533721239-Import-spreadsheet-data-images-and-designs-to-FigJam"
  - feature_use_card_id: "fuc.figma.figjam.leaf.media.v0"
    source_feature_id: "figma.figjam.leaf.media"
    feature_name: "Place images, video, and GIFs in FigJam"
    source_apps: ["FigJam"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figjam_media"
    studio_surface: "StudioFileIO"
    primitive_domain: "file_io"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "compatibility_shim"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Place images, video, and GIFs in FigJam to preserve compatibility with existing creative file and asset workflows through explicit import/export diagnostics."
    user_goal: "A Studio operator can perform the source workflow named Place images, video, and GIFs in FigJam with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figjam_media"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "compatibility_shim"
    expected_result: "Studio can represent or execute Place images, video, and GIFs in FigJam with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "FigJam"
        feature_id: "figma.figjam.leaf.media"
        feature_name: "Place images, video, and GIFs in FigJam"
    command_contract_refs: ["needs_contract.file-io.v0"]
    verification_refs: ["needs_fixture.file-io.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioFileIO / Place images, video, and GIFs in FigJam"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Place images, video, and GIFs in FigJam inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-figjam-media-jina.md"
        url: "https://help.figma.com/hc/en-us/articles/1500004290881-Place-images-video-and-GIFs-in-FigJam"
  - feature_use_card_id: "fuc.figma.motion.leaf.category.v0"
    source_feature_id: "figma.motion.leaf.category"
    feature_name: "Figma Motion timeline, keyframes, easing, anchors, and preset animations"
    source_apps: ["Figma Motion"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_motion"
    studio_surface: "StudioInteractiveDocumentSurface"
    primitive_domain: "interactive"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Figma Motion timeline, keyframes, easing, anchors, and preset animations to define prototype, presentation, motion, animation, or runtime interaction behavior in Studio."
    user_goal: "A Studio operator can perform the source workflow named Figma Motion timeline, keyframes, easing, anchors, and preset animations with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_motion"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_primitive"
    expected_result: "Studio can represent or execute Figma Motion timeline, keyframes, easing, anchors, and preset animations with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Motion"
        feature_id: "figma.motion.leaf.category"
        feature_name: "Figma Motion timeline, keyframes, easing, anchors, and preset animations"
    command_contract_refs: ["needs_contract.interactive.v0"]
    verification_refs: ["needs_fixture.interactive.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioInteractiveDocumentSurface / Figma Motion timeline, keyframes, easing, anchors, and preset animations"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Figma Motion timeline, keyframes, easing, anchors, and preset animations inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-motion-category-jina.md"
        url: "https://help.figma.com/hc/en-us/categories/41274596092695-Figma-Motion"
  - feature_use_card_id: "fuc.figma.slides.leaf.category.v0"
    source_feature_id: "figma.slides.leaf.category"
    feature_name: "Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export"
    source_apps: ["Figma Slides"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_slides"
    studio_surface: "StudioInteractiveDocumentSurface"
    primitive_domain: "interactive"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "must_preserve_existing_format_compatibility"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export to define prototype, presentation, motion, animation, or runtime interaction behavior in Studio."
    user_goal: "A Studio operator can perform the source workflow named Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_slides"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_primitive"
    expected_result: "Studio can represent or execute Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Slides"
        feature_id: "figma.slides.leaf.category"
        feature_name: "Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export"
    command_contract_refs: ["needs_contract.interactive.v0"]
    verification_refs: ["needs_fixture.interactive.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioInteractiveDocumentSurface / Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Slide decks, templates, prototypes in slides, presenter notes, presentation, PowerPoint import, and export inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-slides-category-jina.md"
        url: "https://help.figma.com/hc/en-us/categories/24146015318551-Figma-Slides"
  - feature_use_card_id: "fuc.figma.sites.leaf.category.v0"
    source_feature_id: "figma.sites.leaf.category"
    feature_name: "Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish"
    source_apps: ["Figma Sites"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_sites"
    studio_surface: "StudioPageSpread"
    primitive_domain: "page_layout"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish to control canvas, frame, page, board, slide, site, or layout structures in the local Studio document graph."
    user_goal: "A Studio operator can perform the source workflow named Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_sites"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Sites"
        feature_id: "figma.sites.leaf.category"
        feature_name: "Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish"
    command_contract_refs: ["needs_contract.page-layout.v0"]
    verification_refs: ["needs_fixture.page-layout.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioPageSpread / Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Responsive sites, breakpoints, blocks, embeds, CMS, interactions, preview, and publish inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-sites-category-jina.md"
        url: "https://help.figma.com/hc/en-us/categories/31823555275671-Figma-Sites"
  - feature_use_card_id: "fuc.figma.buzz.leaf.category.v0"
    source_feature_id: "figma.buzz.leaf.category"
    feature_name: "On-brand asset production workflows and templates"
    source_apps: ["Figma Buzz"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_buzz"
    studio_surface: "StudioActionGraph"
    primitive_domain: "automation"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use On-brand asset production workflows and templates to automate, inspect, hand off, or integrate Studio documents through typed local commands and extension surfaces."
    user_goal: "A Studio operator can perform the source workflow named On-brand asset production workflows and templates with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_buzz"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_primitive"
    expected_result: "Studio can represent or execute On-brand asset production workflows and templates with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Buzz"
        feature_id: "figma.buzz.leaf.category"
        feature_name: "On-brand asset production workflows and templates"
    command_contract_refs: ["needs_contract.automation.v0"]
    verification_refs: ["needs_fixture.automation.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioActionGraph / On-brand asset production workflows and templates"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover On-brand asset production workflows and templates inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-buzz-category-jina.md"
        url: "https://help.figma.com/hc/en-us/categories/31194838351767-Figma-Buzz"
  - feature_use_card_id: "fuc.figma.build.leaf.dev-mode.v0"
    source_feature_id: "figma.build.leaf.dev-mode"
    feature_name: "Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP"
    source_apps: ["Build with Figma"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_build"
    studio_surface: "StudioActionGraph"
    primitive_domain: "automation"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP to automate, inspect, hand off, or integrate Studio documents through typed local commands and extension surfaces."
    user_goal: "A Studio operator can perform the source workflow named Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_build"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Build with Figma"
        feature_id: "figma.build.leaf.dev-mode"
        feature_name: "Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP"
    command_contract_refs: ["needs_contract.automation.v0"]
    verification_refs: ["needs_fixture.automation.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioActionGraph / Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Dev Mode inspect, measurements, annotations, code snippets, Code Connect, VS Code, and MCP inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-build-category-jina.md"
        url: "https://help.figma.com/hc/en-us/categories/41306509921687-Build-with-Figma"
  - feature_use_card_id: "fuc.figma.ai.leaf.category.v0"
    source_feature_id: "figma.ai.leaf.category"
    feature_name: "AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors"
    source_apps: ["Figma AI"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_ai"
    studio_surface: "StudioModelToolContract"
    primitive_domain: "ai"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors as a provider-neutral or local-model-assisted Studio workflow with explicit receipts and no cloud dependency in the core."
    user_goal: "A Studio operator can perform the source workflow named AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_ai"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma AI"
        feature_id: "figma.ai.leaf.category"
        feature_name: "AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors"
    command_contract_refs: ["needs_contract.ai.v0"]
    verification_refs: ["needs_fixture.ai.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioModelToolContract / AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover AI workflows, agents, search, image text prototype assistance, custom skills, attachments, MCP connectors inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-ai-section-jina.md"
        url: "https://help.figma.com/hc/en-us/sections/24369548041111"
  - feature_use_card_id: "fuc.figma.draw.leaf.category.v0"
    source_feature_id: "figma.draw.leaf.category"
    feature_name: "Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector"
    source_apps: ["Figma Draw"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_draw"
    studio_surface: "StudioVectorPathGraph"
    primitive_domain: "vector"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "local_primitive"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector to create, edit, transform, or inspect vector geometry in Studio without relying on a vendor-named tool surface."
    user_goal: "A Studio operator can perform the source workflow named Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_draw"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "local_primitive"
    expected_result: "Studio can represent or execute Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Draw"
        feature_id: "figma.draw.leaf.category"
        feature_name: "Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector"
    command_contract_refs: ["needs_contract.vector.v0"]
    verification_refs: ["needs_fixture.vector.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioVectorPathGraph / Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Illustration tools, brushes, transforms, textures, vectorize, recolor, shape builder, simplify vector inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-draw-section-jina.md"
        url: "https://help.figma.com/hc/en-us/sections/31830768959511"
  - feature_use_card_id: "fuc.figma.community.leaf.category.v0"
    source_feature_id: "figma.community.leaf.category"
    feature_name: "Community resources, templates, plugins, widgets, shaders, duplicate and publish flows"
    source_apps: ["Figma Community"]
    source_inventory: "23-figma-leaf-index.md"
    source_category: "figma_community"
    studio_surface: "StudioCollaborationSession"
    primitive_domain: "collaboration"
    naming_posture: "handshake_native_name_with_vendor_source_refs"
    file_format_compatibility: "not_applicable_runtime_state_command"
    local_first_posture: "built_in_no_cloud_required_rust_forward"
    provider_posture: "provider_adapter_or_local_model_candidate"
    explanation_status: "toc_inferred"
    manual_entry_status: "planning_only"
    implementation_readiness: "needs_command_contract_promotion"
    purpose: "Use Community resources, templates, plugins, widgets, shaders, duplicate and publish flows to reproduce collaborative workflow behavior through local-first CRDT/EventLedger state, attribution, and recoverable receipts."
    user_goal: "A Studio operator can perform the source workflow named Community resources, templates, plugins, widgets, shaders, duplicate and publish flows with Handshake-native commands, local state, receipts, and recovery."
    when_to_use:
      - "When equivalent source-app capability is required in Studio."
      - "When the operation needs a typed Rust command contract and no-context UserManual topic."
      - "When local-first state, file compatibility, provenance, and diagnostics must be explicit."
    typical_workflow:
      - "Open or select the Studio document, frame, board, artboard, layer, path, component, asset, or region that the operation targets."
      - "Choose the Handshake-native Studio command mapped from this Feature Use Card."
      - "Set explicit options for target scope, compatibility mode, preview/dry-run behavior, and provider posture."
      - "Preview or validate when supported, then apply the operation."
      - "Inspect the command receipt, diagnostics, document state, and linked internal UserManual topic."
    key_options:
      source_category: "figma_community"
      target_scope: "document|page|board|slide|artboard|frame|layer|path|component|asset|provider_task"
      compatibility_mode: "required_for_import_export_or_migration_paths"
      preview_mode: "required_when_output_or_destructive_state_changes"
      provider_posture: "provider_adapter_or_local_model_candidate"
    expected_result: "Studio can represent or execute Community resources, templates, plugins, widgets, shaders, duplicate and publish flows with visible state changes, undo/replay behavior, diagnostics, and a command receipt."
    common_mistakes:
      - "Using the vendor feature name as the shipped Studio command name instead of a Handshake-native name."
      - "Skipping source-page or app-behavior inspection before implementation."
      - "Losing compatibility, provenance, or unsupported-feature diagnostics in import/export paths."
    edge_cases:
      - "Source app option variants that are not expressible in the initial Studio primitive."
      - "Cloud/provider/account behavior must not become a core dependency for local Studio operation."
      - "Round-trip compatibility may require explicit lossy/unsupported-feature receipts."
    recovery_steps:
      - "Use Studio undo or restore the previous document snapshot."
      - "Inspect the command receipt, target scope, compatibility mode, and provider diagnostics."
      - "Rerun with corrected options or fall back to a documented compatibility shim."
    handshake_tool_design_notes:
      - "Promote this card into a typed Rust Studio command only after exact behavior is specified and tested."
      - "Keep vendor names in source_refs, migration notes, and compatibility fixtures only."
      - "Core operation must remain local-first and no-cloud-required; provider adapters are optional edges."
    equivalent_features:
      - app: "Figma Community"
        feature_id: "figma.community.leaf.category"
        feature_name: "Community resources, templates, plugins, widgets, shaders, duplicate and publish flows"
    command_contract_refs: ["needs_contract.collaboration.v0"]
    verification_refs: ["needs_fixture.collaboration.v0"]
    user_manual_handoff:
      topic_candidate: "Studio / StudioCollaborationSession / Community resources, templates, plugins, widgets, shaders, duplicate and publish flows"
      required_when: "same_change_as_product_behavior_implementation"
      must_explain: ["purpose", "when_to_use", "inputs", "outputs", "operator_path", "model_path", "failure_modes", "recovery", "verification", "receipt_links", "compatibility_limits"]
      no_context_operator_note: "Explain how to use and recover Community resources, templates, plugins, widgets, shaders, duplicate and publish flows inside local-first Studio without relying on vendor product naming or cloud state."
    source_refs:
      - label: "figma-community-category-jina.md"
        url: "https://help.figma.com/hc/en-us/categories/360002772634-Community"
```

</topic>

### [SFR-FIGMA-USE-CARDS.sources] Sources

```yaml
sources:
  - { id: FIG-FUC-S01, path: "23-figma-leaf-index.md", note: "Source inventory used to generate Figma Feature Use Cards." }
  - { id: FIG-FUC-S02, path: "_source_snapshots/figma-design-category-jina.md", note: "Official Figma Design category snapshot." }
  - { id: FIG-FUC-S03, path: "_source_snapshots/figma-make-category-jina.md", note: "Official Figma Make category snapshot." }
  - { id: FIG-FUC-S04, path: "_source_snapshots/figma-import-export-jina.md", note: "Official Figma import/export snapshot." }
  - { id: FIG-FUC-S05, path: "_source_snapshots/figma-export-formats-jina.md", note: "Official Figma export format snapshot." }
  - { id: FIG-FUC-S06, path: "_source_snapshots/figma-api-docs-jina.md", note: "Official Figma developer docs snapshot." }
```
