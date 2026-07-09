---
file_id: studio-app-feature-research-indesign-deep-feature-delta
file_kind: deep_feature_delta
topic_id: SFR-INDESIGN-DEEP-DELTA
title: "Adobe InDesign Deep Feature Delta"
status: draft
app_key: indesign
updated_at: "2026-07-09"
counts:
  total_rows: 416
  modalities: 15
  new_surface_rows: 35
  deepens_existing_rows: 381
  verified_rows: 413
  unverified_rows: 3
  rows_per_modality:
    tools: 42
    menu-commands: 87
    text-and-typography: 48
    styles: 17
    pages-and-layout: 26
    tables: 15
    graphics-and-frames: 30
    color-and-output: 15
    interactive-and-epub: 26
    long-document: 13
    output-and-prepress: 28
    automation-and-scripting: 20
    panels-and-workspace: 16
    preferences: 23
    cloud-and-collab: 10
---

## [SFR-INDESIGN-DEEP-DELTA] InDesign Deep Feature Delta

### [SFR-INDESIGN-DEEP-DELTA.method] Method and Evidence Posture

```yaml
as_of: "2026-07-09"
purpose: "Go below the help-TOC leaf level of 07-indesign-leaf-index.md into the actual tool/command/panel/option surface of Adobe InDesign desktop."
dedupe_rule: "Rows that add option-level or command-level detail under an existing help leaf carry dedupe_status deepens_existing plus deepens_leaf_id; rows for surfaces absent from the leaf index carry dedupe_status new_surface."
evidence_paths:
  - "_source_snapshots/indesign-keyboard-shortcuts-jina.md (official helpx keyboard-shortcuts page body; tool names, shortcut sets, panel actions)"
  - "_source_snapshots/indesign-tools-jina.md (official helpx toolbox page; TOC-level evidence only, body was JS-rendered)"
  - "_source_snapshots/indesign-supported-file-formats-jina.md (official helpx supported-file-formats page body; File menu Open/Save As/Export/Place/Package format tables)"
  - "_source_snapshots/indesign-scripting-jina.md (official helpx scripting page body; Scripts panel, Script Label panel, script folders, sample scripts)"
  - "_source_snapshots/indesign-uxp-dom-api-jina.md (official developer.adobe.com InDesign UXP docs; scripts, plugins, recipes, UXP API tree)"
  - "07-indesign-leaf-index.md (542 official help leaves used for dedupe anchoring)"
fetch_blockers:
  - "Direct WebFetch to helpx.adobe.com timed out (60s) for all attempted pages on 2026-07-09."
  - "Jina Reader relay returned 422 (HTTP/2 framing error against helpx.adobe.com) on 2026-07-09 (authoring pass)."
  - "web.archive.org is not fetchable from this environment."
  - "Verification pass later on 2026-07-09: Jina Reader relay now returns helpx.adobe.com desktop pages (HTTP 200) but only the JS-shell navigation, not article bodies; legacy /indesign/using/ URLs still 422 via Jina; web.archive.org and help.adobe.com CS-era docs remain unreachable; developer.adobe.com (UXP docs and the scripting DOM API reference at /indesign/dom/api/) is directly fetchable with full bodies."
verification_policy: "VERIFIED = named in a local official snapshot, an official help leaf page title/URL, an explicit web-search snippet enumeration, or a directly fetched official developer.adobe.com page body. UNVERIFIED = option-level detail reconstructed from search snippets or domain knowledge without an inspectable official page body; retained per instruction rather than dropped. 2026-07-09 verification pass upgraded 68 of 71 flagged rows via per-topic web-search snippet enumeration plus direct developer.adobe.com DOM API fetches (DD-S27/DD-S28); the 3 rows still UNVERIFIED (help-menu, window-menu-inventory, view-custom-fonts-review) enumerate menu/panel inventories or hosted-view behavior that no reachable evidence surface confirms at option level."
```

### [SFR-INDESIGN-DEEP-DELTA.tools] Toolbox Tools

```yaml
records:
  - id: "indesign.deep.tools.selection"
    name: "Selection tool"
    record_role: "feature_deep_delta"
    app_behavior: "Selects and moves whole frames and objects; shortcut V or Esc; temporarily invoked from any non-selection tool with Ctrl/Command."
    primitive_domain: "selection_mask"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.view-select-tools"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.direct-selection"
    name: "Direct Selection tool"
    record_role: "feature_deep_delta"
    app_behavior: "Selects frame content and individual path anchor points; shortcut A; Ctrl+Tab toggles between Selection and Direct Selection."
    primitive_domain: "selection_mask"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.view-select-tools"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.group-selection"
    name: "Group Selection mode"
    record_role: "feature_deep_delta"
    app_behavior: "Temporarily selects one object inside a group via Direct Selection tool + Alt/Option without ungrouping."
    primitive_domain: "selection_mask"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.page"
    name: "Page tool"
    record_role: "feature_deep_delta"
    app_behavior: "Selects a page as an object to resize or reposition it and to set per-page liquid layout rules; shortcut Shift+P."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.view-select-tools"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01, DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.gap"
    name: "Gap tool"
    record_role: "feature_deep_delta"
    app_behavior: "Resizes the whitespace gap between adjacent objects, moving all objects that share the gap; shortcut U."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.view-select-tools"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.content-collector"
    name: "Content Collector tool"
    record_role: "feature_deep_delta"
    app_behavior: "Picks up page items into the Content Conveyor for reuse or linked placement in the same or another document."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.link-and-update-content-across-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.content-placer"
    name: "Content Placer tool"
    record_role: "feature_deep_delta"
    app_behavior: "Places items from the Content Conveyor as copies or as linked content with a create-link option so parent edits can be pushed to children."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.link-and-update-content-across-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.type"
    name: "Type tool"
    record_role: "feature_deep_delta"
    app_behavior: "Creates and edits text frames and selects text; shortcut T; drag creates a new text frame."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.view-select-tools"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.type-on-a-path"
    name: "Type on a Path tool"
    record_role: "feature_deep_delta"
    app_behavior: "Flows text along any open or closed path; shortcut Shift+T."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.type-on-a-path.add-delete-text-on-path"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.line"
    name: "Line tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws straight line segments; shortcut backslash; Shift constrains to 45-degree increments."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.draw-lines-and-shapes.draw-basic-lines-and-shapes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.pen"
    name: "Pen tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws Bezier paths with straight and curved segments; shortcut P; auto-switches to add/delete/convert modes over existing paths."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.drawing-tools.draw-lines-with-the-pen-tool"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.add-anchor-point"
    name: "Add Anchor Point tool"
    record_role: "feature_deep_delta"
    app_behavior: "Adds anchor points to an existing path; shortcut equals key."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.add-or-delete-anchor-points"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.delete-anchor-point"
    name: "Delete Anchor Point tool"
    record_role: "feature_deep_delta"
    app_behavior: "Removes anchor points from a path without cutting it; shortcut minus key."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.add-or-delete-anchor-points"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.convert-direction-point"
    name: "Convert Direction Point tool"
    record_role: "feature_deep_delta"
    app_behavior: "Converts anchor points between corner and smooth and edits direction handles; shortcut Shift+C."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.convert-anchor-points"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.pencil"
    name: "Pencil tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws freeform paths that are auto-fit with anchor points; shortcut N; tool options control fidelity and smoothness."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.drawing-tools.draw-with-the-pencil-tool"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.smooth"
    name: "Smooth tool"
    record_role: "feature_deep_delta"
    app_behavior: "Drag along an existing path to reduce anchor points and smooth its curvature; grouped in the Pencil tool flyout."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.edit-and-reshape-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.erase"
    name: "Erase tool"
    record_role: "feature_deep_delta"
    app_behavior: "Drag along a path to delete the covered portion of the path; grouped in the Pencil tool flyout."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.edit-and-reshape-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.rectangle-frame"
    name: "Rectangle Frame tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws rectangular placeholder frames (X-through display) for graphics or text; shortcut F."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.add-frames-paths-as-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.ellipse-frame"
    name: "Ellipse Frame tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws elliptical placeholder frames; grouped in the frame tool flyout with rectangle and polygon frames."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.add-frames-paths-as-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.polygon-frame"
    name: "Polygon Frame tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws polygon and star placeholder frames with configurable side count and star inset."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.add-frames-paths-as-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.rectangle"
    name: "Rectangle tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws rectangle shapes; shortcut M; Shift constrains to a square; double-click opens size options."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.draw-lines-and-shapes.draw-basic-lines-and-shapes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.ellipse"
    name: "Ellipse tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws ellipse shapes; shortcut L; Shift constrains to a circle."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.draw-lines-and-shapes.draw-basic-lines-and-shapes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.polygon"
    name: "Polygon tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws polygons and stars; double-click sets number of sides and star inset percentage."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.draw-lines-and-shapes.draw-basic-lines-and-shapes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.frame-grid-horizontal"
    name: "Horizontal Frame Grid tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws CJK frame grids with horizontal writing direction; shortcut Y."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.grids.set-frame-grid-properties"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.frame-grid-vertical"
    name: "Vertical Frame Grid tool"
    record_role: "feature_deep_delta"
    app_behavior: "Draws CJK frame grids with vertical writing direction; shortcut Q."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.grids.set-frame-grid-properties"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.scissors"
    name: "Scissors tool"
    record_role: "feature_deep_delta"
    app_behavior: "Cuts a path or frame at a clicked point, splitting it into open paths; shortcut C."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.edit-and-reshape-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.free-transform"
    name: "Free Transform tool"
    record_role: "feature_deep_delta"
    app_behavior: "Moves, scales, rotates, and shears the selection with one tool; shortcut E."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.transform-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.rotate"
    name: "Rotate tool"
    record_role: "feature_deep_delta"
    app_behavior: "Rotates the selection around a movable reference point; shortcut R; double-click opens a numeric rotate dialog with copy option."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.transform-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.scale"
    name: "Scale tool"
    record_role: "feature_deep_delta"
    app_behavior: "Scales the selection around a reference point; shortcut S; double-click opens a numeric scale dialog with copy option."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.transform-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.shear"
    name: "Shear tool"
    record_role: "feature_deep_delta"
    app_behavior: "Skews the selection along an axis around a reference point; shortcut O; double-click opens a numeric shear dialog."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.transform-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.gradient-swatch"
    name: "Gradient Swatch tool"
    record_role: "feature_deep_delta"
    app_behavior: "Drags to set the direction and span of a gradient fill across the selection; shortcut G."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.modify-and-adjust-gradients"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.gradient-feather"
    name: "Gradient Feather tool"
    record_role: "feature_deep_delta"
    app_behavior: "Drags to fade an object to transparent along a linear gradient; shortcut Shift+G."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.apply-opacity-and-transparency-effects"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.note"
    name: "Note tool"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts inline editorial notes into text for InCopy-style workflows; notes are viewed in the Notes panel and Story Editor."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.track-changes-and-review.add-editorial-notes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.color-theme"
    name: "Color Theme tool"
    record_role: "feature_deep_delta"
    app_behavior: "Samples a color theme from page artwork or images and adds theme swatches to the Swatches panel or CC Libraries."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.sample-colors-from-placed-graphics"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.tools.eyedropper"
    name: "Eyedropper tool"
    record_role: "feature_deep_delta"
    app_behavior: "Samples fill, stroke, and text attributes from one object and applies them to others; shortcut I; double-click opens Eyedropper Options attribute checklists."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.sample-colors-from-placed-graphics"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.measure"
    name: "Measure tool"
    record_role: "feature_deep_delta"
    app_behavior: "Measures distances and angles between points, reporting into the Info panel; shortcut K."
    primitive_domain: "diagnostics"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.measure-distance-between-points"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.hand"
    name: "Hand tool"
    record_role: "feature_deep_delta"
    app_behavior: "Pans the page view; shortcut H; Spacebar (or Alt+Spacebar inside text) temporarily invokes it."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.zoom-and-view-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.zoom"
    name: "Zoom tool"
    record_role: "feature_deep_delta"
    app_behavior: "Zooms the view in or out; shortcut Z; Ctrl+Spacebar zoom-in and Alt+Ctrl+Spacebar zoom-out are temporary invocations."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.zoom-and-view-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.fill-stroke-proxy"
    name: "Fill/Stroke proxy controls"
    record_role: "feature_deep_delta"
    app_behavior: "Toolbox proxy toggles fill vs stroke target (X), swaps fill and stroke (Shift+X), toggles formatting-affects-container vs text (J), and applies color (comma), gradient (period), or none (slash)."
    primitive_domain: "color"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.view-mode-toggle"
    name: "Toolbox view mode toggle"
    record_role: "feature_deep_delta"
    app_behavior: "Toolbox bottom control switches between Normal and Preview screen modes; shortcut W."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.change-screen-modes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.toolbox-layout"
    name: "Toolbox layout switching"
    record_role: "feature_deep_delta"
    app_behavior: "The toolbox itself can display as single column, double column, or single row and can float or dock."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.change-the-toolbox-layout"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/change-the-toolbox-layout.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
  - id: "indesign.deep.tools.tool-hints"
    name: "Tool Hints panel"
    record_role: "feature_deep_delta"
    app_behavior: "Window > Utilities > Tool Hints describes the selected tool and its modifier-key behaviors."
    primitive_domain: "diagnostics"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    source_ids: [DD-S02]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-INDESIGN-DEEP-DELTA.menu-commands] Menu Command Tree

```yaml
records:
  - id: "indesign.deep.menu-commands.file-new-document"
    name: "File > New > Document"
    record_role: "feature_deep_delta"
    app_behavior: "Creates a document from the New Document dialog with intent presets, page size, pages, facing pages, columns, margins, bleed, and slug."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-documents.create-new-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-documents/create-new-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-new-book"
    name: "File > New > Book"
    record_role: "feature_deep_delta"
    app_behavior: "Creates an INDB book file that groups documents for shared numbering, synchronization, and output."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-and-manage-book-files.create-save-book-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-new-library"
    name: "File > New > Library"
    record_role: "feature_deep_delta"
    app_behavior: "Creates an INDL object library file for storing reusable page objects."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.object-libraries-and-snippets.create-object-libraries"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-open"
    name: "File > Open"
    record_role: "feature_deep_delta"
    app_behavior: "Opens INDD, INDT, INDL, INDB, and IDML files, with open-as Normal/Original/Copy modes for documents and templates."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-documents.open-indesign-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-browse-in-bridge"
    name: "File > Browse in Bridge"
    record_role: "feature_deep_delta"
    app_behavior: "Hands off to Adobe Bridge for visual asset browsing, then places assets back into the layout."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.app-integrations.use-adobe-bridge"
    source_url: "https://helpx.adobe.com/indesign/desktop/app-integrations/use-adobe-bridge.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-save-family"
    name: "File > Save / Save As / Save a Copy / Revert"
    record_role: "feature_deep_delta"
    app_behavior: "Saves as INDD or INDT template, saves a parallel copy without switching working files, and reverts to the last saved state."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.save-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-place"
    name: "File > Place"
    record_role: "feature_deep_delta"
    app_behavior: "Imports graphics, text, spreadsheet, and media formats (Ctrl/Cmd+D) with Show Import Options, Replace Selected Item, and a multi-file loaded cursor."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.add-images"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-place-from-cc-libraries"
    name: "Place from CC Libraries"
    record_role: "feature_deep_delta"
    app_behavior: "Places linked or copied assets directly from Creative Cloud Libraries into the layout."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.import-and-convert-file-to-indesign.import-from-cc-libraries"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/import-and-convert-file-to-indesign/import-from-cc-libraries.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-import-xml"
    name: "File > Import XML"
    record_role: "feature_deep_delta"
    app_behavior: "Imports XML content into the document structure with merge or append modes and link/clone import options."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.import-xml-data-into-indesign"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/import-xml-data-into-indesign.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-import-pdf-comments"
    name: "File > Import PDF Comments"
    record_role: "feature_deep_delta"
    app_behavior: "Imports comments from a PDF exported from the same document and lists them in a panel for accept/resolve workflows."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.import-pdf-comments"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/import-pdf-comments.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-adobe-pdf-presets"
    name: "File > Adobe PDF Presets"
    record_role: "feature_deep_delta"
    app_behavior: "Applies or defines named PDF export presets including High Quality Print, Press Quality, Smallest File Size, and PDF/X variants, and exports directly through a chosen preset."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.manage-pdf-presets"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-export"
    name: "File > Export"
    record_role: "feature_deep_delta"
    app_behavior: "Exports to PDF (Print), PDF (Interactive), EPS, EPUB reflowable/fixed, FLA/SWF legacy, IDML, InDesign Tagged Text, RTF, TXT, XML, ICML, JPEG, PNG, and HTML."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.adobe-pdf-export-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-publish-online"
    name: "File > Publish Online"
    record_role: "feature_deep_delta"
    app_behavior: "Uploads the document to Adobe-hosted Publish Online with title/description, page range, spread handling, and analytics options; provider-dependent."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.publish-work-online.publish-indesign-documents-online"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/publish-work-online/publish-indesign-documents-online.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-document-presets"
    name: "File > Document Presets"
    record_role: "feature_deep_delta"
    app_behavior: "Defines, saves, loads, and applies reusable new-document setting presets."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-documents.create-documents-with-presets"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-documents/create-documents-with-presets.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-document-setup"
    name: "File > Document Setup"
    record_role: "feature_deep_delta"
    app_behavior: "Edits page size, page count, facing pages, start page number, bleed, and slug for an existing document, optionally with Adjust Layout."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-documents.change-document-setup"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-documents/change-document-setup.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-file-info"
    name: "File > File Info"
    record_role: "feature_deep_delta"
    app_behavior: "Edits XMP metadata (title, author, description, copyright, custom fields) stored with the document."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.add-edit-file-metadata"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/save-and-export/add-edit-file-metadata.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-package"
    name: "File > Package"
    record_role: "feature_deep_delta"
    app_behavior: "Collects the document, fonts, and links into a handoff folder with summary inventory, instructions file, and optional IDML and PDF copies."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.preflight.package-files-for-output"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-print"
    name: "File > Print"
    record_role: "feature_deep_delta"
    app_behavior: "Opens the eight-panel Print dialog (General, Setup, Marks and Bleed, Output, Graphics, Color Management, Advanced, Summary) with print presets."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.print-documents-and-books"
    source_url: "https://helpx.adobe.com/indesign/using/printers-marks-bleeds.html"
    source_ids: [DD-S22]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.file-print-booklet"
    name: "File > Print Booklet"
    record_role: "feature_deep_delta"
    app_behavior: "Imposes pages into printer spreads for booklet output with 2-up saddle stitch, 2-up perfect bound, and consecutive booklet types plus creep settings."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-booklets.impose-documents-for-booklet-printing"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/print-booklets/booklet-types.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-undo-redo"
    name: "Edit > Undo / Redo"
    record_role: "feature_deep_delta"
    app_behavior: "Multi-step undo/redo of edits, with a History panel for jumping across document states."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.undo-redo-edits"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/undo-redo-edits.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-paste-variants"
    name: "Edit > Paste variants"
    record_role: "feature_deep_delta"
    app_behavior: "Paste, Paste without Formatting, Paste Into (nests content into a selected frame), and Paste in Place (same coordinates) are distinct paste commands."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.edit-duplicate-step-repeat"
    name: "Edit > Duplicate and Step and Repeat"
    record_role: "feature_deep_delta"
    app_behavior: "Duplicates the selection with stored offsets, and Step and Repeat creates count-by-offset arrays including grid arrays."
    primitive_domain: "layout"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/transform-multiple-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.edit-place-and-link"
    name: "Edit > Place and Link"
    record_role: "feature_deep_delta"
    app_behavior: "Creates linked child copies of selected stories or objects whose parents push updates through the Links panel."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.link-and-update-content-across-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/link-and-update-content-across-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-quick-apply"
    name: "Quick Apply"
    record_role: "feature_deep_delta"
    app_behavior: "Type-ahead popup (Ctrl/Cmd+Enter) applies styles, menu commands, scripts, and text variables by name without mouse navigation."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/using-quick-apply.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.edit-find-change"
    name: "Edit > Find/Change"
    record_role: "feature_deep_delta"
    app_behavior: "Single dialog hosts Text, GREP, Glyph, Object, and Color search modes with scope control and saved queries."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-import-text.find-replace-text"
    source_url: "https://helpx.adobe.com/indesign/using/find-change.ug.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-spelling"
    name: "Edit > Spelling submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Check Spelling dialog, Dynamic Spelling underlining, and Autocorrect are separate commands under Edit > Spelling."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.spell-check.check-spelling"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/spell-check/check-spelling.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-transparency-blend-space"
    name: "Edit > Transparency Blend Space"
    record_role: "feature_deep_delta"
    app_behavior: "Sets the per-document blending color space (Document CMYK or Document RGB) used to composite transparency."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.apply-blending-modes"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-flattener-presets"
    name: "Edit > Transparency Flattener Presets"
    record_role: "feature_deep_delta"
    app_behavior: "Creates and manages named flattener presets (raster/vector balance, resolutions, text/stroke outline options) used by print and legacy export."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.create-custom-transparency-flattener-presets"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/advanced-color-techniques/transparency-flattener-preset-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-color-settings"
    name: "Edit > Color Settings / Assign Profiles / Convert to Profile"
    record_role: "feature_deep_delta"
    app_behavior: "Application color settings choose working RGB/CMYK profiles and policies; Assign Profiles and Convert to Profile change document profile handling."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.use-color-management-when-printing"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/use-color-management-when-printing.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.edit-keyboard-shortcuts"
    name: "Edit > Keyboard Shortcuts"
    record_role: "feature_deep_delta"
    app_behavior: "Manages shortcut sets (default, Illustrator-like, Photoshop-like, custom), edits per-command bindings by product area and context, and prints a set listing via Show Set."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.keyboard-shortcuts"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.edit-menus"
    name: "Edit > Menus"
    record_role: "feature_deep_delta"
    app_behavior: "Customizes menu visibility and colorization per menu set, savable as named menu sets."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.customize-menus"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/customize-menus.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-pages-submenu"
    name: "Layout > Pages submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Add Page, Insert Pages, Move Pages, Duplicate Spread, Delete Pages, Apply Parent to Pages, and page navigation live under Layout > Pages."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.arrange-and-order-pages.add-new-pages"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/arrange-and-order-pages/add-new-pages.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-margins-and-columns"
    name: "Layout > Margins and Columns"
    record_role: "feature_deep_delta"
    app_behavior: "Sets margins, column count, gutter, and column direction for selected pages or parents, with optional layout adjustment."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-documents.change-document-setup"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-documents/change-document-setup.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-ruler-guides"
    name: "Layout > Ruler Guides"
    record_role: "feature_deep_delta"
    app_behavior: "Sets guide color and view threshold for selected ruler guides."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.customize-ruler-guides"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/rulers-and-measure-tools/customize-ruler-guides.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-create-guides"
    name: "Layout > Create Guides"
    record_role: "feature_deep_delta"
    app_behavior: "Generates evenly spaced row/column guide grids with gutter values, fit to margins or page, and optional removal of existing guides."
    primitive_domain: "layout"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/rulers-and-measure-tools/create-ruler-guides.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.layout-liquid-layout"
    name: "Layout > Liquid Layout"
    record_role: "feature_deep_delta"
    app_behavior: "Opens the Liquid Layout panel to assign per-page liquid page rules used when pages are resized with the Page tool or Adjust Layout."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.liquid-page-rules-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-create-alternate-layout"
    name: "Layout > Create Alternate Layout"
    record_role: "feature_deep_delta"
    app_behavior: "Duplicates source pages into a named alternate layout with target page size, liquid page rule, linked stories, and copied text style group options."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.create-alternate-layouts"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/alternate-layout-options.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-page-navigation"
    name: "Layout page navigation commands"
    record_role: "feature_deep_delta"
    app_behavior: "First Page, Previous/Next Page, Last Page, Previous/Next Spread, and Go Back/Go Forward navigate the view with dedicated shortcuts."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-numbering-section-options"
    name: "Layout > Numbering & Section Options"
    record_role: "feature_deep_delta"
    app_behavior: "Starts sections, sets page number style and start value, section prefix, section marker text, and chapter numbering."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.page-numbers-chapters-and-sections.document-numbering-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/page-numbers-chapters-and-sections/document-numbering-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.layout-toc"
    name: "Layout > Table of Contents / Update / TOC Styles"
    record_role: "feature_deep_delta"
    app_behavior: "Generates a style-driven TOC story, refreshes it via Update Table of Contents, and stores reusable TOC style definitions."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.add-a-table-of-contents.generate-maintain-tocs"
    source_url: "https://helpx.adobe.com/indesign/using/creating-table-contents.html"
    source_ids: [DD-S23]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-tabs"
    name: "Type > Tabs"
    record_role: "feature_deep_delta"
    app_behavior: "Floating tab ruler sets left/center/right/decimal tab stops, leader characters, align-on character, and repeat tab."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.tabs-indents-and-spacing.edit-and-manage-tab-settings"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/tabs-indents-and-spacing/edit-and-manage-tab-settings.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-glyphs"
    name: "Type > Glyphs"
    record_role: "feature_deep_delta"
    app_behavior: "Opens the Glyphs panel to browse a font's full glyph repertoire, alternates, and saved glyph sets."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.open-and-view-glyphs"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/open-and-view-glyphs.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-story"
    name: "Type > Story (Optical Margin Alignment)"
    record_role: "feature_deep_delta"
    app_behavior: "Story panel enables optical margin alignment that hangs punctuation and serifs outside the text margin at a set size."
    primitive_domain: "typography"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/aligning-text.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.type-create-outlines"
    name: "Type > Create Outlines"
    record_role: "feature_deep_delta"
    app_behavior: "Converts live text to editable vector paths, optionally as inline compound paths replacing the text."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.combine-and-convert-paths.create-paths-from-text-outlines"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/combine-and-convert-paths/create-paths-from-text-outlines.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-find-replace-fonts"
    name: "Type > Find/Replace Fonts"
    record_role: "feature_deep_delta"
    app_behavior: "Lists every font used in the document (including inside placed graphics), flags missing fonts, and replaces fonts document-wide with redefine-style option."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.fonts.find-and-replace-fonts"
    source_url: "https://helpx.adobe.com/indesign/desktop/fonts/find-and-replace-fonts.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-change-case"
    name: "Type > Change Case"
    record_role: "feature_deep_delta"
    app_behavior: "Converts selected text to UPPERCASE, lowercase, Title Case, or Sentence case."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.character-formatting.change-text-case"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/character-formatting/change-text-case.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-type-on-path-options"
    name: "Type > Type on a Path > Options"
    record_role: "feature_deep_delta"
    app_behavior: "Sets path-text effect (Rainbow, Skew, 3D Ribbon, Stair Step, Gravity), alignment to path, spacing, flip, and start/end brackets."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.type-on-a-path.apply-effects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/type-on-a-path/apply-effects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-notes"
    name: "Type > Notes submenu"
    record_role: "feature_deep_delta"
    app_behavior: "New Note, Open Note, Delete Note, Convert to Note, Convert to Text, and note navigation commands manage inline editorial notes."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.track-changes-and-review.add-editorial-notes"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/track-changes-and-review/add-editorial-notes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-track-changes"
    name: "Type > Track Changes submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Enables tracking per story or all stories and accepts/rejects changes individually, by story, or document-wide."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.track-changes-and-review.track-text-changes"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/track-changes-and-review/track-text-changes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-footnote-commands"
    name: "Type > Insert Footnote / Document Footnote Options"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts auto-numbered footnotes and opens document-wide footnote numbering-and-formatting plus layout option tabs."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.footnotes-and-endnotes.create-and-manage-footnotes"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/footnotes-and-endnotes/create-and-manage-footnotes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-endnote-commands"
    name: "Type > Insert Endnote / Document Endnote Options / Convert"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts endnotes collected in an endnote frame, configures scope (story/document), numbering, and converts footnotes to endnotes and back."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.footnotes-and-endnotes.create-endnotes"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/footnotes-and-endnotes/convert-footnotes-endnotes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-text-variables-menu"
    name: "Type > Text Variables"
    record_role: "feature_deep_delta"
    app_behavior: "Define, Insert Variable, and Convert Variable to Text manage the document's variable definitions and instances."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.create-manage-text-variables"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-bulleted-numbered-lists-menu"
    name: "Type > Bulleted & Numbered Lists"
    record_role: "feature_deep_delta"
    app_behavior: "Apply/remove bullets or numbers, restart/continue numbering, convert list formatting to literal text, and define named lists."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.lists-and-numbering.define-and-manage-list-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/lists-and-numbering/define-and-manage-list-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.type-insert-special-character"
    name: "Type > Insert Special Character"
    record_role: "feature_deep_delta"
    app_behavior: "Submenus (Symbols; Markers; Hyphens and Dashes; Quotation Marks; Other) insert items such as bullet, copyright, ellipsis, em/en dash, discretionary hyphen, nonbreaking hyphen, page number markers, section marker, footnote marker, tab, indent-to-here, and end-nested-style."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.insert-glyphs-and-special-characters"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/insert-glyphs-and-special-characters.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.type-insert-white-space"
    name: "Type > Insert White Space"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts fixed-width space characters: Em, En, Nonbreaking, Nonbreaking (Fixed Width), Hair, Sixth, Thin, Quarter, Third, Punctuation, Figure, and Flush spaces."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.hidden-character-glossary"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/hidden-character-glossary.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.type-insert-break-character"
    name: "Type > Insert Break Character"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts flow-control breaks: Column, Frame, Page, Odd Page, Even Page, Paragraph Return, Forced Line Break, and Discretionary Line Break."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.hidden-character-glossary"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/hidden-character-glossary.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.type-fill-with-placeholder-text"
    name: "Type > Fill with Placeholder Text"
    record_role: "feature_deep_delta"
    app_behavior: "Fills the selected frame with dummy text; a modifier click allows choosing the placeholder language script."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-import-text.add-text-to-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-import-text/add-text-to-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.type-show-hidden-characters"
    name: "Type > Show Hidden Characters"
    record_role: "feature_deep_delta"
    app_behavior: "Toggles on-screen display of nonprinting characters (spaces, tabs, breaks, markers, anchors) in story color coding."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.view-or-show-hidden-characters"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/view-or-show-hidden-characters.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-transform-submenu"
    name: "Object > Transform submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Move, Scale, Rotate, Rotate 90 CW/CCW, Flip Horizontal/Vertical, Shear, and Clear Transformations act numerically on the selection."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.transform-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/transform-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-transform-again"
    name: "Object > Transform Again submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Transform Again, Transform Again Individually, Transform Sequence Again, and Transform Sequence Again Individually replay recorded transforms on new selections."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.transform-multiple-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/transform-multiple-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-arrange"
    name: "Object > Arrange submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Bring to Front, Bring Forward, Send Backward, and Send to Back reorder stacking within a layer."
    primitive_domain: "layer_graph"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.stack-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-frames-and-objects/stack-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-select-submenu"
    name: "Object > Select submenu"
    record_role: "feature_deep_delta"
    app_behavior: "First/Next/Previous/Last Object Above or Below, plus Container and Content, select through stacked and nested objects."
    primitive_domain: "selection_mask"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.select-nested-overlapping-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-frames-and-objects/select-nested-overlapping-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-group-lock-hide"
    name: "Object > Group / Ungroup / Lock / Unlock / Hide / Show"
    record_role: "feature_deep_delta"
    app_behavior: "Groups selections, locks objects (with unlock-all-on-spread), and hides objects (with show-all-on-spread)."
    primitive_domain: "layer_graph"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.group-lock-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/group-lock-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-text-frame-options"
    name: "Object > Text Frame Options"
    record_role: "feature_deep_delta"
    app_behavior: "Five-tab dialog: General (columns fixed/flexible, inset, vertical justification, ignore text wrap), Column Rules, Baseline Options (first baseline, custom frame baseline grid), Auto-Size (growth modes and anchor), and Footnotes (span override)."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-manage-text-frames.change-text-frame-properties"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-manage-text-frames/change-text-frame-properties.html"
    source_ids: [DD-S20]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-anchored-object"
    name: "Object > Anchored Object submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Insert creates an anchored frame at the cursor; Options edits position; Release detaches the object from its anchor."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.create-position-anchored-objects"
    source_url: "https://helpx.adobe.com/indesign/using/anchored-objects.html"
    source_ids: [DD-S16]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-fitting"
    name: "Object > Fitting submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Fill Frame Proportionally, Fit Content Proportionally, Content-Aware Fit, Fit Frame to Content, Fit Content to Frame, Center Content, Clear Frame Fitting Options, and persistent Frame Fitting Options with crop amounts and reference point."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.fit-object-to-frame"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-frames-and-objects/fit-object-to-frame.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-content-type"
    name: "Object > Content submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Reassigns a frame's content type among Graphic, Text, and Unassigned."
    primitive_domain: "layout"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-frames-and-objects/add-frames-paths-as-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.object-effects-menu"
    name: "Object > Effects menu"
    record_role: "feature_deep_delta"
    app_behavior: "Opens the Effects dialog to any of the nine effects and to Transparency, applied per target (Object, Fill, Stroke, Text), plus Global Light settings."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.apply-opacity-and-transparency-effects"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-corner-options"
    name: "Object > Corner Options"
    record_role: "feature_deep_delta"
    app_behavior: "Applies per-corner shape (Fancy, Bevel, Inset, Inverse Rounded, Rounded) and size to rectangle corners, also editable on-canvas via live corners."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.apply-corner-effects-to-frames"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/edit-and-style-paths/apply-corner-effects-to-frames.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-object-layer-options"
    name: "Object > Object Layer Options"
    record_role: "feature_deep_delta"
    app_behavior: "Overrides layer and layer-comp visibility inside placed PSD/AI/INDD/PDF files with an update-link visibility policy."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-adobe-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/add-edit-graphics/import-options-for-adobe-files.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.object-object-export-options"
    name: "Object > Object Export Options"
    record_role: "feature_deep_delta"
    app_behavior: "Per-object alt text, tagged PDF role and actual text, and EPUB/HTML conversion overrides (rasterization, size, custom CSS) travel with the object."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.apply-object-export-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-frames-and-objects/apply-object-export-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-captions"
    name: "Object > Captions submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Caption Setup defines metadata-driven caption lines; Generate Live Caption creates variable captions that update with the image; Generate Static Caption and Convert to Static freeze them."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.references-and-bookmarks.create-image-captions"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/references-and-bookmarks/create-image-captions.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-clipping-path"
    name: "Object > Clipping Path submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Options builds clipping from Detect Edges, Alpha Channel, or Photoshop Path with threshold/tolerance/inset controls; Convert Clipping Path to Frame makes it an editable frame."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.combine-and-convert-paths.apply-clipping-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/combine-and-convert-paths/apply-clipping-paths.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-image-color-settings"
    name: "Object > Image Color Settings"
    record_role: "feature_deep_delta"
    app_behavior: "Assigns a color profile and rendering intent to an individual placed image, overriding document defaults."
    primitive_domain: "color"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/use-color-management-when-printing.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.object-interactive-submenu"
    name: "Object > Interactive submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Convert to Button, Convert to Multi-State Object, and related conversion commands turn page items into interactive objects."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.create-and-add-buttons"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08, DD-S18]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-paths-submenu"
    name: "Object > Paths submenu"
    record_role: "feature_deep_delta"
    app_behavior: "Join, Open Path, Close Path, Reverse Path, and Make/Release Compound Path edit path topology."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.extend-join-and-convert-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/edit-and-style-paths/extend-join-and-convert-paths.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-pathfinder-ops"
    name: "Object > Pathfinder boolean operations"
    record_role: "feature_deep_delta"
    app_behavior: "Add, Subtract, Intersect, Exclude Overlap, and Minus Back combine overlapping shapes into compound results."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.combine-and-convert-paths.create-compound-shapes"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/combine-and-convert-paths/create-compound-shapes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.object-convert-shape-point"
    name: "Object > Convert Shape / Convert Point"
    record_role: "feature_deep_delta"
    app_behavior: "Convert Shape recasts a path as rectangle, rounded/beveled/inverse-rounded rectangle, ellipse, triangle, polygon, line, or orthogonal line; Convert Point switches anchor types (line end, corner, smooth, symmetrical)."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.edit-and-reshape-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/edit-and-style-paths/edit-and-reshape-paths.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.view-screen-modes"
    name: "View > Screen Mode"
    record_role: "feature_deep_delta"
    app_behavior: "Normal, Preview, Bleed, Slug, and Presentation screen modes change chrome and pasteboard rendering; W toggles Normal/Preview."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.change-screen-modes"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/change-screen-modes.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.view-display-performance"
    name: "View > Display Performance"
    record_role: "feature_deep_delta"
    app_behavior: "Fast, Typical, and High Quality display modes plus per-object overrides trade preview fidelity for speed; Ctrl+Alt+Shift+Z toggles high quality/fast."
    primitive_domain: "diagnostics"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.adjust-text-display-quality"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.view-overprint-preview"
    name: "View > Overprint Preview"
    record_role: "feature_deep_delta"
    app_behavior: "Simulates on screen how overprinting and blending inks will render on separations output."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.about-overprinting"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/about-overprinting.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.view-proof-setup-colors"
    name: "View > Proof Setup / Proof Colors"
    record_role: "feature_deep_delta"
    app_behavior: "Soft-proofs the document against a chosen output profile with simulate-paper/ink options."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.use-color-management-when-printing"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/use-color-management-when-printing.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.view-grids-guides-extras"
    name: "View > Grids & Guides and Extras"
    record_role: "feature_deep_delta"
    app_behavior: "Show/Hide and Lock Guides, Lock Column Guides, Snap to Guides, Smart Guides, Baseline Grid, Document Grid, Snap to Document Grid, plus Extras toggles for frame edges, text threads, hyperlinks, and notes."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.show-hide-guides"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/grids/show-or-hide-grids.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.view-story-editor"
    name: "Edit > Edit in Story Editor"
    record_role: "feature_deep_delta"
    app_behavior: "Opens the selected story in the text-only Story Editor window (Ctrl/Cmd+Y) with overset indicator and depth ruler."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-manage-text-frames.open-and-use-story-editor"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-manage-text-frames/open-and-use-story-editor.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.view-structure"
    name: "View > Structure pane toggle"
    record_role: "feature_deep_delta"
    app_behavior: "Shows or hides the XML Structure pane docked at the left of the document window."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.structure-and-tag-documents-for-xml"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/structure-and-tag-documents-for-xml.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.menu-commands.window-arrange"
    name: "Window > Arrange"
    record_role: "feature_deep_delta"
    app_behavior: "Tiles, cascades, floats, and consolidates document windows and creates a New Window view of the same document."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/customize-panels.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.menu-commands.help-menu"
    name: "Help menu"
    record_role: "feature_deep_delta"
    app_behavior: "InDesign Help, tutorials, system compatibility report, and account/updates entry points."
    primitive_domain: "diagnostics"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop.html"
    source_ids: [DD-S25]
    verification_status: UNVERIFIED
    residual_reason: "Complete Help-menu entry enumeration is not published as a single official article and helpx was bot-blocked this pass. Capture the exact menu tree from an installed InDesign via the installed-app export playbook (32-adobe-installed-ui-export-playbook.md) before command-contract promotion; the row's surface (Help menu exists) is not in doubt, only the exact leaf enumeration."
```

### [SFR-INDESIGN-DEEP-DELTA.text-and-typography] Text and Typography Engine

```yaml
records:
  - id: "indesign.deep.text-and-typography.paragraph-composer"
    name: "Adobe Paragraph Composer"
    record_role: "feature_deep_delta"
    app_behavior: "Default composer evaluates all lines of a paragraph together, weighting breakpoints by letterspacing, word spacing, and hyphenation penalties."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.set-text-composition"
    source_url: "https://helpx.adobe.com/indesign/using/text-composition.html"
    source_ids: [DD-S17]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.balance-ragged-lines"
    name: "Balance Ragged Lines"
    record_role: "feature_deep_delta"
    app_behavior: "Paragraph/Control panel menu option that redistributes line breaks to even out ragged (non-justified) line lengths for multi-line headings, pull-quotes, and centered paragraphs; requires the Adobe Paragraph Composer and applies to Align Left/Center/Right paragraphs."
    primitive_domain: "typography"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/aligning-text.html"
    source_ids: [DD-S29]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.text-and-typography.single-line-composer"
    name: "Adobe Single-line Composer"
    record_role: "feature_deep_delta"
    app_behavior: "Composes text one line at a time for traditional manual control of line breaks."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.set-text-composition"
    source_url: "https://helpx.adobe.com/indesign/using/text-composition.html"
    source_ids: [DD-S17]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.world-ready-composers"
    name: "Adobe World-Ready Composers"
    record_role: "feature_deep_delta"
    app_behavior: "World-Ready paragraph and single-line composers add complex-script shaping (Arabic, Hebrew, Indic) selectable per paragraph."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.language-settings.adobe-world-ready-composer-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/language-settings/adobe-world-ready-composer-overview.html"
    source_ids: [DD-S17]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.japanese-composers"
    name: "Japanese composers"
    record_role: "feature_deep_delta"
    app_behavior: "Japanese Paragraph and Japanese Single-line composers apply CJK line-breaking, kinsoku, and mojikumi rules."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.chinese-japanese-and-korean.use-kinsoku-settings"
    source_url: "https://helpx.adobe.com/indesign/using/text-composition.html"
    source_ids: [DD-S17]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.justification-controls"
    name: "Justification dialog controls"
    record_role: "feature_deep_delta"
    app_behavior: "Min/Desired/Max ranges for Word Spacing, Letter Spacing, and Glyph Scaling, plus Auto Leading percentage and Single Word Justification policy."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.tabs-indents-and-spacing.adjust-text-spacing"
    source_url: "https://helpx.adobe.com/indesign/using/text-composition.html"
    source_ids: [DD-S17]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.hyphenation-controls"
    name: "Hyphenation Settings dialog"
    record_role: "feature_deep_delta"
    app_behavior: "Words-with-at-least, after-first, before-last letter counts, hyphen limit, hyphenation zone, better-spacing/fewer-hyphens slider, and toggles for capitalized words, last word, and across-column hyphenation."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.control-hyphenation-and-word-breaks"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/composition-and-text-wrapping/control-hyphenation-and-word-breaks.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.text-and-typography.keep-options"
    name: "Keep Options"
    record_role: "feature_deep_delta"
    app_behavior: "Keep With Previous, Keep With Next N lines, Keep Lines Together (all or start/end counts), and Start Paragraph anywhere/next-column/frame/page/odd/even control paragraph flow breaks."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.paragraph-break-options-in-indesign"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/composition-and-text-wrapping/paragraph-break-options-in-indesign.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.span-split-columns"
    name: "Span Columns and Split Columns"
    record_role: "feature_deep_delta"
    app_behavior: "A paragraph can span all or N columns of its frame or split into sub-columns, with before/after spacing and inside/outside gutter controls."
    primitive_domain: "typography"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/paragraph-formatting.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.text-and-typography.drop-caps"
    name: "Drop caps"
    record_role: "feature_deep_delta"
    app_behavior: "Drop cap line count and character count per paragraph, optional character style, align-left-edge, and scale-for-descenders options."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.character-formatting.apply-drop-caps-text-positioning"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/character-formatting/apply-drop-caps-text-positioning.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.nested-styles"
    name: "Nested styles and nested line styles"
    record_role: "feature_deep_delta"
    app_behavior: "Applies character styles through/up-to N occurrences of delimiters (characters, words, tabs, end-nested-style marker) inside a paragraph, plus per-line nested line styles."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.created-nested-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/created-nested-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.grep-styles"
    name: "GREP styles"
    record_role: "feature_deep_delta"
    app_behavior: "Applies a character style automatically to every regex match inside paragraphs carrying the paragraph style."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-grep-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/create-grep-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.paragraph-shading"
    name: "Paragraph shading"
    record_role: "feature_deep_delta"
    app_behavior: "Fills the paragraph area with a color including offsets, corner radii, clip-to-frame, and do-not-print-or-export toggle."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.apply-paragraph-borders-and-backgrounds"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/composition-and-text-wrapping/apply-paragraph-borders-and-backgrounds.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.paragraph-border"
    name: "Paragraph border"
    record_role: "feature_deep_delta"
    app_behavior: "Draws a stroked border around paragraphs with per-side widths, corner shapes, offsets, and merge-consecutive-borders behavior."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.apply-paragraph-borders-and-backgrounds"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/composition-and-text-wrapping/apply-paragraph-borders-and-backgrounds.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.paragraph-rules"
    name: "Paragraph rules above/below"
    record_role: "feature_deep_delta"
    app_behavior: "Rule Above and Rule Below with weight, stroke type, color, tint, overprint, width (column/text), indents, and offset."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.add-or-remove-paragraph-rules"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/composition-and-text-wrapping/add-or-remove-paragraph-rules.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.baseline-grid-alignment"
    name: "Align to baseline grid"
    record_role: "feature_deep_delta"
    app_behavior: "Per-paragraph align-to-grid (all lines or first line only) snaps leading to the document or custom frame baseline grid."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.grids.use-a-baseline-grid"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/grids/use-a-baseline-grid.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.opentype-features"
    name: "OpenType feature set"
    record_role: "feature_deep_delta"
    app_behavior: "Discretionary ligatures, fractions, ordinals, swash, titling and contextual alternates, all small caps, slashed zero, stylistic sets, positional forms, superscript/subscript, and figure styles (tabular/proportional lining/oldstyle) toggle per text run."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.fonts.opentype-font-attributes"
    source_url: "https://helpx.adobe.com/indesign/desktop/fonts/opentype-font-attributes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.character-attributes"
    name: "Character attribute set"
    record_role: "feature_deep_delta"
    app_behavior: "Font family/style, size, leading, kerning (metrics/optical/manual), tracking, horizontal/vertical scale, baseline shift, skew, case styles, underline and strikethrough with custom options, ligatures, no-break, and language."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.tabs-indents-and-spacing.about-kerning-and-tracking"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.glyph-sets"
    name: "Glyphs panel glyph sets"
    record_role: "feature_deep_delta"
    app_behavior: "User-defined glyph sets store glyphs (optionally with font memory), alongside recently-used glyphs and alternate-for-selection filtering."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.insert-glyphs-and-special-characters"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/insert-glyphs-and-special-characters.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.text-and-typography.story-editor"
    name: "Story Editor window"
    record_role: "feature_deep_delta"
    app_behavior: "Text-only editing view with style column, depth ruler, overset text indicator, and inline display of notes, tracked changes, tables, and XML tags."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-manage-text-frames.open-and-use-story-editor"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-manage-text-frames/open-and-use-story-editor.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.autoflow-modes"
    name: "Text placement flow modes"
    record_role: "feature_deep_delta"
    app_behavior: "Loaded text cursor supports manual flow, semi-autoflow (Alt-click), autoflow (Shift-click adding pages), and fixed-page autoflow (Shift+Alt-click)."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-import-text.thread-text-frames"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-import-text/thread-text-frames.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.text-and-typography.smart-text-reflow"
    name: "Smart Text Reflow"
    record_role: "feature_deep_delta"
    app_behavior: "Automatically adds or deletes pages as threaded text grows or shrinks, scoped to primary text frames or all frames."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-import-text.set-up-smart-text-reflow"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-import-text/set-up-smart-text-reflow.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-chapter-number"
    name: "Text variable: Chapter Number"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts the document's chapter number with text before/after and numbering style options."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-creation-date"
    name: "Text variable: Creation Date"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts the date/time the document was first saved, with shared date-format tokens and before/after text."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-modification-date"
    name: "Text variable: Modification Date"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts the date/time the document was last saved to disk, with date-format options."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-output-date"
    name: "Text variable: Output Date"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts the date/time of the current print job, PDF export, or package operation, commonly used in slug areas."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-custom-text"
    name: "Text variable: Custom Text"
    record_role: "feature_deep_delta"
    app_behavior: "Reusable text placeholder whose single edit updates every inserted instance."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-file-name"
    name: "Text variable: File Name"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts the document file name with include-path and include-extension options."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-image-name"
    name: "Text variable: Image Name (Metadata Caption)"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts metadata from a nearby placed image, driving live caption generation."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.text-and-typography.variable-last-page-number"
    name: "Text variable: Last Page Number"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts the section or document last page number for page-x-of-y constructs, with numbering style options."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.text-variables-overview"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.variable-running-header"
    name: "Text variables: Running Header (Paragraph Style / Character Style)"
    record_role: "feature_deep_delta"
    app_behavior: "Pulls the first or last on-page text carrying a chosen style, with delete-end-punctuation and change-case options, for dictionary-style headers."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.page-numbers-chapters-and-sections.create-running-headers-footers"
    source_url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    source_ids: [DD-S09]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.conditional-text"
    name: "Conditional text"
    record_role: "feature_deep_delta"
    app_behavior: "Named conditions with indicator styling hide or show tagged text ranges, and condition sets capture visibility combinations."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.conditional-and-variable-text.create-edit-conditional-text"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/conditional-and-variable-text/create-edit-conditional-text.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.cross-references"
    name: "Cross-references with formats"
    record_role: "feature_deep_delta"
    app_behavior: "Inserts references to paragraphs or text anchors using editable cross-reference formats assembled from building blocks (page number, paragraph text/number, chapter number, file name) with character style."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.references-and-bookmarks.use-cross-reference-formats"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/references-and-bookmarks/use-cross-reference-formats.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.footnote-options"
    name: "Document Footnote Options detail"
    record_role: "feature_deep_delta"
    app_behavior: "Numbering and Formatting tab (style, start, restart per page/spread, prefix/suffix, character/paragraph styles, separator) and Layout tab (spacing, rule above, placement, span across columns)."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.footnotes-and-endnotes.change-footnote-numbering-and-layout-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/footnotes-and-endnotes/change-footnote-numbering-and-layout-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.endnote-options"
    name: "Document Endnote Options detail"
    record_role: "feature_deep_delta"
    app_behavior: "Endnote title, numbering style and restart scope, story vs document scope, endnote frame placement, and paragraph/character styles."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.footnotes-and-endnotes.change-endnote-numbering-and-layout"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/footnotes-and-endnotes/change-endnote-numbering-and-layout.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-text-mode"
    name: "Find/Change: Text mode"
    record_role: "feature_deep_delta"
    app_behavior: "Literal search with metacharacter tokens, format-attribute find/change criteria, and case/whole-word toggles."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-import-text.find-replace-text"
    source_url: "https://helpx.adobe.com/indesign/using/find-change.ug.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-grep-mode"
    name: "Find/Change: GREP mode"
    record_role: "feature_deep_delta"
    app_behavior: "Regex search/replace with capture groups ($1..), lookarounds, location tokens, and formatting application on matches."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.find-and-replace-with-text-patterns-grep"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/construct-a-grep-expression.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-glyph-mode"
    name: "Find/Change: Glyph mode"
    record_role: "feature_deep_delta"
    app_behavior: "Finds and replaces specific glyph IDs or Unicode values per font, covering alternates unreachable by text search."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.find-and-replace-glyphs"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/find-and-replace-glyphs.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-object-mode"
    name: "Find/Change: Object mode"
    record_role: "feature_deep_delta"
    app_behavior: "Searches frames by object formatting attributes (fills, strokes, effects, frame options) and applies replacement attributes or object styles."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-frames-and-objects.find-replace-objects"
    source_url: "https://helpx.adobe.com/indesign/using/find-change.ug.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-color-mode"
    name: "Find/Change: Color mode"
    record_role: "feature_deep_delta"
    app_behavior: "Finds usages of a color and replaces them with another color across objects and text."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.find-and-replace-colors"
    source_url: "https://helpx.adobe.com/indesign/using/find-change.ug.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-queries"
    name: "Find/Change saved queries"
    record_role: "feature_deep_delta"
    app_behavior: "Ships with predefined queries (phone conversion, dashes, trailing whitespace) and saves user queries as sharable files."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.save-and-manage-find-and-replace-queries"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/save-and-manage-find-and-replace-queries.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.find-change-scope"
    name: "Find/Change scope and inclusion toggles"
    record_role: "feature_deep_delta"
    app_behavior: "Search scope spans All Documents/Document/Story/To End of Story/Selection with toggles for locked layers, hidden layers, parent pages, and footnotes."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.search-scope-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/search-scope-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.linked-stories"
    name: "Linked stories"
    record_role: "feature_deep_delta"
    app_behavior: "Child stories placed with Place and Link show update badges in the Links panel and can auto-update or warn on parent edits."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.link-and-update-content-across-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/link-and-update-content-across-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.text-import-options"
    name: "Text import option sets"
    record_role: "feature_deep_delta"
    app_behavior: "Word/RTF import maps or preserves styles, footnotes, endnotes, and tables with saved import presets; TXT import controls encoding, dictionary, and carriage-return cleanup; Excel import selects sheet/range and formatting mode."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-import-text.import-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-import-text/import-options.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.column-rules"
    name: "Text frame column rules"
    record_role: "feature_deep_delta"
    app_behavior: "Draws vertical rules between text frame columns with stroke, inset, offset, and balance controls."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-manage-text-frames.set-column-rules"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-manage-text-frames/set-column-rules.html"
    source_ids: [DD-S20]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.math-expressions"
    name: "Math expressions (MathML)"
    record_role: "feature_deep_delta"
    app_behavior: "Creates, edits, and stylizes inline math expressions in documents."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.create-and-insert-math-expressions"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/create-and-insert-math-expressions.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.dictionaries"
    name: "Hyphenation/spelling dictionary stack"
    record_role: "feature_deep_delta"
    app_behavior: "Hunspell default with optional Duden for German, per-language user dictionaries, added/removed word lists with import/export, and dictionary-merge preferences."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.manage-language-dictionaries.manage-user-dictionaries"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/manage-language-dictionaries/manage-user-dictionaries.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.text-and-typography.text-wrap-options"
    name: "Text wrap full options"
    record_role: "feature_deep_delta"
    app_behavior: "Wrap shapes (none, bounding box, object shape, jump object, jump to next column) with per-side offsets, invert, contour source (edges/alpha/path/frame), wrap-to sides, and include-inside-edges."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.composition-and-text-wrapping.apply-text-wrap"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/composition-and-text-wrapping/apply-text-wrap.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-INDESIGN-DEEP-DELTA.styles] Style System

```yaml
records:
  - id: "indesign.deep.styles.paragraph-styles"
    name: "Paragraph styles"
    record_role: "feature_deep_delta"
    app_behavior: "Named paragraph formatting bundles covering every text attribute group, with shortcut assignment and apply-on-create defaults."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-and-edit-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/create-and-edit-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.character-styles"
    name: "Character styles"
    record_role: "feature_deep_delta"
    app_behavior: "Partial-attribute character styles apply only attributes explicitly set, layering over paragraph styles."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-and-edit-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/create-and-edit-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.object-styles"
    name: "Object styles"
    record_role: "feature_deep_delta"
    app_behavior: "Named frame/object formatting with per-category include toggles (stroke, fill, effects, text frame options, wrap, anchoring, export options, size/position) and default text/graphic frame style slots."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-object-styles.define-and-apply-object-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-object-styles/define-and-apply-object-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.table-styles"
    name: "Table styles"
    record_role: "feature_deep_delta"
    app_behavior: "Table-level formatting referencing up to five cell styles (header, footer, body, left/right column) plus border and alternating patterns."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.table-and-cell-styles.table-and-cell-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/table-and-cell-styles/table-and-cell-styles.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.cell-styles"
    name: "Cell styles"
    record_role: "feature_deep_delta"
    app_behavior: "Cell-level formatting (insets, strokes, fills, diagonal lines, optional paragraph style) applied directly or through table styles."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.table-and-cell-styles.apply-table-cell-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/table-and-cell-styles/apply-table-cell-styles.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.toc-styles"
    name: "TOC styles"
    record_role: "feature_deep_delta"
    app_behavior: "Stored Table of Contents definitions (included styles, levels, entry formatting, options) reusable across documents and books."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.add-a-table-of-contents.customize-toc-style"
    source_url: "https://helpx.adobe.com/indesign/using/creating-table-contents.html"
    source_ids: [DD-S23]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.style-basing"
    name: "Based-on style inheritance"
    record_role: "feature_deep_delta"
    app_behavior: "Every style type supports a Based On parent; child styles store only deltas and update when the parent changes."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-and-edit-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/create-and-edit-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.styles.next-style"
    name: "Next Style chaining"
    record_role: "feature_deep_delta"
    app_behavior: "Paragraph styles declare a Next Style used when typing new paragraphs, and Apply Style Then Next Style formats whole selections sequentially."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.apply-sequential-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/apply-sequential-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.overrides"
    name: "Style override handling"
    record_role: "feature_deep_delta"
    app_behavior: "Plus-sign override indicator, override highlighter, Clear Overrides (with modifier scoping for character vs paragraph deltas), and clear-overrides-when-applying toggle."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.override-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/override-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.redefine-style"
    name: "Redefine Style"
    record_role: "feature_deep_delta"
    app_behavior: "Rewrites a style definition from the current selection's formatting, updating every user of the style."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-and-edit-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.style-groups"
    name: "Style groups"
    record_role: "feature_deep_delta"
    app_behavior: "Folder-like groups organize styles in every styles panel and participate in load/import and alternate-layout style-group copies."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-and-edit-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/create-and-edit-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.styles.load-styles"
    name: "Load/import styles across documents"
    record_role: "feature_deep_delta"
    app_behavior: "Loads selected styles from another document with per-style conflict resolution (use incoming vs auto-rename)."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.table-and-cell-styles.import-table-and-cell-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/table-and-cell-styles/import-table-and-cell-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.export-tag-mapping"
    name: "Export tagging (EPUB/HTML/PDF)"
    record_role: "feature_deep_delta"
    app_behavior: "Each text style maps to an export tag and class for EPUB/HTML plus a PDF tag role, editable singly or via Edit All Export Tags."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.map-styles-to-export-tags"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/map-styles-to-export-tags.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.word-style-mapping"
    name: "Word style mapping on import"
    record_role: "feature_deep_delta"
    app_behavior: "Maps incoming Word styles to existing InDesign styles during Place with saved import presets."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.map-word-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/map-word-styles.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.style-packs"
    name: "Style packs"
    record_role: "feature_deep_delta"
    app_behavior: "Preset or user-created bundles of coordinated text styles applied as a set and manageable as packs."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.create-a-style-pack"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/create-a-style-pack.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.auto-style"
    name: "Auto Style"
    record_role: "feature_deep_delta"
    app_behavior: "Automatically detects text roles in a frame and applies mapped styles from a chosen style pack or style set."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.auto-style-text-and-paragraphs"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/auto-style-text-and-paragraphs.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.styles.break-link"
    name: "Break link to style"
    record_role: "feature_deep_delta"
    app_behavior: "Detaches text or objects from their style, freezing current formatting as local values."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.text-styles.break-links-to-text-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/format-and-style-text/text-styles/break-links-to-text-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.pages-and-layout] Pages, Parents, and Layout

```yaml
records:
  - id: "indesign.deep.pages-and-layout.pages-panel-ops"
    name: "Pages panel operations"
    record_role: "feature_deep_delta"
    app_behavior: "Insert/move/duplicate/delete pages and spreads, drag reordering, allow-shuffle toggles per document and spread, island spreads, color labels, and page thumbnail display options."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.arrange-and-order-pages.add-new-pages"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/arrange-and-order-pages/add-new-pages.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.parent-page-overrides"
    name: "Parent page item override/detach"
    record_role: "feature_deep_delta"
    app_behavior: "Ctrl+Shift-click overrides a single parent item on a document page, Override All Parent Page Items overrides everything, Detach severs the link, and Remove Overrides restores; per-item Allow Overrides can be disabled."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-and-manage-parent-pages.manage-parent-items"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-and-manage-parent-pages/manage-parent-items.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.parent-hierarchy"
    name: "Parent-based-on-parent hierarchy"
    record_role: "feature_deep_delta"
    app_behavior: "Parents can be based on other parents, applied to page ranges, loaded from other documents, and layered via parent page overlays."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-and-manage-parent-pages.create-parent-pages"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-and-manage-parent-pages/use-parent-page-overlays.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.primary-text-frame"
    name: "Primary text frame"
    record_role: "feature_deep_delta"
    app_behavior: "A designated parent-page text frame that auto-adopts new page geometry and rethreads on parent change without manual overrides."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-and-manage-parent-pages.about-parent-pages"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-and-manage-parent-pages/about-parent-pages.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.pages-and-layout.liquid-rule-scale"
    name: "Liquid page rule: Scale"
    record_role: "feature_deep_delta"
    app_behavior: "Resizes all page content proportionally, preserving relative positions when the page size changes."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.liquid-page-rules-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.liquid-rule-recenter"
    name: "Liquid page rule: Re-center"
    record_role: "feature_deep_delta"
    app_behavior: "Keeps content at original size and re-centers it on the resized page."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.liquid-page-rules-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.liquid-rule-object-based"
    name: "Liquid page rule: Object-based"
    record_role: "feature_deep_delta"
    app_behavior: "Per-object pins and resize constraints relative to page edges give mixed fixed/relative behavior on resize."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.liquid-page-rules-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.liquid-rule-guide-based"
    name: "Liquid page rule: Guide-based"
    record_role: "feature_deep_delta"
    app_behavior: "Liquid guides slice the page; objects crossed by a guide stretch while text reflows and images resize without distortion."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.liquid-page-rules-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.liquid-rule-controlled-by-parent"
    name: "Liquid page rule: Controlled by Parent (Master)"
    record_role: "feature_deep_delta"
    app_behavior: "The page inherits whatever liquid rule its parent page defines; Off disables liquid behavior entirely."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.liquid-page-rules-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.adjust-layout"
    name: "Adjust Layout"
    record_role: "feature_deep_delta"
    app_behavior: "Recomputes object positions when page size, margins, or bleed change, with options to adjust font size (with limits), locked content, and ruler guides."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.adjust-document-layout"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/adjust-document-layout.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.alternate-layouts"
    name: "Alternate layouts"
    record_role: "feature_deep_delta"
    app_behavior: "Multiple named layouts (page-size/orientation variants) coexist in one document, shown side-by-side in the Pages panel with linked stories back to the source layout."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.create-alternate-layouts"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/create-alternate-layouts.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.flex-layout"
    name: "Flex Layout"
    record_role: "feature_deep_delta"
    app_behavior: "Container-based responsive layout with flex properties (direction, wrap, alignment, spacing) and conflict reporting against fixed positioning."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.flex-layout-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/flex-layout-overview.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.multi-page-spreads"
    name: "Multi-page and island spreads"
    record_role: "feature_deep_delta"
    app_behavior: "Spreads can hold up to ten pages; disabling Allow Document/Selected Spread to Shuffle preserves island spreads during repagination."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-documents.create-multi-page-spreads"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-documents/create-multi-page-spreads.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.pages-and-layout.rotate-spread-view"
    name: "Rotate spread view"
    record_role: "feature_deep_delta"
    app_behavior: "Rotates the on-screen view of a spread 90/180 degrees for editing rotated content without transforming objects."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.arrange-and-order-pages.rotate-spread-view"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/arrange-and-order-pages/rotate-spread-view.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.measurement-units"
    name: "Measurement unit systems"
    record_role: "feature_deep_delta"
    app_behavior: "Points, picas, inches, inches decimal, millimeters, centimeters, ciceros, agates, pixels, and custom units per axis, cycled from the ruler or Units preferences."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.change-ruler-measurement-units"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/rulers-and-measure-tools/change-ruler-measurement-units.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.pages-and-layout.smart-guides"
    name: "Smart guides and smart spacing"
    record_role: "feature_deep_delta"
    app_behavior: "Dynamic alignment feedback against object edges and centers plus smart dimensions and smart spacing hints while dragging."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.use-smart-guides"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/rulers-and-measure-tools/use-smart-guides.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.guide-management"
    name: "Ruler guide management"
    record_role: "feature_deep_delta"
    app_behavior: "Page and spread guides with per-guide color, view threshold, lock, copy/paste across pages, select-all-guides shortcut, and delete-all on spread."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.manage-ruler-guides"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/rulers-and-measure-tools/manage-ruler-guides.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.document-grid"
    name: "Document grid and baseline grid setup"
    record_role: "feature_deep_delta"
    app_behavior: "Grids preferences define baseline grid start/relative-to/increment/view threshold and document grid subdivisions with snap behavior."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.grids.use-a-document-grid"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/grids/use-a-document-grid.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.cjk-layout-grids"
    name: "CJK layout grids and named grids"
    record_role: "feature_deep_delta"
    app_behavior: "Layout grids define character-count-based page geometry; named grids store frame grid formats applied to frames and importable across documents."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.grids.create-apply-named-grids"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/grids/create-customize-layout-grids.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.page-tool-resize"
    name: "Per-page size and orientation"
    record_role: "feature_deep_delta"
    app_behavior: "The Page tool gives individual pages their own size, orientation, and liquid rule inside one document (mixed page sizes)."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.apply-layout-adjustments.adjust-page-or-spread-layout"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/adjust-page-or-spread-layout.html"
    source_ids: [DD-S07]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.align-distribute"
    name: "Align and Distribute engine"
    record_role: "feature_deep_delta"
    app_behavior: "Aligns/distributes to selection, key object, margins, page, or spread, with distribute-spacing gap values."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.align-distribute-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/align-distribute-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.layers-model"
    name: "Document-wide layers model"
    record_role: "feature_deep_delta"
    app_behavior: "Layers span all pages with per-layer color, visibility, lock, print/export suppression, guide visibility, wrap-when-hidden policy, and expandable per-object sublists."
    primitive_domain: "layer_graph"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.manage-layers.create-set-up-layers"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/manage-layers/create-set-up-layers.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.draw-grid-of-frames"
    name: "Gridified frame drawing"
    record_role: "feature_deep_delta"
    app_behavior: "Arrow keys while dragging any frame tool split the drag into an equal grid of frames (also applies to placing multiple files)."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.draw-lines-and-shapes.draw-multiple-objects-as-a-grid"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/draw-lines-and-shapes/draw-multiple-objects-as-a-grid.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.hide-spreads"
    name: "Hide/unhide spreads"
    record_role: "feature_deep_delta"
    app_behavior: "Spreads can be hidden from view and output while remaining in the document."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.arrange-and-order-pages.hide-spreads"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/arrange-and-order-pages/hide-spreads.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.history-panel"
    name: "History panel document states"
    record_role: "feature_deep_delta"
    app_behavior: "Lists edit states of the session and jumps the document to any recorded state beyond linear undo."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.manage-document-states"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/manage-document-states.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.pages-and-layout.document-recovery"
    name: "Automatic document recovery"
    record_role: "feature_deep_delta"
    app_behavior: "Auto-recovery data restores unsaved changes after a crash on next launch, with a configurable recovery folder."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.recover-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/recover-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.tables] Tables

```yaml
records:
  - id: "indesign.deep.tables.create-table"
    name: "Insert Table dialog"
    record_role: "feature_deep_delta"
    app_behavior: "Creates a table with body/header/footer row counts, column count, and optional table style; tables live inside text frames."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.create-tables.create-and-import-tables"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/create-tables/create-and-import-tables.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.convert-text-table"
    name: "Convert Text to Table / Table to Text"
    record_role: "feature_deep_delta"
    app_behavior: "Converts delimited text to a table and back with selectable column/row separators."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.convert-tables-to-text"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/format-tables/convert-tables-to-text.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.excel-import"
    name: "Excel/Word table import options"
    record_role: "feature_deep_delta"
    app_behavior: "Excel place options select sheet, view, and cell range and import as formatted/unformatted table or tabbed text; Word tables can retain or strip formatting and can remain linked."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.create-tables.create-and-import-tables"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03, DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.table-options-setup"
    name: "Table Options: Table Setup"
    record_role: "feature_deep_delta"
    app_behavior: "Table border stroke, table spacing before/after, stroke drawing order, and header/footer counts."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.strokes-and-fills.change-table-border"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.alternating-strokes-fills"
    name: "Table Options: alternating row/column strokes and fills"
    record_role: "feature_deep_delta"
    app_behavior: "Alternating patterns (every other, every second/third, custom counts) for row strokes, column strokes, and fills with skip-first/skip-last controls."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.strokes-and-fills.add-alternating-strokes-fills"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.headers-footers"
    name: "Table headers and footers"
    record_role: "feature_deep_delta"
    app_behavior: "Header/footer rows repeat per column, frame, or page with skip-first/last options, converted via Convert Rows commands."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.break-tables-across-frames"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.cell-options-text"
    name: "Cell Options: Text"
    record_role: "feature_deep_delta"
    app_behavior: "Cell insets, vertical justification, first baseline, clip-to-cell, and text rotation (0/90/180/270 degrees)."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.format-text-in-tables"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.cell-options-graphic"
    name: "Cell Options: Graphic (graphic cells)"
    record_role: "feature_deep_delta"
    app_behavior: "Cells can be converted to graphic cells that hold placed images with cell-level inset and fitting."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.create-tables.add-content-to-a-table"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.cell-strokes-fills"
    name: "Cell Options: Strokes and Fills"
    record_role: "feature_deep_delta"
    app_behavior: "Per-side cell stroke proxy editing with weight, type, color, tint, gap color, overprint, and cell fill."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.strokes-and-fills.add-strokes-fills-to-cells"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.cell-rows-columns"
    name: "Cell Options: Rows and Columns"
    record_role: "feature_deep_delta"
    app_behavior: "Row height (at-least/exactly), column width, and keep options (keep with next row, start row on next frame/page)."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.resize-columns-rows-and-tables"
    source_url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.cell-diagonal-lines"
    name: "Cell Options: Diagonal Lines"
    record_role: "feature_deep_delta"
    app_behavior: "Adds diagonal or crossed lines per cell with stroke settings and draw-in-front/behind content order."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.strokes-and-fills.add-diagonal-lines-to-a-cell"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/strokes-and-fills/add-diagonal-lines-to-a-cell.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.row-column-ops"
    name: "Row/column structural operations"
    record_role: "feature_deep_delta"
    app_behavior: "Insert/delete/select rows and columns, merge and unmerge cells, split cell horizontally/vertically, distribute rows/columns evenly, paste before/after, and drag-duplicate with modifier."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.select-tables-rows-and-columns"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/format-tables/resize-columns-rows-and-tables.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.table-flow"
    name: "Table flow across frames"
    record_role: "feature_deep_delta"
    app_behavior: "Tables break across threaded frames and pages with repeating headers/footers; Go to Row jumps to a row including header/footer sections."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.break-tables-across-frames"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/format-tables/break-tables-across-frames.html"
    source_ids: [DD-S15]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.nested-tables"
    name: "Nested tables"
    record_role: "feature_deep_delta"
    app_behavior: "A table can be embedded within a cell of another table."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.create-tables.embed-a-table-within-a-table"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/create-tables/embed-a-table-within-a-table.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.tables.table-alignment"
    name: "Table alignment within frame"
    record_role: "feature_deep_delta"
    app_behavior: "Tables align left/center/right within the text frame column and inherit paragraph-level spacing controls."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-tables-and-data.format-tables.change-table-alignment"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-tables-and-data/format-tables/change-table-alignment.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.graphics-and-frames] Graphics, Frames, and Effects

```yaml
records:
  - id: "indesign.deep.graphics-and-frames.psd-import-options"
    name: "PSD/PSB import options"
    record_role: "feature_deep_delta"
    app_behavior: "Placed Photoshop files preserve layers, layer comps, transparency, and channels, with per-place layer visibility selection and color mode support for RGB/CMYK/Lab/Grayscale."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-adobe-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.ai-pdf-import-options"
    name: "AI/PDF place options"
    record_role: "feature_deep_delta"
    app_behavior: "Placing AI or PDF selects pages, crop-to (bounding box, art, crop, trim, bleed, media), and transparent background; multi-page PDFs load sequential pages onto the cursor."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-adobe-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/add-edit-graphics/import-options-for-adobe-files.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.graphics-and-frames.indd-placement"
    name: "INDD-in-INDD placement"
    record_role: "feature_deep_delta"
    app_behavior: "InDesign documents place as graphics with page selection and layer visibility overrides, tracked as links."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-adobe-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/add-edit-graphics/import-options-for-adobe-files.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.graphics-and-frames.image-import-options"
    name: "Raster image import options"
    record_role: "feature_deep_delta"
    app_behavior: "TIFF/JPEG/PNG import options control applying embedded clipping paths, alpha channel choice, and color profile/rendering intent per image."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.import-options-for-image-formats"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/add-edit-graphics/import-options-for-image-formats.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.svg-import"
    name: "SVG/SVGZ import"
    record_role: "feature_deep_delta"
    app_behavior: "SVG and compressed SVGZ place as scalable vector graphics."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.system-and-product-info.supported-file-formats"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.links-panel-ops"
    name: "Links panel operations"
    record_role: "feature_deep_delta"
    app_behavior: "Relink, Relink to Folder, relink across file extensions, Update Link(s), Edit Original, Edit With, Go to Link, Embed/Unembed, Reveal in Explorer/Finder/Bridge, and Copy Link(s) To."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-links.update-restore-replace-links"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-links/update-restore-replace-links.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.link-info"
    name: "Link Info metadata pane"
    record_role: "feature_deep_delta"
    app_behavior: "Per-link status, format, color space, resolution (actual and effective PPI), scale, layer, and path metadata with configurable columns."
    primitive_domain: "diagnostics"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-links.links-panel-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-links/links-panel-overview.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-drop-shadow"
    name: "Effect: Drop Shadow"
    record_role: "feature_deep_delta"
    app_behavior: "Shadow behind object/stroke/fill/text with mode, color, opacity, distance, angle, size, spread, noise, and knockout controls."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-inner-shadow"
    name: "Effect: Inner Shadow"
    record_role: "feature_deep_delta"
    app_behavior: "Shadow inside object edges giving a recessed look, with distance, angle, size, choke, and noise settings."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-outer-glow"
    name: "Effect: Outer Glow"
    record_role: "feature_deep_delta"
    app_behavior: "Glow emanating from outside edges with technique, size, spread, and noise settings."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-inner-glow"
    name: "Effect: Inner Glow"
    record_role: "feature_deep_delta"
    app_behavior: "Glow from inside edges with source center/edge choice, choke, and noise."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-bevel-emboss"
    name: "Effect: Bevel and Emboss"
    record_role: "feature_deep_delta"
    app_behavior: "3D highlight/shadow relief with style, technique, depth, direction, size, soften, and shading angle/altitude."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-satin"
    name: "Effect: Satin"
    record_role: "feature_deep_delta"
    app_behavior: "Interior shading producing a satin finish with angle, distance, size, and invert."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-basic-feather"
    name: "Effect: Basic Feather"
    record_role: "feature_deep_delta"
    app_behavior: "Fades all edges to transparent over a width with corner style (sharp/rounded/diffused) and noise."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-directional-feather"
    name: "Effect: Directional Feather"
    record_role: "feature_deep_delta"
    app_behavior: "Per-side feather widths with angle, shape, and noise for asymmetric edge fades."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-gradient-feather"
    name: "Effect: Gradient Feather"
    record_role: "feature_deep_delta"
    app_behavior: "Linear or radial opacity gradient fading the object to transparent with editable gradient stops."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.transparency-effects-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.blend-modes"
    name: "Blending modes and isolation"
    record_role: "feature_deep_delta"
    app_behavior: "Sixteen blend modes (Normal, Multiply, Screen, Overlay, Soft/Hard Light, Color Dodge/Burn, Darken, Lighten, Difference, Exclusion, Hue, Saturation, Color, Luminosity) plus Isolate Blending and Knockout Group."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.blending-mode-options"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.effect-targeting"
    name: "Effect targeting levels"
    record_role: "feature_deep_delta"
    app_behavior: "Opacity, blend mode, and each effect apply independently to Object, Fill, Stroke, or Text of one frame, shown as an effects tree in the Effects panel."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.apply-opacity-and-transparency-effects"
    source_url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    source_ids: [DD-S06]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.anchored-inline"
    name: "Anchored object: Inline and Above Line"
    record_role: "feature_deep_delta"
    app_behavior: "Inline anchors sit on the text baseline with Y offset; Above Line adds alignment (Left, Center, Right, Towards/Away From Spine, Text Alignment) and space before/after."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.create-position-anchored-objects"
    source_url: "https://helpx.adobe.com/indesign/using/anchored-objects.html"
    source_ids: [DD-S16]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.anchored-custom"
    name: "Anchored object: Custom position with Relative to Spine"
    record_role: "feature_deep_delta"
    app_behavior: "Custom anchoring positions objects by reference points relative to anchor marker, column, frame, page margin, or page edge, with Relative to Spine mirroring across facing pages and keep-within-top/bottom boundaries."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.manage-update-anchored-objects"
    source_url: "https://helpx.adobe.com/indesign/using/anchored-objects.html"
    source_ids: [DD-S16]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.object-states-panel"
    name: "Object States panel and MSOs"
    record_role: "feature_deep_delta"
    app_behavior: "Converts a selection to a multi-state object, adds/reorders/deletes states, adds objects to the visible state, pastes into a state, resets all MSOs, and supports hidden-until-triggered."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.create-object-versions"
    source_url: "https://www.adobe.com/support/indesign/gettingstarted/pdfs/indesign_howto_f_mso.pdf"
    source_ids: [DD-S18]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.qr-codes"
    name: "QR code generator"
    record_role: "feature_deep_delta"
    app_behavior: "Generates editable QR codes of type Web Hyperlink, Plain Text, Text Message, Email, or Business Card with color choice, editable afterward."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.qr-codes.generate-qr-codes"
    source_url: "https://helpx.adobe.com/indesign/desktop/interactive-elements-and-forms/qr-codes/generate-qr-codes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.snippets"
    name: "Snippets (IDMS)"
    record_role: "feature_deep_delta"
    app_behavior: "Drag-out or export selections as IDMS snippet files that re-place at original or cursor position."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.object-libraries-and-snippets.create-add-snippets"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.object-libraries"
    name: "Object libraries (INDL)"
    record_role: "feature_deep_delta"
    app_behavior: "Library panels store, search, sort, and place reusable objects with per-item type and description metadata."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.object-libraries-and-snippets.manage-library-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/object-libraries-and-snippets/manage-library-objects.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.content-conveyor"
    name: "Content Conveyor"
    record_role: "feature_deep_delta"
    app_behavior: "Holds collected items and item sets for placement with place/gun modes (place once, place all, keep in conveyor) and create-link toggle."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.link-and-update-content-across-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/link-and-update-content-across-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.graphics-and-frames.edit-original-with"
    name: "Edit Original / Edit With"
    record_role: "feature_deep_delta"
    app_behavior: "Round-trips a placed asset to its source application (or a chosen app) and auto-updates the link on save."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.manage-links.edit-original-artwork"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/manage-links/edit-original-artwork.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.stroke-panel-options"
    name: "Stroke panel full options"
    record_role: "feature_deep_delta"
    app_behavior: "Weight, cap, join, miter limit, align stroke (center/inside/outside), stroke type list, start/end arrowheads with scale, and gap color/tint for patterned strokes."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.line-stroke-options-and-settings"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/edit-and-style-paths/line-stroke-options-and-settings.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.custom-stroke-styles"
    name: "Custom stroke styles"
    record_role: "feature_deep_delta"
    app_behavior: "User-defined dash, dotted, and stripe stroke styles with pattern length/corner adjustment, savable and loadable across documents."
    primitive_domain: "vector"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-lines-and-shapes.edit-and-style-paths.apply-and-save-line-stroke-styles"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-lines-and-shapes/edit-and-style-paths/apply-and-save-line-stroke-styles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.generative-expand-image"
    name: "Generative Expand image beyond border"
    record_role: "feature_deep_delta"
    app_behavior: "AI-based extension of a placed image to fill frame space beyond its original border; provider-dependent Firefly feature."
    primitive_domain: "raster"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.add-edit-graphics.extend-an-image-beyond-its-border"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/add-edit-graphics/extend-an-image-beyond-its-border.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.graphics-and-frames.media-import"
    name: "Movie and sound placement"
    record_role: "feature_deep_delta"
    app_behavior: "Places FLV/F4V/MP4/MOV video and MP3/AAC/WAV audio with poster frame, controller skin, play-on-page-load, loop, and navigation points for interactive output."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.movies-and-sound.add-movie-and-sound-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.color-and-output] Color System

```yaml
records:
  - id: "indesign.deep.color-and-output.swatch-types"
    name: "Swatch type system"
    record_role: "feature_deep_delta"
    app_behavior: "Process and spot swatches in CMYK/RGB/Lab, tint swatches, gradient swatches, mixed ink swatches, mixed ink groups, and the reserved None/Paper/Black/Registration swatches."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.swatch-types"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/swatch-types.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.mixed-ink"
    name: "Mixed ink swatches and groups"
    record_role: "feature_deep_delta"
    app_behavior: "Combines a spot ink with process inks into one swatch, and mixed ink groups generate stepped swatch series with editable base inks."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.create-mixed-ink-swatches-and-groups"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/create-mixed-ink-swatches-and-groups.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.swatch-panel-ops"
    name: "Swatches panel operations"
    record_role: "feature_deep_delta"
    app_behavior: "Create/duplicate/edit/delete swatches with replace-on-delete, color groups, merge swatches, add unnamed colors, select all unused, and load/save ASE swatch exchange."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.organize-and-reuse-color-swatches"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/import-and-share-swatch-libraries.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.color-libraries"
    name: "Color libraries (PANTONE and others)"
    record_role: "feature_deep_delta"
    app_behavior: "Ships spot/process color book libraries loadable in the New Swatch dialog, plus import of swatches from other documents."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.import-and-share-swatch-libraries"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/import-and-share-swatch-libraries.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.color-and-output.tints"
    name: "Tint swatches and tint slider"
    record_role: "feature_deep_delta"
    app_behavior: "Percentage tints of base swatches as standalone swatches that follow base color edits."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.create-and-edit-tints"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/create-and-edit-tints.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.gradient-editor"
    name: "Gradient editor"
    record_role: "feature_deep_delta"
    app_behavior: "Linear and radial gradients with multi-stop editing, midpoint control, swatch-based stops, reverse, and angle control via Gradient panel and Gradient Swatch tool."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.create-and-name-gradient-swatches"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/advanced-color-techniques/modify-and-adjust-gradients.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.apply-to-grayscale"
    name: "Colorize grayscale images"
    record_role: "feature_deep_delta"
    app_behavior: "Applies swatch color to grayscale/bitmap image content directly in the frame."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.apply-colors-to-grayscale-images"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/apply-colors-to-grayscale-images.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.ink-manager"
    name: "Ink Manager"
    record_role: "feature_deep_delta"
    app_behavior: "Per-ink spot-to-process conversion, All Spots to Process, ink aliasing, Use Standard Lab Values for Spots, and trapping ink types (Normal, Transparent, Opaque, Opaque Ignore) with neutral density and sequence."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.ink-and-color-management.manage-inks-for-separation"
    source_url: "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html"
    source_ids: [DD-S19]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.overprint-attributes"
    name: "Overprint fill/stroke/gap attributes"
    record_role: "feature_deep_delta"
    app_behavior: "Attributes panel sets overprint fill, stroke, and gap per object, plus nonprinting flag; black overprint policy is a preference."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.overprint-strokes-and-fills"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/overprint-strokes-and-fills.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.separations-preview"
    name: "Separations Preview panel"
    record_role: "feature_deep_delta"
    app_behavior: "Per-plate on/off preview of separations, Ink Limit view with configurable total-ink threshold, and per-ink coverage readouts."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.preview-color-separations-and-ink-coverage"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/preview-color-separations-and-ink-coverage.html"
    source_ids: [DD-S19]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.trap-presets"
    name: "Trap presets"
    record_role: "feature_deep_delta"
    app_behavior: "Named trap presets set trap width/black width, join/end styles, trap appearance thresholds, image trap placement, and are assigned to page ranges for built-in or Adobe in-RIP trapping."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.trap-preset-options"
    source_url: "https://helpx.adobe.com/indesign/using/adjusting-ink-options-trapping.html"
    source_ids: [DD-S19]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.flattener-preview"
    name: "Flattener Preview panel"
    record_role: "feature_deep_delta"
    app_behavior: "Highlights page areas affected by transparency flattening (rasterized regions, outlined text/strokes) per flattener preset."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.advanced-color-techniques.fix-transparency-flattener-preview"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/advanced-color-techniques/fix-transparency-flattener-preview.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.appearance-of-black"
    name: "Appearance of Black policy"
    record_role: "feature_deep_delta"
    app_behavior: "Controls whether 100K black displays/prints as rich black or accurate black on RGB devices, per screen and export/print."
    primitive_domain: "color"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.color-management-pipeline"
    name: "Document color management pipeline"
    record_role: "feature_deep_delta"
    app_behavior: "Working spaces, per-document assigned profiles, per-image profiles, rendering intents, proofing, and print-time conversion policies form the color pipeline."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.use-color-management-when-printing"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/use-color-management-when-printing.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.color-and-output.color-panel-hsb"
    name: "Color panel and color modes"
    record_role: "feature_deep_delta"
    app_behavior: "Mixes colors in CMYK, RGB, Lab, or HSB with out-of-gamut warnings, tandem slider movement with Shift, and add-to-swatches."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.apply-color.define-and-manage-color-assets.mix-and-define-custom-colors"
    source_url: "https://helpx.adobe.com/indesign/desktop/apply-color/define-and-manage-color-assets/mix-and-define-custom-colors.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.interactive-and-epub] Interactive, Digital Publishing, and EPUB

```yaml
records:
  - id: "indesign.deep.interactive-and-epub.hyperlink-destinations"
    name: "Hyperlink destination types"
    record_role: "feature_deep_delta"
    app_behavior: "Hyperlinks target URL, File, Email, Page (with zoom setting), Text Anchor, or Shared Destination, managed in the Hyperlinks panel."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.hyperlinks.create-hyperlinks"
    source_url: "https://helpx.adobe.com/indesign/using/hyperlinks.html"
    source_ids: [DD-S24]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.hyperlink-appearance"
    name: "Hyperlink appearance and character style"
    record_role: "feature_deep_delta"
    app_behavior: "Visible/invisible rectangle with highlight (none/invert/outline/inset), color, width, plus optional character style on the source text and auto URL detection/conversion."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.hyperlinks.hyperlink-appearance-options"
    source_url: "https://helpx.adobe.com/indesign/using/hyperlinks.html"
    source_ids: [DD-S24]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.bookmarks-panel"
    name: "Bookmarks panel"
    record_role: "feature_deep_delta"
    app_behavior: "Creates nested, sortable PDF bookmarks from selections or TOC generation, with rename/delete and arrange operations."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.references-and-bookmarks.create-bookmarks"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/references-and-bookmarks/create-bookmarks.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.button-events"
    name: "Button event types"
    record_role: "feature_deep_delta"
    app_behavior: "On Release or Tap, On Click, On Roll Over, On Roll Off, On Focus, and On Blur trigger button actions."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.create-interactive-buttons-with-actions"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.button-pdf-actions"
    name: "Button actions (PDF set)"
    record_role: "feature_deep_delta"
    app_behavior: "Go To Destination, Go To First/Last/Next/Previous Page, Go To URL, Show/Hide Buttons and Forms, Sound, Video, Clear Form, Go To Next/Previous View, Open File, Print Form, Submit Form, and View Zoom."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.create-interactive-buttons-with-actions"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.interactive-and-epub.button-epub-actions"
    name: "Button actions (SWF/EPUB set)"
    record_role: "feature_deep_delta"
    app_behavior: "Animation, Go To Page, Go To State, Go To Next State, Go To Previous State, Sound, and Video actions target fixed-layout EPUB and legacy SWF output."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.create-interactive-buttons-with-actions"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.interactive-and-epub.button-appearance-states"
    name: "Button appearance states"
    record_role: "feature_deep_delta"
    app_behavior: "Normal, Rollover, and Click appearance states each hold distinct artwork; hidden-until-triggered supports popup patterns."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.change-and-manage-button-appearance"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.form-fields"
    name: "PDF form field types"
    record_role: "feature_deep_delta"
    app_behavior: "Check Box, Combo Box, List Box, Radio Button, Signature Field, and Text Field (plus buttons) with options like description, required, printable, multiline, password, read-only, sort items, and export values."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.create-fillable-forms"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.interactive-and-epub.tab-order"
    name: "Form/button tab order"
    record_role: "feature_deep_delta"
    app_behavior: "Per-page tab order dialog sequences keyboard focus across buttons and form fields."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.set-the-order-for-tabbing-between-buttons-using-tab-order"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.sample-buttons-library"
    name: "Sample Buttons and Forms library"
    record_role: "feature_deep_delta"
    app_behavior: "Built-in library of preconfigured buttons (navigation arrows preset with Go To Next/Previous Page) and form elements."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.buttons.create-and-add-buttons"
    source_url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    source_ids: [DD-S08]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.animation-panel"
    name: "Animation panel and motion presets"
    record_role: "feature_deep_delta"
    app_behavior: "Applies motion presets to objects with event triggers (On Page Load, On Page Click, On Click Self, On Roll Over Self, On Button Event), duration, play count/loop, speed easing, and animate from/to properties (opacity, rotation, scale, visibility)."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.animation.animate-documents-with-motion-presets"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/animation/motion-preset-options.html"
    source_ids: [DD-S21]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.motion-paths"
    name: "Motion path editing"
    record_role: "feature_deep_delta"
    app_behavior: "Animation motion paths are editable vector paths, and any drawn path can be converted to a motion path for a selected object."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.animation.edit-motion-paths"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/animation/edit-motion-paths.html"
    source_ids: [DD-S21]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.timing-panel"
    name: "Timing panel"
    record_role: "feature_deep_delta"
    app_behavior: "Sequences animations per trigger event with delays, reordering, linked play-together groups, and per-group play counts."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.animation.change-animation-order"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/animation/change-animation-order.html"
    source_ids: [DD-S21]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.custom-motion-presets"
    name: "Motion preset management"
    record_role: "feature_deep_delta"
    app_behavior: "Saves custom motion presets, duplicates/deletes them, and imports/exports presets as files compatible with Animate-style presets."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.animation.manage-motion-presets"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/animation/manage-motion-presets.html"
    source_ids: [DD-S21]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.epub-interactivity-preview"
    name: "EPUB Interactivity Preview panel"
    record_role: "feature_deep_delta"
    app_behavior: "In-app preview of animations, MSOs, buttons, and media for the current spread or whole document before export."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.preview-and-present-interactive-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/interactive-elements-and-forms/forms-and-pdfs/preview-and-present-interactive-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.page-transitions"
    name: "Page transitions"
    record_role: "feature_deep_delta"
    app_behavior: "Per-spread or all-spread transition presets (such as Blinds, Comb, Dissolve, Fade, Push, Wipe, Zoom In, and Page Turn for SWF) with direction and speed, honored in full-screen PDF and SWF."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.page-transitions.apply-page-transitions"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/page-transitions/apply-page-transitions.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.interactive-and-epub.epub-reflowable-general"
    name: "EPUB (Reflowable) export: General options"
    record_role: "feature_deep_delta"
    app_behavior: "EPUB version, cover source (none/rasterize first page/choose image), navigation TOC style, content order, split-document-by-style, and document metadata inclusion."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-epub.epub-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/export-content-epub-cc.html"
    source_ids: [DD-S13]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.epub-reflowable-text"
    name: "EPUB (Reflowable) export: Text options"
    record_role: "feature_deep_delta"
    app_behavior: "Footnote placement, list handling (map to ordered/unordered), and soft-return removal during export."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-epub.epub-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/export-content-epub-cc.html"
    source_ids: [DD-S13]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.epub-reflowable-object-conversion"
    name: "EPUB (Reflowable) export: Object and Conversion Settings"
    record_role: "feature_deep_delta"
    app_behavior: "Object appearance/CSS size handling plus image conversion to PNG/JPEG/GIF with resolution, alignment, and space settings."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-epub.epub-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/export-content-epub-cc.html"
    source_ids: [DD-S13]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.epub-reflowable-css-js-metadata"
    name: "EPUB (Reflowable) export: CSS, JavaScript, Metadata, Viewing Apps"
    record_role: "feature_deep_delta"
    app_behavior: "Additional CSS file attachment, custom JavaScript inclusion, EPUB metadata fields (title, creator, date, publisher, rights), and post-export viewing application choice."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-epub.epub-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/export-content-epub-cc.html"
    source_ids: [DD-S13]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.interactive-and-epub.epub-fixed-layout"
    name: "EPUB (Fixed Layout) export"
    record_role: "feature_deep_delta"
    app_behavior: "Preserves exact page geometry with spread control, cover and navigation TOC options, and conversion/metadata/viewing panes; carries animations, MSOs, and media."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-epub.export-to-epub"
    source_url: "https://helpx.adobe.com/indesign/using/export-content-epub-cc.html"
    source_ids: [DD-S13]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.interactive-pdf-export"
    name: "Interactive PDF export panels"
    record_role: "feature_deep_delta"
    app_behavior: "General (pages/spreads, full-screen, open settings, page transitions, forms and media inclusion), Compression, Advanced (accessibility, tagged PDF), and Security tabs."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.interactive-pdf-options"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.html5-export"
    name: "HTML5 and legacy HTML export"
    record_role: "feature_deep_delta"
    app_behavior: "Exports content as HTML5 (and legacy HTML) with content-order, image conversion, and CSS options for web reuse."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-html-and-web.export-content-as-html5"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/export-to-html-and-web/html5-export-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.articles-panel"
    name: "Articles panel"
    record_role: "feature_deep_delta"
    app_behavior: "Defines named article threads of frames controlling content order for EPUB/HTML export and tagged PDF reading order."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.export-to-html-and-web.create-and-manage-articles"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/export-to-html-and-web/create-and-manage-articles.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.legacy-swf-fla"
    name: "Legacy SWF/FLA export"
    record_role: "feature_deep_delta"
    app_behavior: "Exports interactive SWF (with page curl) and editable FLA for legacy Flash workflows."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.system-and-product-info.supported-file-formats"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.interactive-and-epub.publish-online-posture"
    name: "Publish Online provider posture"
    record_role: "feature_deep_delta"
    app_behavior: "Hosted document output with shareable URL, embed code, analytics, and dashboard management is an Adobe cloud service, not a local capability; Studio parity requires a local-first equivalent plus optional adapter."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.publish-work-online.publish-online-faq"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/publish-work-online/publish-online-faq.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.long-document] Books, TOC, and Indexing

```yaml
records:
  - id: "indesign.deep.long-document.book-panel-ops"
    name: "Book panel operations"
    record_role: "feature_deep_delta"
    app_behavior: "Adds/removes/reorders documents, designates the style source document, shows per-document status, and opens documents from the book list."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-and-manage-book-files.add-documents-to-book-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-and-manage-book-files/add-documents-to-book-files.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.book-synchronize"
    name: "Book synchronize options"
    record_role: "feature_deep_delta"
    app_behavior: "Synchronizes selected categories (styles, swatches, variables, numbered lists, cross-reference formats, conditional text, parent pages, trap presets) from the style source across book documents, with smart style-group matching."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.create-and-manage-book-files.sync-documents-books"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/create-and-manage-book-files/sync-documents-books.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.long-document.book-numbering"
    name: "Book page/chapter numbering"
    record_role: "feature_deep_delta"
    app_behavior: "Continues numbering across documents (continue, continue on next odd/even page with inserted blanks), updates numbering on demand, and supports turning automatic numbering off."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.page-numbers-chapters-and-sections.manage-book-page-numbering"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/page-numbers-chapters-and-sections/manage-book-page-numbering.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.book-output"
    name: "Book-wide output"
    record_role: "feature_deep_delta"
    app_behavior: "Prints, exports PDF/EPUB, preflights, and packages the whole book or selected documents from the book panel."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.print-or-export-book-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/print-production-and-file-creation/print-or-export-book-files.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.toc-dialog-options"
    name: "TOC dialog full options"
    record_role: "feature_deep_delta"
    app_behavior: "Included paragraph styles with per-style entry style, level, page number placement (before/after/none) and number character style, between-entry separator, sort alphabetically, run-in vs nested, include book documents, include hidden-layer text, numbered-paragraph handling, frame orientation, and Create PDF Bookmarks."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.add-a-table-of-contents.customize-toc-style"
    source_url: "https://helpx.adobe.com/indesign/using/creating-table-contents.html"
    source_ids: [DD-S23]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.multiple-tocs"
    name: "Multiple TOCs per document"
    record_role: "feature_deep_delta"
    app_behavior: "Distinct TOC styles generate multiple lists (contents, figures, tables, advertisers) in one document."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.add-a-table-of-contents.create-multiple-tocs"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/add-a-table-of-contents/create-multiple-tocs.html"
    source_ids: [DD-S23]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.interactive-toc"
    name: "Interactive TOC links"
    record_role: "feature_deep_delta"
    app_behavior: "TOC entries become live hyperlinks and PDF bookmarks for interactive PDF and EPUB navigation."
    primitive_domain: "interactive"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.add-a-table-of-contents.add-interactivity-to-tocs"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/add-a-table-of-contents/add-interactivity-to-tocs.html"
    source_ids: [DD-S23]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.index-topics-references"
    name: "Index topics and references (4 levels)"
    record_role: "feature_deep_delta"
    app_behavior: "Index panel Reference and Topic modes build up to four-level topic hierarchies with sort-by overrides and per-reference page-range scoping (to next style change, next use of style, end of story/document/section, custom paragraph count)."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.create-an-index.create-index-entries"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/create-an-index/create-index-entries.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.long-document.index-cross-references"
    name: "Index cross-references"
    record_role: "feature_deep_delta"
    app_behavior: "See, See also, See herein, See also herein, and custom cross-reference forms link index topics without page numbers."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.create-an-index.add-items-manually-to-an-index"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/create-an-index/add-items-manually-to-an-index.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.long-document.generate-index"
    name: "Generate Index dialog"
    record_role: "feature_deep_delta"
    app_behavior: "Generates the index story with title style, nested vs run-in format, section headings, include-book-documents, include-hidden-entries, and entry/separator style assignments."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.create-an-index.generate-and-format-an-index"
    source_url: "https://helpx.adobe.com/indesign/desktop/indexes-and-references/create-an-index/index-formatting-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.index-shortcuts"
    name: "Index entry capture shortcuts"
    record_role: "feature_deep_delta"
    app_behavior: "Keyboard shortcuts index the selected word, proper names (last-name-first), and open the New Index Entry dialog during text editing."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.indexes-and-references.create-an-index.create-index-entries"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.text-anchors"
    name: "Text anchors"
    record_role: "feature_deep_delta"
    app_behavior: "Named text anchor destinations support hyperlinks and cross-references to exact text positions across documents."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.hyperlinks.create-hyperlinks"
    source_url: "https://helpx.adobe.com/indesign/using/hyperlinks.html"
    source_ids: [DD-S24]
    verification_status: VERIFIED
  - id: "indesign.deep.long-document.sequential-paragraph-numbering"
    name: "Sequential paragraph numbering across books"
    record_role: "feature_deep_delta"
    app_behavior: "Numbered lists can continue across stories and across book documents for figure/table numbering."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.create-and-organize-pages.page-numbers-chapters-and-sections.use-sequential-paragraph-numbering"
    source_url: "https://helpx.adobe.com/indesign/desktop/create-and-organize-pages/page-numbers-chapters-and-sections/use-sequential-paragraph-numbering.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.output-and-prepress] Output, Prepress, XML, and Data Merge

```yaml
records:
  - id: "indesign.deep.output-and-prepress.live-preflight"
    name: "Live preflight engine"
    record_role: "feature_deep_delta"
    app_behavior: "Continuously validates the document against the active profile, reporting an error count in the status bar and per-error fix info in the Preflight panel, limitable to page ranges."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.preflight.live-preflighting"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/preflight/configure-and-use-the-preflight-panel.html"
    source_ids: [DD-S12]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.preflight-profile-categories"
    name: "Preflight profile rule categories"
    record_role: "feature_deep_delta"
    app_behavior: "Profiles group rules into General, Links (missing/modified), Color (blend space, plates, color spaces, overprint), Images and Objects (resolution, transparency, stroke weight), Text (missing fonts, overset), and Document (page size, page count, blanks, bleed/slug)."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.preflight.create-and-manage-preflight-profiles"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/preflight/create-and-manage-preflight-profiles.html"
    source_ids: [DD-S12]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.preflight-profile-management"
    name: "Preflight profile management and embedding"
    record_role: "feature_deep_delta"
    app_behavior: "Profiles are created, exported/imported as IDPP files, and embeddable in documents so recipients preflight with the same rules."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.preflight.create-and-manage-preflight-profiles"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03, DD-S12]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.print-general-setup"
    name: "Print dialog: General and Setup panels"
    record_role: "feature_deep_delta"
    app_behavior: "General selects pages/spreads, copies, collation, and layers-to-print; Setup controls paper size/orientation, scale (with constrain), fit-to-page, position on media, thumbnails, and tiling."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.page-set-up-and-printer-marks.specify-paper-size-and-orientation"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/page-set-up-and-printer-marks/specify-page-range.html"
    source_ids: [DD-S22]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.print-marks-bleed"
    name: "Print dialog: Marks and Bleed panel"
    record_role: "feature_deep_delta"
    app_behavior: "Crop marks, bleed marks, registration marks, color bars, and page information marks with offset and weight, plus bleed values and include-slug."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.page-set-up-and-printer-marks.set-printer-marks"
    source_url: "https://helpx.adobe.com/indesign/using/printers-marks-bleeds.html"
    source_ids: [DD-S22]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.print-output-panel"
    name: "Print dialog: Output panel"
    record_role: "feature_deep_delta"
    app_behavior: "Composite (leave unchanged/gray/RGB/CMYK) vs Separations vs In-RIP Separations, text-as-black, trapping mode, flip/negative, screening frequency/angle per ink, and Ink Manager access."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.color-output-options-for-composites"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/color-output-and-separations/create-color-separations.html"
    source_ids: [DD-S19, DD-S22]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.print-graphics-advanced"
    name: "Print dialog: Graphics, Color Management, Advanced, Summary"
    record_role: "feature_deep_delta"
    app_behavior: "Graphics controls image data sent (all/optimized/proxy/none), font download policy, and PostScript level; Color Management picks printer profile and rendering; Advanced sets OPI omission and flattener preset; Summary lists all settings."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.print-documents-and-books"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/print-production-and-file-creation/print-documents-and-books.html"
    source_ids: [DD-S22]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.print-presets"
    name: "Print presets"
    record_role: "feature_deep_delta"
    app_behavior: "Saves complete print dialog states as named presets, exportable and importable across machines."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.create-and-manage-print-presets"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/print-production-and-file-creation/create-and-manage-print-presets.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.print-as-bitmap"
    name: "Print as bitmap"
    record_role: "feature_deep_delta"
    app_behavior: "Rasterizes all page content at a chosen resolution when printing to non-PostScript printers."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.print-as-bitmap"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/print-production-and-file-creation/print-as-bitmap.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.postscript-eps"
    name: "PostScript and EPS file creation"
    record_role: "feature_deep_delta"
    app_behavior: "Creates device-independent or device-specific PostScript files via printer or file driver and exports pages as EPS."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.create-postscript-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/print/print-production-and-file-creation/create-postscript-files.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.pdf-export-general"
    name: "PDF (Print) export: General panel"
    record_role: "feature_deep_delta"
    app_behavior: "Preset and standard (PDF/X) choice, compatibility (Acrobat version), pages/spreads, view/layout open settings, layers export, and include options (bookmarks, hyperlinks, non-printing objects, visible guides/grids)."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.adobe-pdf-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.pdf-export-compression"
    name: "PDF (Print) export: Compression panel"
    record_role: "feature_deep_delta"
    app_behavior: "Per image class (color/grayscale/monochrome) downsampling method and threshold, compression codec (JPEG/ZIP/JPEG2000/CCITT), quality tiers, and crop-image-data-to-frames."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.adobe-pdf-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.pdf-export-output"
    name: "PDF (Print) export: Marks and Bleeds plus Output panels"
    record_role: "feature_deep_delta"
    app_behavior: "Printer marks and bleed/slug inclusion; color conversion (none/convert preserve numbers), destination profile, PDF/X output intent, and Ink Manager access."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.export-pdfs-for-printing"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.pdf-export-advanced-security"
    name: "PDF (Print) export: Advanced, Security, Summary panels"
    record_role: "feature_deep_delta"
    app_behavior: "Font subsetting threshold, OPI omission, flattener preset for legacy compatibility, tagged-PDF accessibility options, open/permissions passwords with print/copy restrictions, and a summary listing."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.adobe-pdf-export-options"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.pdfx-standards"
    name: "PDF/X standards support"
    record_role: "feature_deep_delta"
    app_behavior: "Exports PDF/X-1a, PDF/X-3, and PDF/X-4 compliant files with output intents through built-in presets."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.print-production-and-file-creation.produce-print-ready-pdf-files"
    source_url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    source_ids: [DD-S11]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.output-and-prepress.tagged-pdf-accessibility"
    name: "Tagged PDF and accessibility pipeline"
    record_role: "feature_deep_delta"
    app_behavior: "Style-to-tag mapping, per-object alt text and roles, articles-driven reading order, tab order, and document title metadata produce accessible tagged PDF."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.interactive-elements-and-forms.forms-and-pdfs.use-tags-for-accessible-pdfs"
    source_url: "https://helpx.adobe.com/indesign/desktop/interactive-elements-and-forms/forms-and-pdfs/accessible-pdfs.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.jpeg-png-export"
    name: "JPEG/PNG export options"
    record_role: "feature_deep_delta"
    app_behavior: "Exports selection, ranges, or all pages/spreads with quality, resolution, color space, anti-alias, bleed, and overlap settings."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.jpeg-and-png-export-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/save-and-export/jpeg-and-png-export-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.idml-export"
    name: "IDML export/open"
    record_role: "feature_deep_delta"
    app_behavior: "InDesign Markup Language provides backward compatibility to CS4 and cross-version document exchange as a zip-of-XML representation."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.system-and-product-info.supported-file-formats"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.tagged-text"
    name: "InDesign Tagged Text export/import"
    record_role: "feature_deep_delta"
    app_behavior: "Plain-text format with formatting tags round-trips complete InDesign text formatting for automation pipelines."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.system-and-product-info.supported-file-formats"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    source_ids: [DD-S03]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.xml-structure-tags"
    name: "XML structure pane and Tags panel"
    record_role: "feature_deep_delta"
    app_behavior: "Tags panel defines element tags; the Structure pane shows the element tree with drag placement, attribute editing, DTD loading/validation, and tagged-frame/text views."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.structure-and-tag-documents-for-xml"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/structure-and-tag-documents-for-xml.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.xml-map-tags-styles"
    name: "Map Tags to Styles / Styles to Tags"
    record_role: "feature_deep_delta"
    app_behavior: "Bidirectional mapping automates formatting of imported XML and tagging of styled content for export."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.import-xml-data-into-indesign"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/import-xml-data-into-indesign.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.output-and-prepress.xml-export"
    name: "Export XML with images"
    record_role: "feature_deep_delta"
    app_behavior: "Exports tagged content as XML with optional untagged-table handling and image copying/optimization to a support folder."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.save-and-export.export-to-xml"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/save-and-export/export-to-xml.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.data-merge-panel"
    name: "Data Merge panel"
    record_role: "feature_deep_delta"
    app_behavior: "Selects a CSV/TXT data source, drags text and image fields (@-prefixed columns) into placeholders, previews records, and generates merged documents."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.merge-data.merge-data"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/merge-data/merge-data.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.data-merge-layout-options"
    name: "Data merge multiple-records layout"
    record_role: "feature_deep_delta"
    app_behavior: "Multiple records per page with arrangement (rows first/columns first), margins, and spacing between records, plus content placement options (fitting, center, link images) and blank-line removal."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.merge-data.set-content-placement-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/merge-data/set-content-placement-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.data-merge-export-pdf"
    name: "Data merge direct PDF export"
    record_role: "feature_deep_delta"
    app_behavior: "Merges records straight to PDF without generating an intermediate InDesign document, honoring record ranges."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.merge-data.merge-records"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/merge-data/merge-records.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.output-and-prepress.data-merge-qr"
    name: "Data merge QR code fields"
    record_role: "feature_deep_delta"
    app_behavior: "Generates per-record QR codes from #-prefixed data columns during merge."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.merge-data.add-qr-codes-to-merged-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/merge-data/add-qr-codes-to-merged-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.output-and-prepress.background-tasks"
    name: "Background Tasks panel"
    record_role: "feature_deep_delta"
    app_behavior: "PDF export and Publish Online run as background tasks with progress and cancel in a dedicated panel."
    primitive_domain: "diagnostics"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/troubleshoot/file-and-output-issues/pdf-export-hangs-in-background.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.output-and-prepress.pdf-comments-export"
    name: "Review-ready PDF with comments round-trip"
    record_role: "feature_deep_delta"
    app_behavior: "Exported PDFs reviewed in Acrobat can return comments into InDesign via Import PDF Comments, anchored to layout positions."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.import-pdf-comments"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/import-pdf-comments.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.automation-and-scripting] Automation and Scripting

```yaml
records:
  - id: "indesign.deep.automation-and-scripting.scripts-panel"
    name: "Scripts panel"
    record_role: "feature_deep_delta"
    app_behavior: "Window > Utilities > Scripts lists scripts from application and user Scripts Panel folders; double-click runs a script, with reveal-in-Explorer/Finder access."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.automate-workflows-with-scripts"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/automate-workflows-with-scripts.html"
    source_ids: [DD-S04]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.script-label-panel"
    name: "Script Label panel"
    record_role: "feature_deep_delta"
    app_behavior: "Assigns string labels to page items so scripts can identify specific objects."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.automate-workflows-with-scripts"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/automate-workflows-with-scripts.html"
    source_ids: [DD-S04]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.scripting-languages"
    name: "Scripting language support"
    record_role: "feature_deep_delta"
    app_behavior: "ExtendScript (.jsx), UXP JavaScript (.idjs), AppleScript (macOS), and VBScript (Windows) drive the scripting DOM."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.automate-workflows-with-scripts"
    source_url: "https://developer.adobe.com/indesign/uxp/scripts/"
    source_ids: [DD-S04, DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.startup-scripts"
    name: "Startup scripts"
    record_role: "feature_deep_delta"
    app_behavior: "Scripts placed in a startup scripts folder run at application launch to install event listeners and menu items."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.automate-workflows-with-scripts"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/automate-workflows-with-scripts.html"
    source_ids: [DD-S04]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.sample-scripts"
    name: "Installable sample scripts"
    record_role: "feature_deep_delta"
    app_behavior: "Adobe-published sample scripts (including community GitHub scripts such as note alerts) install into the Scripts panel and run by double-click."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.automation-and-scripting.document-automation.automate-workflows-with-scripts"
    source_url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/automate-workflows-with-scripts.html"
    source_ids: [DD-S04]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.dom-application"
    name: "Scripting DOM: Application"
    record_role: "feature_deep_delta"
    app_behavior: "Root object exposes documents, preferences, menus, dialogs, script args/results, and doScript execution."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-document"
    name: "Scripting DOM: Document"
    record_role: "feature_deep_delta"
    app_behavior: "Document object owns spreads, pages, layers, stories, styles, swatches, links, sections, and export/print methods."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-spread-page"
    name: "Scripting DOM: Spread, Page, Layer"
    record_role: "feature_deep_delta"
    app_behavior: "Spread/Page objects hold page items and geometry; Layer objects expose visibility, lock, and stacking."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-pageitems"
    name: "Scripting DOM: PageItem hierarchy"
    record_role: "feature_deep_delta"
    app_behavior: "Rectangle, Oval, Polygon, GraphicLine, TextFrame, Group, and Button share the PageItem base with transforms, fills/strokes, and geometric bounds."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-text"
    name: "Scripting DOM: Story and Text hierarchy"
    record_role: "feature_deep_delta"
    app_behavior: "Story, Text, Paragraph, Line, Word, Character, InsertionPoint, and TextStyleRange objects expose full typographic attributes and find/change GREP methods."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-styles"
    name: "Scripting DOM: style collections"
    record_role: "feature_deep_delta"
    app_behavior: "ParagraphStyle, CharacterStyle, ObjectStyle, TableStyle, and CellStyle collections support creation, editing, grouping, and import by script."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-tables"
    name: "Scripting DOM: Table, Row, Column, Cell"
    record_role: "feature_deep_delta"
    app_behavior: "Table objects and their rows/columns/cells are fully scriptable including merges, strokes, fills, and content."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-books-xml"
    name: "Scripting DOM: Book, XML, hyperlink, index objects"
    record_role: "feature_deep_delta"
    app_behavior: "Books, XML elements/tags, hyperlinks, bookmarks, cross-references, and index topics are scriptable domains for long-document automation."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-export-prefs"
    name: "Scripting DOM: preference and export objects"
    record_role: "feature_deep_delta"
    app_behavior: "PDFExportPreference, EPubExportPreference, print preferences, and app/document preference objects script the full export/print surface."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.automation-and-scripting.dom-events"
    name: "Scripting events and menu actions"
    record_role: "feature_deep_delta"
    app_behavior: "Event listeners on application/document events and scriptable menu actions enable workflow hooks; UXP recipes document InDesign events and menus."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/recipes/indesign-events/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.uxp-plugins"
    name: "UXP plugins"
    record_role: "feature_deep_delta"
    app_behavior: "UXP plugins with manifest, entry points, panels, and lifecycle hooks are developed and debugged via UXP Developer Tools and distributed as packages."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.app-integrations.create-indesign-plugins-with-uxp"
    source_url: "https://developer.adobe.com/indesign/uxp/plugins/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.uxp-script-model"
    name: "UXP script lifecycle and debugging"
    record_role: "feature_deep_delta"
    app_behavior: "UXP scripts (.idjs) support global await, script arguments and results, and debugging via developer tooling."
    primitive_domain: "automation"
    dedupe_status: "new_surface"
    source_url: "https://developer.adobe.com/indesign/uxp/scripts/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.cpp-sdk-plugins"
    name: "C++ SDK and legacy CEP plugins"
    record_role: "feature_deep_delta"
    app_behavior: "Native C++ plug-ins and legacy CEP extensions remain installable third-party extension surfaces alongside UXP, with migration guides."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.app-integrations.install-plugins"
    source_url: "https://developer.adobe.com/indesign/uxp/resources/migration-guides/cep/"
    source_ids: [DD-S05]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.indesign-server"
    name: "InDesign Server automation runtime"
    record_role: "feature_deep_delta"
    app_behavior: "Headless InDesign Server runs the same scripting DOM at scale for template-driven production, with its own licensing and deployment packages."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.system-and-product-info.create-deploy-indesign-server-packages"
    source_url: "https://developer.adobe.com/indesign/uxp/introduction/applications/ids"
    source_ids: [DD-S05]
    verification_status: VERIFIED
  - id: "indesign.deep.automation-and-scripting.grep-everywhere"
    name: "GREP posture across features"
    record_role: "feature_deep_delta"
    app_behavior: "One GREP engine powers Find/Change GREP mode, GREP styles, and scriptable findGrep/changeGrep, so regex semantics must be identical across surfaces."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.glyphs-characters-and-expressions.construct-a-grep-expression"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/glyphs-characters-and-expressions/construct-a-grep-expression.html"
    source_ids: [DD-S14]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
```

### [SFR-INDESIGN-DEEP-DELTA.panels-and-workspace] Panels and Workspace

```yaml
records:
  - id: "indesign.deep.panels-and-workspace.window-menu-inventory"
    name: "Window menu panel inventory"
    record_role: "feature_deep_delta"
    app_behavior: "Window menu groups panels under Arrange, Workspace, and submenus: Color (Color, Gradient, Swatches, Adobe Color Themes), Editorial (Assignments, Notes, Track Changes), Interactive (Animation, Bookmarks, Buttons and Forms, EPUB Interactivity Preview, Hyperlinks, Liquid Layout, Media, Object States, Page Transitions, SWF Preview, Timing), Object & Layout (Align, Pathfinder, Transform), Output (Attributes, Flattener Preview, Preflight, Separations Preview, Trap Presets), Styles (Cell/Character/Object/Paragraph/Table Styles), Type & Tables (Character, Conditional Text, Cross-References, Glyphs, Index, Paragraph, Story, Table), and Utilities (Articles, Background Tasks, Data Merge, Script Label, Scripts, Tags, Tool Hints), plus top-level CC Libraries, Comments, Control, Effects, Info, Layers, Links, Pages, Properties, Stroke, Text Wrap, and Overlays."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/workspace-basics.html"
    source_ids: [DD-S25]
    verification_status: UNVERIFIED
    residual_reason: "The complete Window-menu panel enumeration is not published as one official list and helpx was bot-blocked this pass. Capture the exact panel inventory from an installed InDesign via the installed-app export playbook (32-adobe-installed-ui-export-playbook.md) before command-contract promotion; individual panels are already covered as their own rows, only the exhaustive menu roster is unconfirmed."
  - id: "indesign.deep.panels-and-workspace.properties-panel"
    name: "Properties panel"
    record_role: "feature_deep_delta"
    app_behavior: "Context-sensitive panel surfacing the most relevant controls for the current selection or document state."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/properties-panel.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.contextual-task-bar"
    name: "Contextual Task Bar"
    record_role: "feature_deep_delta"
    app_behavior: "Floating bar under the selection offering next-step actions, movable and hideable."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.toolbox.use-contextual-task-bar"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/use-contextual-task-bar.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.control-panel"
    name: "Control panel"
    record_role: "feature_deep_delta"
    app_behavior: "Docked context bar exposing character/paragraph or object transform controls per selection, keyboard-activatable and customizable."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.align-panel"
    name: "Align panel"
    record_role: "feature_deep_delta"
    app_behavior: "Align/distribute buttons with align-to scope selector and use-spacing distribute values."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.align-distribute-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/align-distribute-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.transform-panel"
    name: "Transform panel"
    record_role: "feature_deep_delta"
    app_behavior: "Numeric X/Y/W/H with reference-point proxy, scale percentages, rotation and shear angles, and panel-menu transform options (dimensions include stroke weight, transformations are totals)."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.object-transformation-settings-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/object-transformation-settings-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.info-panel"
    name: "Info panel"
    record_role: "feature_deep_delta"
    app_behavior: "Read-only readout of cursor position, selection size, rotation, colors, measure-tool results, and file info context."
    primitive_domain: "diagnostics"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.measure-distance-between-points"
    source_url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/rulers-and-measure-tools/measure-distance-between-points.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.panels-and-workspace.attributes-panel"
    name: "Attributes panel"
    record_role: "feature_deep_delta"
    app_behavior: "Per-object overprint fill/stroke/gap toggles and the Nonprinting flag."
    primitive_domain: "prepress"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-graphics-and-media.transform-and-arrange-objects.create-hidden-nonprinting-objects"
    source_url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/transform-and-arrange-objects/create-hidden-nonprinting-objects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.notes-panel"
    name: "Notes panel"
    record_role: "feature_deep_delta"
    app_behavior: "Lists and navigates inline editorial notes with author metadata and note preferences for color/display."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.track-changes-and-review.add-editorial-notes"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/track-changes-and-review/add-editorial-notes.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.assignments-panel"
    name: "Assignments panel"
    record_role: "feature_deep_delta"
    app_behavior: "Creates and manages InCopy assignment files, shows checked-out status icons, and packages assignments for editors."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.edit-with-incopy.create-and-manage-assignments"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/edit-with-incopy/create-and-manage-assignments.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.cc-libraries-panel"
    name: "CC Libraries panel"
    record_role: "feature_deep_delta"
    app_behavior: "Browses, places, and adds colors, text styles, and graphics to shared Creative Cloud Libraries; provider-dependent."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.app-integrations.manage-assets-cc-libraries"
    source_url: "https://helpx.adobe.com/indesign/desktop/app-integrations/manage-assets-cc-libraries.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.comments-panel"
    name: "Comments/Review panel"
    record_role: "feature_deep_delta"
    app_behavior: "Displays Share for Review and imported PDF comments in-context with reply, resolve, and filter operations; provider-dependent for cloud reviews."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.manage-feedback-for-shared-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/manage-feedback-for-shared-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.overlays-panel"
    name: "Overlays panel (legacy DPS)"
    record_role: "feature_deep_delta"
    app_behavior: "Legacy digital-publishing overlay settings (slideshows, hyperlinks, media) retained for older folio-era workflows."
    primitive_domain: "interactive"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/overlays-panel.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.panels-and-workspace.workspaces"
    name: "Named workspaces"
    record_role: "feature_deep_delta"
    app_behavior: "Preset workspaces (such as Essentials, Advanced, Book, Digital Publishing, Interactive for PDF, Printing and Proofing, Typography) plus save/delete/reset of custom workspaces capturing panel and menu state."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.create-and-manage-workspaces"
    source_url: "https://helpx.adobe.com/indesign/using/customizing-workspace.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.panels-and-workspace.panel-docking"
    name: "Panel docking and stashing"
    record_role: "feature_deep_delta"
    app_behavior: "Panels dock, group, stack, collapse to icons, float, and stash to screen edges, with open/close-all-stashed shortcuts and per-panel menus."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.customize-panels"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/customize-panels.html"
    source_ids: [DD-S01]
    verification_status: VERIFIED
  - id: "indesign.deep.panels-and-workspace.ui-scaling"
    name: "UI scaling"
    record_role: "feature_deep_delta"
    app_behavior: "Scales the whole interface (and optionally cursor size) across displays with slider control."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.scale-user-interface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/scale-user-interface.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.preferences] Preferences Categories

```yaml
records:
  - id: "indesign.deep.preferences.general"
    name: "Preferences: General"
    record_role: "feature_deep_delta"
    app_behavior: "Page numbering view (absolute/section), object scaling policy for stroke/effects, prevent-selection-of-locked-objects, and startup behaviors."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.set-general-preferences"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/set-general-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.interface"
    name: "Preferences: Interface"
    record_role: "feature_deep_delta"
    app_behavior: "Color theme brightness, pasteboard-matches-theme, cursor and gesture options, panel behavior, and live screen drawing."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.indesign-interface-preferences"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/indesign-interface-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.ui-scaling"
    name: "Preferences: UI Scaling"
    record_role: "feature_deep_delta"
    app_behavior: "Interface scale slider with cursor scaling on supported displays."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.scale-user-interface"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/scale-user-interface.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.type"
    name: "Preferences: Type"
    record_role: "feature_deep_delta"
    app_behavior: "Typographer's quotes, drag-and-drop text editing, smart text pasting, triple-click behavior, and apply-leading-to-whole-paragraph."
    primitive_domain: "typography"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.advanced-type"
    name: "Preferences: Advanced Type"
    record_role: "feature_deep_delta"
    app_behavior: "Superscript/subscript/small-cap size and position percentages, default composer input options, and missing-glyph protection."
    primitive_domain: "typography"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.composition"
    name: "Preferences: Composition"
    record_role: "feature_deep_delta"
    app_behavior: "Highlight toggles (keep violations, H&J violations, custom tracking/kerning, substituted fonts/glyphs) and text-wrap interaction rules (justify text next to object, skip by leading, wrap only affects text beneath)."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.format-and-style-text.tabs-indents-and-spacing.highlight-text-spacing"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.units-increments"
    name: "Preferences: Units & Increments"
    record_role: "feature_deep_delta"
    app_behavior: "Ruler origin (spread/page/spine), horizontal/vertical/other units, point/pica size basis, and keyboard increment values for cursor key, size/leading, baseline shift, and kerning/tracking."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.change-ruler-measurement-units"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.grids"
    name: "Preferences: Grids"
    record_role: "feature_deep_delta"
    app_behavior: "Baseline grid color/start/relative-to/increment/view threshold and document grid gridline spacing/subdivisions with grids-in-back option."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.grids.use-a-baseline-grid"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.guides-pasteboard"
    name: "Preferences: Guides & Pasteboard"
    record_role: "feature_deep_delta"
    app_behavior: "Guide colors (margins, columns, bleed, slug, preview background), smart guide categories, guides-in-back, snap zone, and pasteboard size margins."
    primitive_domain: "layout"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.layout-and-grid-tools.rulers-and-measure-tools.customize-ruler-guides"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.dictionary"
    name: "Preferences: Dictionary"
    record_role: "feature_deep_delta"
    app_behavior: "Per-language hyphenation/spelling provider selection, quote character defaults, user dictionary merge policy, and recompose-on-change."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.manage-language-dictionaries.change-dictionary-preferences"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/manage-language-dictionaries/change-dictionary-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.spelling"
    name: "Preferences: Spelling"
    record_role: "feature_deep_delta"
    app_behavior: "Find rules (misspelled, repeated, uncapitalized words/sentences) and dynamic spelling underline colors."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.spell-check.set-spelling-preferences"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/spell-check/set-spelling-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.autocorrect"
    name: "Preferences: Autocorrect"
    record_role: "feature_deep_delta"
    app_behavior: "Enables autocorrection with per-language misspelling/correction pair lists and capitalization fixes."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.language-and-proofing.spell-check.autocorrect-spelling-errors"
    source_url: "https://helpx.adobe.com/indesign/desktop/language-and-proofing/spell-check/autocorrect-spelling-errors.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.notes"
    name: "Preferences: Notes"
    record_role: "feature_deep_delta"
    app_behavior: "Note color, tooltips, spell-check/find inclusion of note content, and Story Editor note background."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.track-changes-and-review.add-editorial-notes"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.track-changes"
    name: "Preferences: Track Changes"
    record_role: "feature_deep_delta"
    app_behavior: "Which edit kinds are tracked and their marking colors/styles per user, plus change-bar options."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.track-changes-and-review.tracked-changes-preferences"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/track-changes-and-review/tracked-changes-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.story-editor-display"
    name: "Preferences: Story Editor Display"
    record_role: "feature_deep_delta"
    app_behavior: "Story Editor font, size, line spacing, text/background color themes, and cursor style."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.add-and-manage-text.add-and-manage-text-frames.open-and-use-story-editor"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.display-performance"
    name: "Preferences: Display Performance"
    record_role: "feature_deep_delta"
    app_behavior: "Default view mode, per-mode raster/vector/transparency quality sliders, and greek-type-below threshold."
    primitive_domain: "diagnostics"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.adjust-text-display-quality"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.gpu-performance"
    name: "Preferences: GPU Performance"
    record_role: "feature_deep_delta"
    app_behavior: "Enables GPU-accelerated display and animated zoom when a compatible GPU is present."
    primitive_domain: "diagnostics"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.system-and-product-info.gpu-performance"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/gpu-performance.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.appearance-of-black"
    name: "Preferences: Appearance of Black"
    record_role: "feature_deep_delta"
    app_behavior: "On-screen and print/export black rendering (accurate vs rich black) and overprint-of-100K-black policy."
    primitive_domain: "color"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.print.color-output-and-separations.change-the-black-overprint-setting"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.file-handling"
    name: "Preferences: File Handling"
    record_role: "feature_deep_delta"
    app_behavior: "Document recovery data folder, number of recent items, snippet import position, save-preview options, and link relink/preserve policies."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.recover-documents"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.clipboard-handling"
    name: "Preferences: Clipboard Handling"
    record_role: "feature_deep_delta"
    app_behavior: "Prefer PDF vs PDF+AICB clipboard formats and whether pasted external text keeps formatting or becomes plain text."
    primitive_domain: "document"
    dedupe_status: "new_surface"
    source_url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    source_ids: [DD-S10, DD-S26]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.publish-online"
    name: "Preferences: Publish Online"
    record_role: "feature_deep_delta"
    app_behavior: "Enables or disables the Publish Online feature; provider-dependent."
    primitive_domain: "export"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.save-export-and-publish.publish-work-online.publish-online-faq"
    source_url: "https://helpx.adobe.com/indesign/desktop/save-export-and-publish/publish-work-online/publish-online-faq.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.preferences.preference-files"
    name: "Preference file model"
    record_role: "feature_deep_delta"
    app_behavior: "InDesign Defaults and InDesign SavedData files under the user profile store preferences and caches; application defaults set with no document open persist for new documents."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.troubleshoot.settings-interface-and-feature-issues.preferences-support-file-locations"
    source_url: "https://helpx.adobe.com/indesign/desktop/troubleshoot/settings-interface-and-feature-issues/preferences-support-file-locations.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.preferences.reset-and-migrate"
    name: "Reset, export, and migrate settings"
    record_role: "feature_deep_delta"
    app_behavior: "Startup-modifier preference reset, Reset Settings command, export/import of user settings, and Migrate Previous Local Settings across versions."
    primitive_domain: "document"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.settings-and-preferences.export-and-import-user-settings"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/reset-settings-preferences.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.cloud-and-collab] Cloud and Collaboration Posture

```yaml
records:
  - id: "indesign.deep.cloud-and-collab.share-for-review"
    name: "Share for Review workflow"
    record_role: "feature_deep_delta"
    app_behavior: "Creates a hosted review link with invite-only or public access; reviewers add pin, highlight, strikethrough, insert, and reply comments viewed in-app; provider-dependent cloud service."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.share-for-review-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/share-for-review-overview.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
    verified_at: "2026-07-09"
  - id: "indesign.deep.cloud-and-collab.review-link-management"
    name: "Review link management"
    record_role: "feature_deep_delta"
    app_behavior: "Updates the shared version, manages reviewer access and comment permissions, and deletes review links; provider-dependent."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.share-documents-with-review-links"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/share-documents-with-review-links.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.cloud-documents"
    name: "Cloud documents"
    record_role: "feature_deep_delta"
    app_behavior: "Documents can be saved as Adobe cloud documents with autosave, version history, and cross-device access; provider-dependent alternative to local INDD."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.get-started.cloud-document-management-options"
    source_url: "https://helpx.adobe.com/indesign/desktop/get-started/cloud-document-management-options.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.invite-to-edit"
    name: "Invite collaborators to edit cloud documents"
    record_role: "feature_deep_delta"
    app_behavior: "Shares edit access to a cloud document with other Creative Cloud users; provider-dependent."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.invite-collaborators-to-edit-cloud-documents"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/invite-collaborators-to-edit-cloud-documents.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.projects"
    name: "Projects (shared cloud folders)"
    record_role: "feature_deep_delta"
    app_behavior: "Creates and shares project containers holding cloud documents and files for a team; provider-dependent."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.create-and-share-projects"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/create-and-share-projects.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.cc-libraries-sync"
    name: "CC Libraries asset sync and sharing"
    record_role: "feature_deep_delta"
    app_behavior: "Library assets sync through Creative Cloud and can be shared with other users with view/edit rights; provider-dependent."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.app-integrations.share-libraries-with-cc-users"
    source_url: "https://helpx.adobe.com/indesign/desktop/app-integrations/share-libraries-with-cc-users.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.adobe-fonts-activation"
    name: "Adobe Fonts auto-activation"
    record_role: "feature_deep_delta"
    app_behavior: "Missing fonts auto-activate from the Adobe Fonts service when available; provider-dependent with local font install as the offline path."
    primitive_domain: "typography"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.fonts.install-and-activate-fonts"
    source_url: "https://helpx.adobe.com/indesign/desktop/fonts/install-and-activate-fonts.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.view-custom-fonts-review"
    name: "Custom fonts in shared reviews"
    record_role: "feature_deep_delta"
    app_behavior: "Reviewers of shared documents see the document's custom fonts rendered in the hosted view; provider-dependent."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.share-and-collaborate.view-custom-fonts"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/share-and-collaborate/view-custom-fonts.html"
    source_ids: [DD-S25]
    verification_status: UNVERIFIED
    residual_reason: "The exact hosted-view custom-font rendering/substitution behavior in Share-for-Review is not surfaced by the reachable docs and helpx was bot-blocked this pass. This is a provider/cloud path (Studio maps it to a local-first review surface per 48-provider-offline-parity-registry.md); verify exact behavior against a live Share-for-Review session before command-contract promotion."
  - id: "indesign.deep.cloud-and-collab.incopy-file-workflow"
    name: "InCopy assignment workflow posture"
    record_role: "feature_deep_delta"
    app_behavior: "Assignment/ICML check-in/check-out collaboration is file-based (works on shared local/network storage) with an optional InCopy-on-the-web cloud path; largely local-first reproducible."
    primitive_domain: "collaboration"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.collaborate-and-review.edit-with-incopy.about-assignment-files"
    source_url: "https://helpx.adobe.com/indesign/desktop/collaborate-and-review/edit-with-incopy/about-assignment-files.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
  - id: "indesign.deep.cloud-and-collab.genai-partner-models"
    name: "Generative AI and partner models posture"
    record_role: "feature_deep_delta"
    app_behavior: "AI Assistant, text variation generation, text-to-image, and partner model selection are cloud provider features requiring adapter-based treatment for Studio parity."
    primitive_domain: "automation"
    dedupe_status: "deepens_existing"
    deepens_leaf_id: "indesign.leaf.generative-ai-features.gen-ai-features-overview"
    source_url: "https://helpx.adobe.com/indesign/desktop/generative-ai-features/gen-ai-features-overview.html"
    source_ids: [DD-S25]
    verification_status: VERIFIED
```

### [SFR-INDESIGN-DEEP-DELTA.sources] Sources

```yaml
sources:
  - id: DD-S01
    url: "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/keyboard-shortcuts.html"
    note: "Official keyboard shortcuts page; local snapshot _source_snapshots/indesign-keyboard-shortcuts-jina.md (body captured) verifies tool names, shortcut sets, and panel actions."
  - id: DD-S02
    url: "https://helpx.adobe.com/indesign/desktop/get-started/toolbox/view-select-tools.html"
    note: "Official toolbox page; local snapshot _source_snapshots/indesign-tools-jina.md captured TOC only (page body JS-rendered). Tool rows citing this source were verified 2026-07-09 via per-tool web-search snippet enumeration."
  - id: DD-S03
    url: "https://helpx.adobe.com/indesign/desktop/get-started/system-and-product-info/supported-file-formats.html"
    note: "Official supported file formats page; local snapshot _source_snapshots/indesign-supported-file-formats-jina.md (body captured) verifies open/save/export/place/package format tables."
  - id: DD-S04
    url: "https://helpx.adobe.com/indesign/desktop/automation-and-scripting/document-automation/automate-workflows-with-scripts.html"
    note: "Official scripting page; local snapshot _source_snapshots/indesign-scripting-jina.md (body captured) verifies Scripts panel, Script Label panel, script folders, and sample scripts."
  - id: DD-S05
    url: "https://developer.adobe.com/indesign/uxp/"
    note: "Official InDesign UXP developer docs; local snapshot _source_snapshots/indesign-uxp-dom-api-jina.md verifies scripts/plugins/recipes/UXP API tree and InDesign Server pages."
  - id: DD-S06
    url: "https://helpx.adobe.com/indesign/using/adding-transparency-effects.html"
    note: "Official transparency effects page; direct fetch blocked, nine-effect list and settings confirmed via web-search snippet enumeration on 2026-07-09."
  - id: DD-S07
    url: "https://helpx.adobe.com/indesign/desktop/layout-and-grid-tools/apply-layout-adjustments/liquid-page-rules-overview.html"
    note: "Official liquid page rules page; five rules confirmed via web-search snippet enumeration."
  - id: DD-S08
    url: "https://helpx.adobe.com/indesign/using/interactivity-5.html"
    note: "Official buttons and forms page; events/actions confirmed via search snippets. PDF and SWF/EPUB action sets and form-field option lists verified 2026-07-09 via snippet enumeration."
  - id: DD-S09
    url: "https://helpx.adobe.com/indesign/using/text-variables.html"
    note: "Official text variables page; variable types confirmed via search snippet enumeration."
  - id: DD-S10
    url: "https://helpx.adobe.com/indesign/using/setting-preferences.html"
    note: "Official preferences page (direct fetch blocked); pane list corroborated by DD-S26 enumeration."
  - id: DD-S11
    url: "https://helpx.adobe.com/indesign/using/pdf-options.html"
    note: "Official Adobe PDF export options page; print-PDF panel set and interactive-PDF four-tab set confirmed via search snippets."
  - id: DD-S12
    url: "https://helpx.adobe.com/indesign/desktop/print/preflight/create-and-manage-preflight-profiles.html"
    note: "Official preflight pages; rule categories confirmed via search snippet enumeration."
  - id: DD-S13
    url: "https://helpx.adobe.com/indesign/using/export-content-epub-cc.html"
    note: "Official EPUB export page; reflowable option tabs confirmed via search snippets. CSS/JavaScript/Metadata/Viewing Apps panes verified 2026-07-09 via snippet enumeration."
  - id: DD-S14
    url: "https://helpx.adobe.com/indesign/using/find-change.ug.html"
    note: "Official Find/Change page; Text/GREP/Glyph/Object modes confirmed, Color mode confirmed as dialog function via search snippets."
  - id: DD-S15
    url: "https://helpx.adobe.com/indesign/using/table-strokes-fills.html"
    note: "Official table strokes/fills and options pages; Table Options and Cell Options tab sets confirmed via search snippets."
  - id: DD-S16
    url: "https://helpx.adobe.com/indesign/using/anchored-objects.html"
    note: "Official anchored objects page; Inline/Above Line/Custom and Relative to Spine confirmed via search snippets."
  - id: DD-S17
    url: "https://helpx.adobe.com/indesign/using/text-composition.html"
    note: "Official text composition page; composer set and justification parameter ranges confirmed via search snippets."
  - id: DD-S18
    url: "https://www.adobe.com/support/indesign/gettingstarted/pdfs/indesign_howto_f_mso.pdf"
    note: "Official Adobe multistate-object guide; Object States panel operations confirmed via search snippets."
  - id: DD-S19
    url: "https://helpx.adobe.com/indesign/using/inks-separations-screen-frequency.html"
    note: "Official inks/separations page; Ink Manager options and trapping ink types confirmed via search snippets."
  - id: DD-S20
    url: "https://helpx.adobe.com/indesign/desktop/add-and-manage-text/add-and-manage-text-frames/change-text-frame-properties.html"
    note: "Official text frame properties page; five Text Frame Options tab groups confirmed via search snippets."
  - id: DD-S21
    url: "https://helpx.adobe.com/indesign/desktop/add-graphics-and-media/animation/motion-preset-options.html"
    note: "Official animation/motion-preset pages; events, Timing panel behavior, and play-together confirmed via search snippets."
  - id: DD-S22
    url: "https://macworld.com/article/1154573/IDprinting.html"
    note: "Third-party corroboration of the eight Print dialog panels; official print pages in leaf index provide the primary anchors."
  - id: DD-S23
    url: "https://helpx.adobe.com/indesign/using/creating-table-contents.html"
    note: "Official TOC page; dialog options confirmed via search snippets."
  - id: DD-S24
    url: "https://helpx.adobe.com/indesign/using/hyperlinks.html"
    note: "Official hyperlinks page; destination types and appearance options confirmed via search snippets."
  - id: DD-S25
    url: "https://helpx.adobe.com/indesign/desktop.html"
    note: "Official InDesign desktop help TOC (snapshot basis of 07-indesign-leaf-index.md); leaf URLs cited per-row anchor feature existence at page level."
  - id: DD-S26
    url: "https://ebookreading.net/view/book/EB9780470607169_10.html"
    note: "Third-party book excerpt enumerating the classic 18 Preferences panes, used as corroboration while direct helpx fetch was blocked."
  - id: DD-S27
    url: "https://developer.adobe.com/indesign/dom/api/"
    note: "Official InDesign scripting DOM API reference; class pages Application, Document, Spread, Layer, PageItem, Story, Table, PDFExportPreference, EPubExportPreference, and CrossReferenceType fetched directly with full bodies on 2026-07-09; verifies the automation-and-scripting DOM rows and the index cross-reference type list (See, See also, See herein, See also herein, Custom)."
  - id: DD-S28
    url: "https://developer.adobe.com/indesign/uxp/resources/fundamentals/object-model/"
    note: "Official UXP object-model overview fetched directly on 2026-07-09; confirms Application/Document/Story containment hierarchy and preference-object pattern (notes the diagram is non-comprehensive)."
  - id: DD-S29
    url: "https://helpx.adobe.com/indesign/using/aligning-text.html"
    note: "Official 'Align or justify text' help page; confirms Balance Ragged Lines (Paragraph/Control panel menu, requires Adobe Paragraph Composer, applies to Align Left/Center/Right). Added by the 2026-07-09 completeness-audit round 2 fill."
fetch_blocker_note: "Authoring pass 2026-07-09: helpx.adobe.com direct fetch timed out and Jina Reader relay returned 422; web.archive.org not fetchable. Verification pass later on 2026-07-09: Jina relay reaches helpx desktop pages but returns JS-shell navigation only (no article bodies); web.archive.org and help.adobe.com remain unreachable; developer.adobe.com fetches directly with full bodies. 68 of 71 flagged rows were upgraded to VERIFIED via per-topic web-search snippet enumeration and DD-S27/DD-S28 fetches; the remaining 3 UNVERIFIED rows are retained per instruction instead of being dropped."
```
