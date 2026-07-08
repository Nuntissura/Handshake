---
file_id: "illustrator-expanded-count-ledger"
topic_id: SFR-ILLUSTRATOR-EXPANDED-COUNTS
title: "Illustrator Expanded Count Ledger"
status: draft
summary: "Expanded online-source count ledger for Illustrator desktop."
sources: 10
updated_at: "2026-07-05"
---

## [SFR-ILLUSTRATOR-EXPANDED-COUNTS] Illustrator Expanded Count Ledger

### [SFR-ILLUSTRATOR-EXPANDED-COUNTS.counts] Count Ledger

```yaml
count_status: "ONLINE_SOURCE_DISTILLED_SEED"
reason: "Adobe public Illustrator Help, Tools, Shortcuts, file-format, scripting, and release pages expose the source surfaces to distill a Studio rebuild target; installed exports enrich exact ids and context states."
public_source_counts:
  help_raw_leaves: 532
  help_feature_leaves: 515
  help_support_context_leaves: 17
  feature_families: 12
  tool_categories: 6
  tool_categories_named: [Draw, Select, Navigate, Paint, Text, Modify]
  tools_page_body_named_tools_agent_count: 34
  tools_page_toc_tool_technique_links: 42
  shortcut_page_tool_rows_agent_count: 42
  public_tool_shortcut_label_union_agent_count: 57
  shortcut_sections_agent_count: 23
  shortcut_rows_agent_count: 391
  unique_shortcut_labels_agent_count: 367
  file_action_format_rows_agent_count: 85
  file_action_format_breakdown:
    open: 29
    place: 28
    save: 6
    export_as: 14
    save_for_web: 3
    export_for_screens: 5
  unique_file_format_labels_agent_count: 32
  unique_extensions_agent_count: 48
  public_scripting_object_classes_agent_count: 138
  public_scripting_option_classes_agent_count: 33
  public_scripting_properties_agent_count: 1449
  public_scripting_methods_agent_count: 379
  latest_release_note_feature_updates_agent_count: 4
  latest_whats_new_story_entries_agent_count: 2
local_snapshot_counts:
  illustrator_public_shortcut_rows_script_count: 390
  illustrator_public_shortcut_sections_script_count: 22
  illustrator_tool_technique_links_script_count: 42
count_interpretation:
  help_feature_leaves: "planning-card source coverage"
  shortcut_rows_agent_count: "public shortcut proxy; Adobe states public list is not exhaustive"
  tools_page_toc_tool_technique_links: "public tool-technique pages under tool categories"
  public_scripting_object_classes_agent_count: "automation surface, not visible tool count"
```

### [SFR-ILLUSTRATOR-EXPANDED-COUNTS.installed-enrichment] Installed Enrichment

```yaml
optional_installed_enrichment_exports:
  - id: "ai-installed-keyboard-shortcut-export"
    source_path: "Edit > Keyboard Shortcuts > Menu Commands and Tools"
    expected_output: "full installed shortcut/menu command list"
    closes: ["shortcut_row", "menu_command"]
  - id: "ai-installed-edit-toolbar-export"
    source_path: "Edit Toolbar / All Tools drawer"
    expected_output: "complete default/hidden toolbar tool inventory"
    closes: ["toolbar_tool"]
  - id: "ai-installed-window-panel-capture"
    source_path: "Window menu and panel flyout menus"
    expected_output: "panel and panel-menu command manifest"
    closes: ["panel_or_panel_menu"]
  - id: "ai-action-recordable-command-capture"
    source_path: "Actions panel command recording and batch options"
    expected_output: "action-recordable command inventory"
    closes: ["menu_command", "automation"]
  - id: "ai-scripting-object-model-export"
    source_path: "Illustrator JavaScript object reference or installed object model export"
    expected_output: "classes, properties, methods, enums, option classes"
    closes: ["scripting_api"]
```

### [SFR-ILLUSTRATOR-EXPANDED-COUNTS.sources] Sources

```yaml
sources:
  - { id: ILX-S01, path: "22-illustrator-leaf-index.md", note: "Current Illustrator help leaf inventory." }
  - { id: ILX-S02, path: "24-illustrator-feature-use-cards.md", note: "Current Illustrator Feature Use Cards." }
  - { id: ILX-S03, path: "_source_snapshots/illustrator-default-keyboard-shortcuts-jina.md", note: "Illustrator public shortcut snapshot." }
  - { id: ILX-S04, path: "_source_snapshots/illustrator-toolbar-jina.md", note: "Illustrator toolbar and tool-technique snapshot." }
  - { id: ILX-S05, path: "_source_snapshots/illustrator-tools-jina.md", note: "Illustrator tools overview snapshot." }
  - { id: ILX-S06, path: "_source_snapshots/illustrator-supported-file-formats-jina.md", note: "Illustrator supported file formats snapshot." }
  - { id: ILX-S07, path: "_source_snapshots/illustrator-release-notes-jina.md", note: "Illustrator release notes snapshot." }
  - { id: ILX-S08, url: "https://ai-scripting.docsforadobe.dev/jsobjref/javascript-object-reference/", note: "Public Illustrator scripting object reference used by source agent." }
  - { id: ILX-S09, url: "https://developer.adobe.com/firefly-services/docs/illustrator/getting-started/concepts/", note: "Adobe Developer Illustrator API/custom-script concepts." }
  - { id: ILX-S10, url: "https://helpx.adobe.com/illustrator/desktop/get-started/preferences-and-settings/customize-keyboard-shortcuts.html", note: "Illustrator installed shortcut dialog and Menu Commands posture." }
```
