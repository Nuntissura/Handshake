---
file_id: "adobe-installed-ui-export-playbook"
topic_id: SFR-ADOBE-INSTALLED-UI-EXPORT
title: "Adobe Installed UI Export Playbook"
status: draft
summary: "Optional installed-app enrichment path for IDs, shortcuts, panels, context states, and file-dialog fixture detail."
sources: 4
updated_at: "2026-07-05"
---

## [SFR-ADOBE-INSTALLED-UI-EXPORT] Adobe Installed UI Export Playbook

### [SFR-ADOBE-INSTALLED-UI-EXPORT.purpose] Purpose

```yaml
purpose:
  goal: "Enrich the online-source distilled Photoshop, InDesign, and Illustrator inventories with installed IDs, shortcuts, panels, context states, and file-dialog fixture detail."
  rule: "Online sources define the source-distilled feature/tool rebuild inventory. Installed-app exports are verification and enrichment, not a blocker to documenting all source-observable features/tools."
  output_location: ".GOV/reference/studio_app_feature_research/_installed_exports/"
  status_label_before_exports: "ONLINE_SOURCE_DISTILLED"
  status_label_after_exports: "ONLINE_SOURCE_DISTILLED_WITH_INSTALLED_ID_ENRICHMENT"
```

### [SFR-ADOBE-INSTALLED-UI-EXPORT.schema] Export Schema

```yaml
export_record_schema:
  app: "photoshop|indesign|illustrator"
  app_version: "exact installed version"
  platform: "windows|macos"
  locale: "exact locale"
  workspace: "workspace/profile used during export"
  document_context: "no_document|blank_document|raster_doc|vector_doc|layout_doc|opened_fixture"
  source_surface: "shortcut_summary|menu_actions|menus|panel_list|panel_menu|toolbar|toolbox|context_menu|scripting_api|file_dialog"
  source_method: "manual_export|script_export|ui_capture|api_manifest|fixture_dialog_capture"
  stable_source_id: "machine stable id"
  display_name: "visible name"
  parent_path: "menu/panel/tool group path"
  command_id: "vendor/internal id if available"
  shortcut_windows: "optional"
  shortcut_macos: "optional"
  enabled_state: "enabled|disabled|contextual|unknown"
  provider_posture: "local_primitive|provider_adapter|compatibility_shim|unknown"
  studio_surface_candidate: "Handshake-native target surface"
  source_refs: []
```

### [SFR-ADOBE-INSTALLED-UI-EXPORT.app-steps] Per-App Steps

```yaml
photoshop:
  required_exports:
    - "Edit > Keyboard Shortcuts > Summarize for Application Menus, Panel Menus, Tools"
    - "Edit > Menus / Keyboard Shortcuts & Menus application and panel menu sets"
    - "Edit > Toolbar default and extra tools"
    - "Window panel list and panel flyout captures"
    - "UXP Photoshop DOM and batchPlay/action descriptor inventory"
    - "File open/save/export/import dialog option fixtures"
indesign:
  required_exports:
    - "Edit > Keyboard Shortcuts > Show Set"
    - "DOM script: app.menuActions"
    - "DOM script: app.scriptMenuActions"
    - "DOM script: app.menus"
    - "DOM script: app.panels"
    - "DOM script: app.toolBoxTools"
    - "export/import preference APIs and dialog option fixtures"
illustrator:
  required_exports:
    - "Edit > Keyboard Shortcuts > Menu Commands and Tools export"
    - "Edit Toolbar / All Tools drawer inventory"
    - "Window menu and panel flyout captures"
    - "Actions panel recordable command capture"
    - "JavaScript object model class/property/method manifest"
    - "file open/place/save/export dialog option fixtures"
```

### [SFR-ADOBE-INSTALLED-UI-EXPORT.acceptance] Acceptance

```yaml
acceptance:
  - "Each app ledger records exact app version, platform, locale, workspace, and document context."
  - "Every exported command/tool/panel row has a stable source id."
  - "Public-source rows and installed-export rows are reconciled without replacing one count semantic with another."
  - "Rows that require provider/cloud/account behavior are marked provider_adapter or compatibility_shim."
  - "File-format rows have fixture and round-trip expectations before implementation parity is claimed."
  - "Installed exports may enrich the app ledger with exact ids, context states, and version/platform evidence, but online-source distillation remains valid rebuild planning input before enrichment."
```

### [SFR-ADOBE-INSTALLED-UI-EXPORT.sources] Sources

```yaml
sources:
  - { id: AIE-S01, path: "28-adobe-count-methodology.md", note: "Count semantics and proof policy." }
  - { id: AIE-S02, path: "29-photoshop-expanded-count-ledger.md", note: "Photoshop expanded count ledger." }
  - { id: AIE-S03, path: "30-indesign-expanded-count-ledger.md", note: "InDesign expanded count ledger." }
  - { id: AIE-S04, path: "31-illustrator-expanded-count-ledger.md", note: "Illustrator expanded count ledger." }
```
