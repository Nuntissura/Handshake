---
file_id: 57-deep-delta-cross-app-overlap-map
file_kind: generated_overlap_map
topic_id: SFR-DEEP-DELTA-OVERLAP
title: "Deep-Delta Cross-App Overlap Map"
status: generated
updated_at: "2026-07-09"
generator: _tools/generate-deep-delta-overlap-map.py
source_files: [51, 52, 53, 54, 55]
deep_delta_row_count: 2275
overlap_group_count: 159
---

## [SFR-DEEP-DELTA-OVERLAP] Deep-Delta Cross-App Overlap Map

> GENERATED FILE - regenerate with `python _tools/generate-deep-delta-overlap-map.py` after any 51-55 change. Policy is identical to file 44: shared capability across source apps maps to ONE Handshake-native Studio primitive; app rows stay as source-specific provenance variants and are never deleted.

### [SFR-DEEP-DELTA-OVERLAP.coverage] Coverage

```yaml
coverage:
  deep_delta_row_count: 2275
  affinity_rows: 440
  figma_rows: 400
  illustrator_rows: 447
  indesign_rows: 415
  photoshop_rows: 573
  overlap_group_count: 159
  policy: shared_behavior_maps_to_one_studio_primitive_source_rows_stay_as_variants
```

### [SFR-DEEP-DELTA-OVERLAP.domain-counts] Per-Domain Row Counts

```yaml
domain_counts:
  automation:
    affinity: 14
    figma: 84
    illustrator: 29
    indesign: 32
    photoshop: 28
  camera_raw:
    affinity: 24
    photoshop: 51
  collaboration:
    affinity: 2
    figma: 28
    illustrator: 7
    indesign: 19
    photoshop: 6
  color:
    affinity: 50
    figma: 8
    illustrator: 38
    indesign: 21
    photoshop: 47
  component_system:
    figma: 38
  diagnostics:
    affinity: 8
    figma: 17
    illustrator: 6
    indesign: 9
    photoshop: 11
  document:
    affinity: 16
    figma: 38
    illustrator: 39
    indesign: 62
    photoshop: 25
  export:
    affinity: 37
    figma: 23
    illustrator: 42
    indesign: 26
    photoshop: 22
  interactive:
    affinity: 16
    figma: 6
    illustrator: 1
    indesign: 21
    photoshop: 29
  layer_graph:
    affinity: 55
    figma: 10
    illustrator: 12
    indesign: 3
    photoshop: 72
  layout:
    affinity: 25
    figma: 36
    illustrator: 22
    indesign: 75
    photoshop: 6
  prepress:
    affinity: 13
    indesign: 22
  prototype:
    figma: 39
  raster:
    affinity: 78
    figma: 17
    illustrator: 17
    indesign: 17
    photoshop: 189
  selection_mask:
    affinity: 23
    figma: 2
    illustrator: 18
    indesign: 4
    photoshop: 31
  typography:
    affinity: 24
    figma: 22
    illustrator: 48
    indesign: 82
    photoshop: 36
  vector:
    affinity: 55
    figma: 32
    illustrator: 168
    indesign: 22
    photoshop: 20
```

### [SFR-DEEP-DELTA-OVERLAP.groups] Cross-App Overlap Groups

```yaml
overlap_groups:
- overlap_key: "add"
  apps: [affinity, illustrator]
  primitive_domains: [layer_graph, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-boolean-add
  - affinity.deep.layers-and-adjustments.suite-blend-mode-add
  - illustrator.deep.effects.pathfinder-add
- overlap_key: "add anchor point tool"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.add-anchor-point-tool
  - indesign.deep.tools.add-anchor-point
  - photoshop.deep.tools.add-anchor-point-tool
- overlap_key: "add noise"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-add-noise
  - photoshop.deep.filters.noise-add-noise
- overlap_key: "adobe pdf preset"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [export]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.import-export.pdf-preset-groups
  - indesign.deep.menu-commands.file-adobe-pdf-presets
  - photoshop.deep.import-export-dialogs.adobe-pdf-presets
- overlap_key: "appearance black"
  apps: [illustrator, indesign]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.preferences.appearance-of-black
  - indesign.deep.preferences.appearance-of-black
- overlap_key: "application"
  apps: [illustrator, indesign]
  primitive_domains: [automation]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.automation-and-scripting.dom-application
  - indesign.deep.automation-and-scripting.dom-application
- overlap_key: "arrange"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document, interactive, layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.object-arrange
  - indesign.deep.menu-commands.window-arrange
  - photoshop.deep.menu-commands.window-arrange
- overlap_key: "artboard"
  apps: [illustrator, photoshop]
  primitive_domains: [layout]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.panels-and-workspace.panel-artboards
  - photoshop.deep.smart-and-linked.artboards
- overlap_key: "artboard tool"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [layout]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-artboard-tool
  - illustrator.deep.tools.artboard-tool
  - photoshop.deep.tools.artboard-tool
- overlap_key: "average"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-average
  - photoshop.deep.filters.blur-average
- overlap_key: "bevel emboss"
  apps: [indesign, photoshop]
  primitive_domains: [layer_graph, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.graphics-and-frames.effect-bevel-emboss
  - photoshop.deep.adjustments-and-blending.layer-style-bevel-emboss
- overlap_key: "black white adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-black-and-white
  - photoshop.deep.adjustments-and-blending.black-and-white
- overlap_key: "book"
  apps: [affinity, indesign]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.publisher-layout.publisher-books-afbook
  - indesign.deep.menu-commands.file-new-book
- overlap_key: "box blur"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-box-blur
  - photoshop.deep.filters.blur-box-blur
- overlap_key: "brightness contrast adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-brightness-contrast
  - photoshop.deep.adjustments-and-blending.brightness-contrast
- overlap_key: "brushe panel"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.suite-brushes-panel
  - photoshop.deep.panels-and-workspace.panel-brushes
- overlap_key: "bulleted numbered list"
  apps: [figma, indesign, photoshop]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - figma.deep.typography.lists
  - indesign.deep.menu-commands.type-bulleted-numbered-lists-menu
  - photoshop.deep.type-engine.bulleted-numbered-lists
- overlap_key: "change case"
  apps: [illustrator, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.type-change-case
  - indesign.deep.menu-commands.type-change-case
- overlap_key: "channel mixer adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-channel-mixer
  - photoshop.deep.adjustments-and-blending.channel-mixer
- overlap_key: "channel panel"
  apps: [affinity, photoshop]
  primitive_domains: [selection_mask]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.photo-channels-panel
  - photoshop.deep.channels-and-color.channels-panel
  - photoshop.deep.panels-and-workspace.panel-channels
- overlap_key: "clipboard handling"
  apps: [illustrator, indesign]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.preferences.clipboard-handling
  - indesign.deep.preferences.clipboard-handling
- overlap_key: "color"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [color, layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-color
  - illustrator.deep.panels-and-workspace.panel-color
  - photoshop.deep.adjustments-and-blending.blend-mode-color
- overlap_key: "color balance adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-color-balance
  - photoshop.deep.adjustments-and-blending.color-balance
- overlap_key: "color burn"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-color-burn
  - photoshop.deep.adjustments-and-blending.blend-mode-color-burn
- overlap_key: "color dodge"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-color-dodge
  - photoshop.deep.adjustments-and-blending.blend-mode-color-dodge
- overlap_key: "contextual task bar"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.panels-and-workspace.contextual-task-bar
  - indesign.deep.panels-and-workspace.contextual-task-bar
  - photoshop.deep.menu-commands.contextual-task-bar
- overlap_key: "create outline"
  apps: [illustrator, indesign]
  primitive_domains: [typography, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.type-create-outlines
  - indesign.deep.menu-commands.type-create-outlines
- overlap_key: "crop tool"
  apps: [affinity, photoshop]
  primitive_domains: [document, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-crop-tool
  - photoshop.deep.tools.crop-tool
- overlap_key: "curve adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-curves
  - photoshop.deep.adjustments-and-blending.curves
- overlap_key: "darken"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-darken
  - photoshop.deep.adjustments-and-blending.blend-mode-darken
- overlap_key: "darker color"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-darker-color
  - photoshop.deep.adjustments-and-blending.blend-mode-darker-color
- overlap_key: "delete anchor point tool"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.delete-anchor-point-tool
  - indesign.deep.tools.delete-anchor-point
  - photoshop.deep.tools.delete-anchor-point-tool
- overlap_key: "difference"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-difference
  - photoshop.deep.adjustments-and-blending.blend-mode-difference
- overlap_key: "diffuse"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-diffuse
  - photoshop.deep.filters.stylize-diffuse
- overlap_key: "diffuse glow"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-diffuse-glow
  - photoshop.deep.filters.distort-diffuse-glow
- overlap_key: "direct selection tool"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [selection_mask, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.direct-selection-tool
  - indesign.deep.tools.direct-selection
  - photoshop.deep.tools.direct-selection-tool
- overlap_key: "displace"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-displace
  - photoshop.deep.filters.distort-displace
- overlap_key: "divide"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [layer_graph, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-boolean-divide
  - affinity.deep.layers-and-adjustments.suite-blend-mode-divide
  - illustrator.deep.effects.pathfinder-divide
  - photoshop.deep.adjustments-and-blending.blend-mode-divide
- overlap_key: "document setup"
  apps: [illustrator, indesign]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.file-document-setup
  - indesign.deep.menu-commands.file-document-setup
- overlap_key: "drop shadow"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [layer_graph, raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.effects.stylize-drop-shadow
  - indesign.deep.graphics-and-frames.effect-drop-shadow
  - photoshop.deep.adjustments-and-blending.layer-style-drop-shadow
- overlap_key: "dust scratche"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-dust-and-scratches
  - photoshop.deep.filters.noise-dust-and-scratches
- overlap_key: "ellipse tool"
  apps: [affinity, illustrator, indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-ellipse
  - illustrator.deep.tools.ellipse-tool
  - indesign.deep.tools.ellipse
  - photoshop.deep.tools.ellipse-tool
- overlap_key: "eraser tool"
  apps: [illustrator, photoshop]
  primitive_domains: [raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.eraser-tool
  - photoshop.deep.tools.eraser-tool
- overlap_key: "exclusion"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-exclusion
  - photoshop.deep.adjustments-and-blending.blend-mode-exclusion
- overlap_key: "export"
  apps: [indesign, photoshop]
  primitive_domains: [export]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.menu-commands.file-export
  - photoshop.deep.preferences.export
- overlap_key: "exposure adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-exposure
  - photoshop.deep.adjustments-and-blending.exposure
- overlap_key: "eyedropper tool"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.eyedropper-tool
  - indesign.deep.tools.eyedropper
  - photoshop.deep.tools.eyedropper-tool
- overlap_key: "field blur"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-field-blur
  - photoshop.deep.filters.blur-gallery-field-blur
- overlap_key: "file handling"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.preferences.file-handling
  - indesign.deep.preferences.file-handling
  - photoshop.deep.preferences.file-handling
- overlap_key: "file info"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.file-file-info
  - indesign.deep.menu-commands.file-file-info
  - photoshop.deep.menu-commands.file-info-dialog
- overlap_key: "find replace font"
  apps: [illustrator, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.type-find-replace-fonts
  - indesign.deep.menu-commands.type-find-replace-fonts
- overlap_key: "free transform tool"
  apps: [illustrator, indesign]
  primitive_domains: [layout, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.free-transform-tool
  - indesign.deep.tools.free-transform
- overlap_key: "gaussian blur"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-gaussian-blur
  - photoshop.deep.filters.blur-gaussian-blur
- overlap_key: "general"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.preferences.general
  - indesign.deep.preferences.general
  - photoshop.deep.preferences.general
- overlap_key: "glyph"
  apps: [illustrator, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.type-glyphs-command
  - illustrator.deep.panels-and-workspace.panel-glyphs
  - indesign.deep.menu-commands.type-glyphs
- overlap_key: "gradient map adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-gradient-map
  - photoshop.deep.adjustments-and-blending.gradient-map
- overlap_key: "gradient tool"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [color, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-gradient-tool
  - illustrator.deep.tools.gradient-tool
  - photoshop.deep.tools.gradient-tool
- overlap_key: "hand tool"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.hand-tool
  - indesign.deep.tools.hand
  - photoshop.deep.tools.hand-tool
- overlap_key: "hard light"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-hard-light
  - photoshop.deep.adjustments-and-blending.blend-mode-hard-light
- overlap_key: "hard mix"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [color, layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-hard-mix
  - illustrator.deep.effects.pathfinder-hard-mix
  - photoshop.deep.adjustments-and-blending.blend-mode-hard-mix
- overlap_key: "healing brush tool"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-healing-brush-tool
  - photoshop.deep.tools.healing-brush-tool
- overlap_key: "high pass"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-high-pass
  - photoshop.deep.filters.other-high-pass
- overlap_key: "histogram panel"
  apps: [affinity, photoshop]
  primitive_domains: [diagnostics]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.photo-histogram-panel
  - photoshop.deep.panels-and-workspace.panel-histogram
- overlap_key: "history panel"
  apps: [affinity, photoshop]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.suite-history-panel
  - photoshop.deep.panels-and-workspace.panel-history
- overlap_key: "hue"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-hue
  - photoshop.deep.adjustments-and-blending.blend-mode-hue
- overlap_key: "info panel"
  apps: [affinity, indesign, photoshop]
  primitive_domains: [diagnostics]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.photo-info-panel
  - indesign.deep.panels-and-workspace.info-panel
  - photoshop.deep.panels-and-workspace.panel-info
- overlap_key: "inner glow"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [layer_graph, raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.effects.stylize-inner-glow
  - indesign.deep.graphics-and-frames.effect-inner-glow
  - photoshop.deep.adjustments-and-blending.layer-style-inner-glow
- overlap_key: "inner shadow"
  apps: [indesign, photoshop]
  primitive_domains: [layer_graph, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.graphics-and-frames.effect-inner-shadow
  - photoshop.deep.adjustments-and-blending.layer-style-inner-shadow
- overlap_key: "interface"
  apps: [indesign, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.preferences.interface
  - photoshop.deep.preferences.interface
- overlap_key: "intersect"
  apps: [affinity, illustrator]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-boolean-intersect
  - illustrator.deep.effects.pathfinder-intersect
- overlap_key: "invert adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-invert
  - photoshop.deep.adjustments-and-blending.invert
- overlap_key: "isolation mode"
  apps: [affinity, illustrator]
  primitive_domains: [interactive, layout]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.suite-isolation-mode
  - illustrator.deep.layout-and-artboards.isolation-mode
- overlap_key: "keyboard shortcut"
  apps: [illustrator, indesign]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.edit-keyboard-shortcuts
  - indesign.deep.menu-commands.edit-keyboard-shortcuts
- overlap_key: "knife tool"
  apps: [affinity, illustrator]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-knife-tool
  - illustrator.deep.tools.knife-tool
- overlap_key: "lasso tool"
  apps: [illustrator, photoshop]
  primitive_domains: [selection_mask]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.lasso-tool
  - photoshop.deep.tools.lasso-tool
- overlap_key: "layer panel"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.suite-layers-panel
  - photoshop.deep.panels-and-workspace.panel-layers
- overlap_key: "len blur"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-lens-blur
  - photoshop.deep.filters.blur-lens-blur
- overlap_key: "level adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-levels
  - photoshop.deep.adjustments-and-blending.levels
- overlap_key: "lighten"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-lighten
  - photoshop.deep.adjustments-and-blending.blend-mode-lighten
- overlap_key: "lighter color"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-lighter-color
  - photoshop.deep.adjustments-and-blending.blend-mode-lighter-color
- overlap_key: "lighting"
  apps: [affinity, illustrator]
  primitive_domains: [raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-lighting
  - illustrator.deep.effects.3d-lighting
- overlap_key: "line tool"
  apps: [indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.tools.line
  - photoshop.deep.tools.line-tool
- overlap_key: "linear burn"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-linear-burn
  - photoshop.deep.adjustments-and-blending.blend-mode-linear-burn
- overlap_key: "linear light"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-linear-light
  - photoshop.deep.adjustments-and-blending.blend-mode-linear-light
- overlap_key: "luminosity"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-luminosity
  - photoshop.deep.adjustments-and-blending.blend-mode-luminosity
- overlap_key: "magic wand tool"
  apps: [illustrator, photoshop]
  primitive_domains: [selection_mask]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.magic-wand-tool
  - photoshop.deep.tools.magic-wand-tool
- overlap_key: "matting"
  apps: [affinity, photoshop]
  primitive_domains: [color, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.color-and-formats.photo-matting
  - photoshop.deep.menu-commands.layer-matting-submenu
- overlap_key: "measure tool"
  apps: [affinity, illustrator, indesign]
  primitive_domains: [diagnostics, layout]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-measure-tool
  - illustrator.deep.tools.measure-tool
  - indesign.deep.tools.measure
- overlap_key: "missing font detection replacement"
  apps: [figma, photoshop]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - figma.deep.typography.missing-fonts
  - photoshop.deep.type-engine.missing-fonts-replacement
- overlap_key: "motion blur"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-motion-blur
  - photoshop.deep.filters.blur-motion-blur
- overlap_key: "move tool"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-move-tool
  - photoshop.deep.tools.move-tool
- overlap_key: "multiply"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-multiply
  - photoshop.deep.adjustments-and-blending.blend-mode-multiply
- overlap_key: "navigator panel"
  apps: [affinity, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.suite-navigator-panel
  - photoshop.deep.panels-and-workspace.panel-navigator
- overlap_key: "normal"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-normal
  - photoshop.deep.adjustments-and-blending.blend-mode-normal
- overlap_key: "note panel"
  apps: [indesign, photoshop]
  primitive_domains: [collaboration]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.panels-and-workspace.notes-panel
  - photoshop.deep.panels-and-workspace.panel-notes
- overlap_key: "note tool"
  apps: [indesign, photoshop]
  primitive_domains: [collaboration]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.tools.note
  - photoshop.deep.tools.note-tool
- overlap_key: "object selection tool"
  apps: [affinity, photoshop]
  primitive_domains: [selection_mask]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-object-selection-tool-ml
  - photoshop.deep.tools.object-selection-tool
- overlap_key: "open"
  apps: [indesign, photoshop]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.menu-commands.file-open
  - photoshop.deep.menu-commands.file-open
- overlap_key: "outer glow"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [layer_graph, raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.effects.stylize-outer-glow
  - indesign.deep.graphics-and-frames.effect-outer-glow
  - photoshop.deep.adjustments-and-blending.layer-style-outer-glow
- overlap_key: "overlay"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-overlay
  - photoshop.deep.adjustments-and-blending.blend-mode-overlay
- overlap_key: "package"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [document, export, prepress]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.file-package
  - indesign.deep.menu-commands.file-package
  - photoshop.deep.smart-and-linked.package-linked
- overlap_key: "patch tool"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-patch-tool
  - photoshop.deep.tools.patch-tool
- overlap_key: "pen tool"
  apps: [affinity, illustrator, indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-pen-tool
  - illustrator.deep.tools.pen-tool
  - indesign.deep.tools.pen
  - photoshop.deep.tools.pen-tool
- overlap_key: "pencil tool"
  apps: [affinity, illustrator, indesign, photoshop]
  primitive_domains: [raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-pencil-tool
  - illustrator.deep.tools.pencil-tool
  - indesign.deep.tools.pencil
  - photoshop.deep.tools.pencil-tool
- overlap_key: "performance"
  apps: [illustrator, photoshop]
  primitive_domains: [diagnostics]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.preferences.performance
  - photoshop.deep.preferences.performance
- overlap_key: "pin light"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-pin-light
  - photoshop.deep.adjustments-and-blending.blend-mode-pin-light
- overlap_key: "place"
  apps: [illustrator, indesign]
  primitive_domains: [document, export]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.file-place
  - indesign.deep.menu-commands.file-place
- overlap_key: "polygon tool"
  apps: [affinity, illustrator, indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-polygon
  - illustrator.deep.tools.polygon-tool
  - indesign.deep.tools.polygon
  - photoshop.deep.tools.polygon-tool
- overlap_key: "posterize adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-posterize
  - photoshop.deep.adjustments-and-blending.posterize
- overlap_key: "print"
  apps: [illustrator, indesign]
  primitive_domains: [export, prepress]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.file-print
  - indesign.deep.menu-commands.file-print
- overlap_key: "proof setup proof color"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.color-systems.proof-setup-soft-proofing
  - indesign.deep.menu-commands.view-proof-setup-colors
  - photoshop.deep.channels-and-color.proof-setup-colors
- overlap_key: "propertie panel"
  apps: [indesign, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.panels-and-workspace.properties-panel
  - photoshop.deep.panels-and-workspace.panel-properties
- overlap_key: "radial blur"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-radial-blur
  - photoshop.deep.filters.blur-radial-blur
- overlap_key: "rectangle tool"
  apps: [affinity, illustrator, indesign, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-rectangle
  - illustrator.deep.tools.rectangle-tool
  - indesign.deep.tools.rectangle
  - photoshop.deep.tools.rectangle-tool
- overlap_key: "revert"
  apps: [illustrator, photoshop]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.file-revert
  - photoshop.deep.menu-commands.file-revert
- overlap_key: "ripple"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-ripple
  - photoshop.deep.filters.distort-ripple
- overlap_key: "rotate tool"
  apps: [illustrator, indesign]
  primitive_domains: [layout, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.rotate-tool
  - indesign.deep.tools.rotate
- overlap_key: "rotate view tool"
  apps: [illustrator, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.rotate-view-tool
  - photoshop.deep.tools.rotate-view-tool
- overlap_key: "rounded rectangle tool"
  apps: [affinity, illustrator]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-rounded-rectangle
  - illustrator.deep.tools.rounded-rectangle-tool
- overlap_key: "running header"
  apps: [affinity, indesign]
  primitive_domains: [layout, typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.publisher-layout.publisher-running-headers
  - indesign.deep.text-and-typography.variable-running-header
- overlap_key: "satin"
  apps: [indesign, photoshop]
  primitive_domains: [layer_graph, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.graphics-and-frames.effect-satin
  - photoshop.deep.adjustments-and-blending.layer-style-satin
- overlap_key: "saturation"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-saturation
  - photoshop.deep.adjustments-and-blending.blend-mode-saturation
- overlap_key: "scale tool"
  apps: [illustrator, indesign]
  primitive_domains: [layout, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.scale-tool
  - indesign.deep.tools.scale
- overlap_key: "scissor tool"
  apps: [illustrator, indesign]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.scissors-tool
  - indesign.deep.tools.scissors
- overlap_key: "screen"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-screen
  - photoshop.deep.adjustments-and-blending.blend-mode-screen
- overlap_key: "screen mode"
  apps: [illustrator, indesign]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.view-screen-modes
  - indesign.deep.menu-commands.view-screen-modes
- overlap_key: "selection brush tool"
  apps: [affinity, photoshop]
  primitive_domains: [selection_mask]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-selection-brush-tool
  - photoshop.deep.tools.selection-brush-tool
- overlap_key: "selection tool"
  apps: [illustrator, indesign]
  primitive_domains: [selection_mask]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.selection-tool
  - indesign.deep.tools.selection
- overlap_key: "selective color adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-selective-color
  - photoshop.deep.adjustments-and-blending.selective-color
- overlap_key: "shadow highlight"
  apps: [affinity, photoshop]
  primitive_domains: [color, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-shadows-highlights
  - photoshop.deep.adjustments-and-blending.shadows-highlights
- overlap_key: "shape builder tool"
  apps: [affinity, illustrator]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-shape-builder-tool
  - illustrator.deep.tools.shape-builder-tool
- overlap_key: "shear tool"
  apps: [illustrator, indesign]
  primitive_domains: [layout, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.shear-tool
  - indesign.deep.tools.shear
- overlap_key: "slice tool"
  apps: [illustrator, photoshop]
  primitive_domains: [export]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.slice-tool
  - photoshop.deep.tools.slice-tool
- overlap_key: "smooth tool"
  apps: [illustrator, indesign]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.smooth-tool
  - indesign.deep.tools.smooth
- overlap_key: "soft light"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-soft-light
  - photoshop.deep.adjustments-and-blending.blend-mode-soft-light
- overlap_key: "spiral tool"
  apps: [affinity, illustrator]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-spiral
  - illustrator.deep.tools.spiral-tool
- overlap_key: "star tool"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-star
  - illustrator.deep.tools.star-tool
  - photoshop.deep.tools.star-tool
- overlap_key: "stroke"
  apps: [illustrator, photoshop]
  primitive_domains: [layer_graph, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.panels-and-workspace.panel-stroke
  - photoshop.deep.adjustments-and-blending.layer-style-stroke
- overlap_key: "subtract"
  apps: [affinity, illustrator, photoshop]
  primitive_domains: [layer_graph, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.designer-boolean-subtract
  - affinity.deep.layers-and-adjustments.suite-blend-mode-subtract
  - illustrator.deep.effects.pathfinder-subtract
  - photoshop.deep.adjustments-and-blending.blend-mode-subtract
- overlap_key: "tab"
  apps: [illustrator, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.panels-and-workspace.panel-tabs
  - indesign.deep.menu-commands.type-tabs
- overlap_key: "text frame option"
  apps: [affinity, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.typography.publisher-text-frame-options
  - indesign.deep.menu-commands.object-text-frame-options
- overlap_key: "text path"
  apps: [affinity, figma]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.typography.suite-path-text-options
  - figma.deep.typography.text-on-path
- overlap_key: "threshold adjustment"
  apps: [affinity, photoshop]
  primitive_domains: [color]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-adjustment-threshold
  - photoshop.deep.adjustments-and-blending.threshold
- overlap_key: "transform panel"
  apps: [affinity, indesign]
  primitive_domains: [layer_graph, layout]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.panels-and-workspace.suite-transform-panel
  - indesign.deep.panels-and-workspace.transform-panel
- overlap_key: "transform submenu"
  apps: [indesign, photoshop]
  primitive_domains: [layout, raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - indesign.deep.menu-commands.object-transform-submenu
  - photoshop.deep.menu-commands.edit-transform-submenu
- overlap_key: "trap"
  apps: [illustrator, photoshop]
  primitive_domains: [color, export]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.effects.pathfinder-trap
  - photoshop.deep.menu-commands.image-trap
- overlap_key: "triangle tool"
  apps: [affinity, photoshop]
  primitive_domains: [vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.designer-tools.suite-shape-triangle
  - photoshop.deep.tools.triangle-tool
- overlap_key: "twirl"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-twirl
  - photoshop.deep.filters.distort-twirl
- overlap_key: "type"
  apps: [illustrator, indesign, photoshop]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.preferences.type
  - indesign.deep.preferences.type
  - photoshop.deep.preferences.type
- overlap_key: "type path tool"
  apps: [illustrator, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.type-on-path-tool
  - indesign.deep.tools.type-on-a-path
- overlap_key: "type tool"
  apps: [illustrator, indesign]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.type-tool
  - indesign.deep.tools.type
- overlap_key: "undo redo"
  apps: [illustrator, indesign]
  primitive_domains: [document]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.menu-commands.edit-undo-redo
  - indesign.deep.menu-commands.edit-undo-redo
- overlap_key: "unsharp mask"
  apps: [affinity, photoshop]
  primitive_domains: [raster]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.photo-live-filter-unsharp-mask
  - photoshop.deep.filters.sharpen-unsharp-mask
- overlap_key: "variable font axi slider"
  apps: [figma, illustrator]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - figma.deep.typography.variable-font-axes
  - illustrator.deep.typography.variable-font-axes
- overlap_key: "vertical type tool"
  apps: [illustrator, photoshop]
  primitive_domains: [typography]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.tools.vertical-type-tool
  - photoshop.deep.tools.vertical-type-tool
- overlap_key: "vivid light"
  apps: [affinity, photoshop]
  primitive_domains: [layer_graph]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.layers-and-adjustments.suite-blend-mode-vivid-light
  - photoshop.deep.adjustments-and-blending.blend-mode-vivid-light
- overlap_key: "wave"
  apps: [illustrator, photoshop]
  primitive_domains: [raster, vector]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - illustrator.deep.effects.warp-wave
  - photoshop.deep.filters.distort-wave
- overlap_key: "workspace"
  apps: [figma, photoshop]
  primitive_domains: [collaboration, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - figma.deep.organization-admin.workspaces
  - photoshop.deep.menu-commands.window-workspace
  - photoshop.deep.preferences.workspace
- overlap_key: "zoom tool"
  apps: [affinity, illustrator, indesign, photoshop]
  primitive_domains: [document, interactive]
  studio_primitive_rule: one_studio_primitive_multiple_source_variants
  member_ids:
  - affinity.deep.photo-tools.photo-zoom-tool
  - illustrator.deep.tools.zoom-tool
  - indesign.deep.tools.zoom
  - photoshop.deep.tools.zoom-tool
```

### [SFR-DEEP-DELTA-OVERLAP.sources] Sources

```yaml
sources:
  - { id: DDO-S01, path: "54-affinity-deep-feature-delta.md", note: "affinity deep-delta rows." }
  - { id: DDO-S02, path: "55-figma-deep-feature-delta.md", note: "figma deep-delta rows." }
  - { id: DDO-S03, path: "52-illustrator-deep-feature-delta.md", note: "illustrator deep-delta rows." }
  - { id: DDO-S04, path: "53-indesign-deep-feature-delta.md", note: "indesign deep-delta rows." }
  - { id: DDO-S05, path: "51-photoshop-deep-feature-delta.md", note: "photoshop deep-delta rows." }
```
