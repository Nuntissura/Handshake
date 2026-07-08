---
file_id: "indesign-expanded-count-ledger"
topic_id: SFR-INDESIGN-EXPANDED-COUNTS
title: "InDesign Expanded Count Ledger"
status: draft
summary: "Expanded online-source count ledger for InDesign desktop."
sources: 10
updated_at: "2026-07-05"
---

## [SFR-INDESIGN-EXPANDED-COUNTS] InDesign Expanded Count Ledger

### [SFR-INDESIGN-EXPANDED-COUNTS.counts] Count Ledger

```yaml
count_status: "ONLINE_SOURCE_DISTILLED_SEED"
reason: "Adobe Help, shortcut rows, file-format rows, and DOM pages are separate online ledgers that can be distilled into a Studio rebuild target; installed InDesign exports enrich exact ids and context states."
public_source_counts:
  help_feature_leaves: 542
  help_support_context_leaves: 61
  rendered_help_nav_links_agent_count: 603
  rendered_help_nav_links_after_obvious_exclusions_agent_count: 596
  keyboard_shortcut_rows_agent_count: 190
  shortcut_tool_rows_agent_count: 41
  named_visible_or_shortcut_addressable_tools_agent_count: 28
  supported_file_format_rows_agent_count: 48
  open_format_rows: 5
  save_as_format_rows: 2
  export_rows_in_help_file_format_page: 15
  place_import_rows: 22
  dom_api_markdown_pages_adobedocs_agent_count: 1080
  dom_api_markdown_pages_excluding_index_agent_count: 1079
  adobedocs_total_markdown_pages_agent_count: 1465
  dom_export_format_enum_constants: 16
  dom_versions_listed: 27
local_snapshot_counts:
  tool_shortcut_labels_heuristic: 36
  public_shortcut_rows_heuristic: 613
  developer_reference_links_in_snapshot: 1446
count_interpretation:
  keyboard_shortcut_rows_agent_count: "shortcut/action rows, not all commands"
  named_visible_or_shortcut_addressable_tools_agent_count: "conservative public shortcut-table tool proxy"
  dom_api_markdown_pages_adobedocs_agent_count: "scripting/API documentation pages, not UI features"
  local_snapshot_heuristics: "diagnostic only; agent row counts are preferred where manually verified"
```

### [SFR-INDESIGN-EXPANDED-COUNTS.installed-enrichment] Installed Enrichment

```yaml
optional_installed_enrichment_exports:
  - id: "id-installed-show-set"
    source_path: "Edit > Keyboard Shortcuts > Show Set"
    expected_output: "printable shortcut set"
    closes: ["shortcut_row", "menu_command_proxy"]
  - id: "id-installed-menu-actions"
    source_path: "InDesign DOM app.menuActions, app.scriptMenuActions, app.menus"
    expected_output: "menu action manifest with ids, names, areas, enabled/context states"
    closes: ["menu_command"]
  - id: "id-installed-panels"
    source_path: "InDesign DOM app.panels"
    expected_output: "panel manifest"
    closes: ["panel_or_panel_menu"]
  - id: "id-installed-toolbox-tools"
    source_path: "InDesign DOM app.toolBoxTools"
    expected_output: "toolbox tool manifest"
    closes: ["toolbar_tool"]
  - id: "id-file-dialog-options"
    source_path: "export/import preference APIs plus dialog fixture capture"
    expected_output: "format-specific option matrix"
    closes: ["import_export_option"]
```

### [SFR-INDESIGN-EXPANDED-COUNTS.sources] Sources

```yaml
sources:
  - { id: IDX-S01, path: "07-indesign-leaf-index.md", note: "Current InDesign help leaf inventory." }
  - { id: IDX-S02, path: "17-indesign-feature-use-cards.md", note: "Current InDesign Feature Use Cards." }
  - { id: IDX-S03, path: "_source_snapshots/indesign-keyboard-shortcuts-jina.md", note: "InDesign keyboard shortcuts snapshot." }
  - { id: IDX-S04, path: "_source_snapshots/indesign-tools-jina.md", note: "InDesign toolbox/help snapshot." }
  - { id: IDX-S05, path: "_source_snapshots/indesign-supported-file-formats-jina.md", note: "InDesign file format snapshot." }
  - { id: IDX-S06, path: "_source_snapshots/indesign-scripting-jina.md", note: "InDesign scripting Help snapshot." }
  - { id: IDX-S07, path: "_source_snapshots/indesign-uxp-dom-api-jina.md", note: "InDesign UXP/DOM API snapshot." }
  - { id: IDX-S08, url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/dom-versioning/", note: "InDesign DOM versioning and require() posture." }
  - { id: IDX-S09, url: "https://developer.adobe.com/indesign/uxp/dom/api/a/application/", note: "Application collections including panels, menuActions, menus, scriptMenuActions, and toolBoxTools." }
  - { id: IDX-S10, url: "https://github.com/AdobeDocs/uxp-indesign", note: "Official AdobeDocs source repository for InDesign UXP docs." }
```
