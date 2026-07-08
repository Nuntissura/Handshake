---
file_id: "adobe-expanded-count-methodology"
topic_id: SFR-ADOBE-COUNT-METHODOLOGY
title: "Adobe Expanded Count Methodology"
status: draft
summary: "Count semantics and proof rules for distilling Photoshop, InDesign, and Illustrator features/tools from online sources."
sources: 9
updated_at: "2026-07-05"
---

## [SFR-ADOBE-COUNT-METHODOLOGY] Adobe Expanded Count Methodology

### [SFR-ADOBE-COUNT-METHODOLOGY.policy] Count Policy

```yaml
count_policy:
  correction: "Online sources are sufficient to distill the source-observable feature/tool inventory for rebuild planning."
  false_claim_to_avoid: "Do not collapse all source surfaces into only the help-leaf count."
  current_card_semantics: "generated Feature Use Cards are help-leaf or category-inferred planning cards."
  online_source_status: "ONLINE_SOURCE_DISTILLABLE"
  installed_export_status: "optional verification and id enrichment, not a blocker to feature/tool distillation"
  reason: "Adobe Help pages, shortcut rows, toolbar tools, menu commands, panel commands, scripting APIs, file-format rows, and release deltas are separate source surfaces that must be merged into one source-distilled Studio parity inventory."
  required_count_ledgers:
    - help_leaf
    - toolbar_tool
    - shortcut_row
    - menu_command
    - panel_or_panel_menu
    - scripting_api_object
    - scripting_api_property
    - scripting_api_method
    - file_format_action_row
    - import_export_option
    - release_delta
```

### [SFR-ADOBE-COUNT-METHODOLOGY.semantics] Count Semantics

```yaml
count_semantics:
  help_leaf:
    meaning: "Public Adobe Help article/topic leaf."
    use: "Coverage planning and source-page promotion queue."
    warning: "One page can contain many commands; one command can appear on multiple pages."
  toolbar_tool:
    meaning: "Named tool in toolbar/Edit Toolbar/tool-technique surface."
    use: "Native Studio tool planning."
    warning: "Public pages can omit hidden, locale-specific, beta, or workspace-dependent tools."
  shortcut_row:
    meaning: "Public or exported keyboard shortcut row."
    use: "Command proxy and operator shortcut planning."
    warning: "Shortcut rows are command evidence and tool evidence; merge them with help, toolbar, and scripting surfaces rather than treating them as the only command list."
  menu_command:
    meaning: "Installed application menu command or menu action."
    use: "Closest command-level UI count."
    warning: "Online docs and shortcut pages expose many menu commands; installed app export enriches exact ids, context states, and locale/version-specific rows."
  panel_or_panel_menu:
    meaning: "Window panel, panel flyout, contextual panel command, or panel-specific action."
    use: "UI parity and manual topic planning."
    warning: "Public Help and shortcuts expose panel families and common commands; installed exports enrich flyout ids and context-specific states."
  scripting_api:
    meaning: "Developer DOM/API object, property, method, enum, action, or scriptable event."
    use: "Automation and hidden-capability parity."
    warning: "API entities overcount implementation detail and do not map 1:1 to visible user features."
  file_format_action_row:
    meaning: "Format support under open, place/import, save, export, package, screen/web, or media action."
    use: "Compatibility matrix and fixture planning."
    warning: "Supported-format rows do not specify full schema behavior or round-trip fidelity."
```

### [SFR-ADOBE-COUNT-METHODOLOGY.workflow] Closure Workflow

```yaml
closure_workflow:
  public_source_layer:
    status: "primary_distillation_layer"
    produces: "source-distilled feature/tool inventory across help, shortcuts, tools, panels, scripting APIs, file formats, and release deltas"
    can_prove: "online-source feature/tool parity target for Studio rebuild planning"
  installed_app_layer:
    status: "verification_and_enrichment_layer"
    produces:
      - "shortcut/menu command export"
      - "toolbar/Edit Toolbar tool export"
      - "Window panels and panel menus export"
      - "context menus and flyout commands export"
      - "scripting DOM/Object Model manifest"
      - "file-dialog option manifests"
      - "release-delta reconciliation"
  studio_promotion_layer:
    status: "future implementation gate"
    requires:
      - "Handshake-native name"
      - "typed Rust command contract"
      - "manual topic"
      - "fixtures/tests"
      - "receipt/diagnostic surface"
      - "compatibility limits"
```

### [SFR-ADOBE-COUNT-METHODOLOGY.sources] Sources

```yaml
sources:
  - { id: ACM-S01, url: "https://helpx.adobe.com/photoshop/desktop.html", note: "Photoshop Help baseline." }
  - { id: ACM-S02, url: "https://helpx.adobe.com/photoshop/desktop/get-started/settings-and-preferences/view-keyboard-shortcuts.html", note: "Photoshop in-app shortcut inventory categories." }
  - { id: ACM-S03, url: "https://developer.adobe.com/photoshop/uxp/2022/ps-reference/", note: "Photoshop UXP API reference." }
  - { id: ACM-S04, url: "https://helpx.adobe.com/indesign/desktop.html", note: "InDesign Help baseline." }
  - { id: ACM-S05, url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html", note: "InDesign public shortcut page and Show Set guidance." }
  - { id: ACM-S06, url: "https://developer.adobe.com/indesign/uxp/dom/api/", note: "InDesign DOM API reference." }
  - { id: ACM-S07, url: "https://helpx.adobe.com/illustrator/desktop.html", note: "Illustrator Help baseline." }
  - { id: ACM-S08, url: "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html", note: "Illustrator public shortcut rows." }
  - { id: ACM-S09, url: "https://helpx.adobe.com/illustrator/using/tools-in-illustrator.html", note: "Illustrator public tools categories." }
```
