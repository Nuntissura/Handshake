---
file_id: "photoshop-expanded-count-ledger"
topic_id: SFR-PHOTOSHOP-EXPANDED-COUNTS
title: "Photoshop Expanded Count Ledger"
status: draft
summary: "Expanded online-source count ledger for Photoshop desktop and Camera Raw."
sources: 12
updated_at: "2026-07-05"
---

## [SFR-PHOTOSHOP-EXPANDED-COUNTS] Photoshop Expanded Count Ledger

### [SFR-PHOTOSHOP-EXPANDED-COUNTS.counts] Count Ledger

```yaml
count_status: "ONLINE_SOURCE_DISTILLED_SEED"
reason: "Online Adobe sources are sufficient to distill the Studio rebuild target; installed exports enrich exact ids, context states, and shortcut files."
public_source_counts:
  help_feature_leaves: 441
  help_support_context_leaves: 20
  desktop_help_nav_entries_observed_by_agent: 461
  keyboard_shortcut_categories_visible_publicly: 3
  keyboard_shortcut_categories:
    - "Application Menus"
    - "Panel Menus"
    - "Tools"
  toolbar_management_workflow_commands_observed: 7
  supported_file_format_entries_observed_by_agent: 66
  bit_depth_16_format_support_entries: 10
  bit_depth_32_format_support_entries: 7
  camera_raw_shortcut_filter_categories_excluding_all: 14
  camera_raw_shortcut_rows_agent_estimate: 164
  camera_raw_select_tools_rows: 22
  camera_raw_edit_panels: 9
  camera_raw_masking_types_or_tools_minimum: 11
  photoshop_uxp_root_modules: 4
  photoshop_uxp_class_pages_agent_minimum: 18
local_snapshot_counts:
  help_leaves_with_tool_in_name: 37
  uxp_class_links_in_local_snapshot: 7
count_interpretation:
  help_feature_leaves: "planning-card source coverage"
  keyboard_shortcut_categories_visible_publicly: "online evidence for menu, panel menu, and tool command source surfaces"
  supported_file_format_entries_observed_by_agent: "compatibility rows, not command count"
  camera_raw_shortcut_rows_agent_estimate: "public Camera Raw shortcut coverage proxy"
```

### [SFR-PHOTOSHOP-EXPANDED-COUNTS.installed-enrichment] Installed Enrichment

```yaml
optional_installed_enrichment_exports:
  - id: "ps-installed-shortcut-summary"
    source_path: "Edit > Keyboard Shortcuts > Shortcuts For: Application Menus, Panel Menus, Tools > Summarize"
    expected_output: "HTML shortcut summaries"
    closes: ["menu_command", "panel_menu_command", "tool_shortcut"]
  - id: "ps-installed-menu-summary"
    source_path: "Edit > Menus or Window > Workspace > Keyboard Shortcuts & Menus"
    expected_output: "application and panel menu inventory"
    closes: ["menu_command", "panel_or_panel_menu"]
  - id: "ps-installed-toolbar-export"
    source_path: "Edit > Toolbar"
    expected_output: "default and extra tools, tool groups, tool presets"
    closes: ["toolbar_tool"]
  - id: "ps-uxp-batchplay-action-catalog"
    source_path: "UXP Photoshop DOM plus batchPlay/action descriptors"
    expected_output: "scriptable actions and DOM surfaces"
    closes: ["scripting_api", "hidden_command_proxy"]
```

### [SFR-PHOTOSHOP-EXPANDED-COUNTS.sources] Sources

```yaml
sources:
  - { id: PSX-S01, path: "06-photoshop-leaf-index.md", note: "Current Photoshop help leaf inventory." }
  - { id: PSX-S02, path: "15-photoshop-feature-use-cards.md", note: "Current Photoshop Feature Use Cards." }
  - { id: PSX-S03, path: "_source_snapshots/photoshop-keyboard-shortcuts-jina.md", note: "Photoshop keyboard shortcut access page snapshot." }
  - { id: PSX-S04, path: "_source_snapshots/photoshop-customizing-keyboard-shortcuts-jina.md", note: "Photoshop shortcut/menu customization snapshot." }
  - { id: PSX-S05, path: "_source_snapshots/photoshop-customize-toolbar-jina.md", note: "Photoshop toolbar customization snapshot." }
  - { id: PSX-S06, path: "_source_snapshots/photoshop-workspace-overview-jina.md", note: "Photoshop workspace overview snapshot." }
  - { id: PSX-S07, path: "_source_snapshots/photoshop-scripting-jina.md", note: "Photoshop scripting Help snapshot." }
  - { id: PSX-S08, path: "_source_snapshots/photoshop-uxp-api-jina.md", note: "Photoshop UXP API snapshot." }
  - { id: PSX-S09, url: "https://helpx.adobe.com/photoshop/desktop/save-and-export/export-files-to-different-formats/supported-file-formats-in-photoshop.html", note: "Photoshop supported file formats." }
  - { id: PSX-S10, url: "https://helpx.adobe.com/camera-raw/using/default-keyboard-shortcuts.html", note: "Camera Raw public shortcuts." }
  - { id: PSX-S11, url: "https://helpx.adobe.com/camera-raw/using/masking.html", note: "Camera Raw masking tools." }
  - { id: PSX-S12, url: "https://developer.adobe.com/photoshop/uxp/2022/ps-reference/", note: "Photoshop UXP API reference." }
```
