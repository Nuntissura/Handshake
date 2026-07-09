---
file_id: 55-figma-deep-feature-delta
file_kind: deep_feature_delta
topic_id: SFR-FIGMA-DEEP-DELTA
title: "Figma Family Deep Feature Delta"
status: draft
app_key: figma
updated_at: "2026-07-09"
counts:
  total_rows: 400
  modalities: 13
  new_surface_rows: 225
  deepens_existing_rows: 175
  verified_rows: 244
  unverified_rows: 156
---

## [SFR-FIGMA-DEEP-DELTA] Figma Family Deep Feature Delta

This file is the deep feature/tool delta inventory over the existing Figma corpus (`21-figma-feature-map.md`, `23-figma-leaf-index.md`, `43-figma-source-distilled-feature-rows.md`). Every row is either a truly new surface (`new_surface`) or a more-granular deepening of an existing help-leaf topic (`deepens_existing` + `deepens_leaf_id`). Vendor names are research/provenance only per the folder naming policy. Cloud, org, billing, AI-model, and hosting behavior is marked provider-dependent inside `app_behavior`; Handshake Studio maps collaboration behavior onto its own local-first CRDT layer.

### [SFR-FIGMA-DEEP-DELTA.design-core] Design Core: Frames, Constraints, Auto Layout, Grids, Selection

```yaml
records:
- id: "figma.deep.design-core.frames-vs-groups-object-model"
  name: "Frames vs groups object model"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Frames are first-class containers with their own size, fills, strokes, effects, corner radius, layout grids, auto layout, and constraints, while groups are transient wrappers whose bounds derive entirely from children and carry no independent styling or layout."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039832054-the-difference-between-frames-and-groups"
  source_url: "https://help.figma.com/hc/en-us/articles/360039832054"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.frame-clip-content"
  name: "Frame clip content toggle"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Frames can clip child content to their bounds via a per-frame Clip content toggle, so children can intentionally overflow or be hard-masked by the frame rectangle."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041539473-frames-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/360041539473"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.frame-presets"
  name: "Frame device/size presets"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The frame tool offers preset frame sizes grouped by device class (phone, tablet, desktop, watch, social, paper) that stamp a correctly sized top-level frame in one click."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041539473-frames-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/360041539473"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.frame-resize-to-fit"
  name: "Resize frame to fit contents"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A resize-to-fit command shrinks or grows a frame so its bounds exactly wrap current children without moving them on canvas."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041539473-frames-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/360041539473"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.sections-container"
  name: "Sections as canvas organizers"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Sections are named canvas regions that group frames without affecting layout, carry their own fill color and label, can be marked ready-for-dev, and act as prototype state containers."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.9771500257687-organize-your-canvas-with-sections"
  source_url: "https://help.figma.com/hc/en-us/articles/9771500257687"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.pages-model"
  name: "Multi-page file model"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A design file contains an ordered list of pages, each an independent infinite canvas with its own background color and prototype flows, switchable from the left sidebar; page count is plan-limited on free tiers (provider-dependent limit, local concept)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.constraints-horizontal"
  name: "Horizontal constraints (left/right/left-and-right/center/scale)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Each child of a frame carries one horizontal constraint - left, right, left-and-right (stretch), center, or scale - that determines how its x position and width respond when the parent frame resizes."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957734"
  source_ids: [DEEP-S02]
  verification_status: VERIFIED
- id: "figma.deep.design-core.constraints-vertical"
  name: "Vertical constraints (top/bottom/top-and-bottom/center/scale)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Each child carries one vertical constraint - top, bottom, top-and-bottom (stretch), center, or scale - controlling y position and height on parent resize; default constraints are top-left."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957734"
  source_ids: [DEEP-S02]
  verification_status: VERIFIED
- id: "figma.deep.design-core.constraints-scale-mode"
  name: "Scale constraint proportional resizing"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The scale constraint stores the child's size and position as percentages of the parent frame so the child grows and shrinks proportionally with the frame on both axes."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957734"
  source_ids: [DEEP-S02]
  verification_status: VERIFIED
- id: "figma.deep.design-core.constraints-ignore-modifier"
  name: "Temporarily ignore constraints during resize"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Holding a modifier key (Ctrl/Cmd) while resizing a frame suspends child constraints for that drag, resizing the frame without repositioning children."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957734-apply-constraints-to-define-how-layers-resize"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957734"
  source_ids: [DEEP-S02]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-direction"
  name: "Auto layout flow direction (vertical/horizontal)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout frames flow children in a single vertical or horizontal direction, repositioning siblings automatically as children are added, removed, resized, or reordered."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-grid-flow"
  name: "Auto layout grid flow"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A third auto layout flow arranges children in a two-dimensional grid with resizable rows and columns and per-child row/column span controls."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-grid-span"
  name: "Grid child row/column span"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Children inside a grid-flow auto layout frame can span multiple rows or columns, and individual rows/columns can be resized."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-wrap"
  name: "Auto layout wrap"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Horizontal auto layout frames can wrap overflowing children onto the next line, producing responsive multi-row flows from a single container."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-gap"
  name: "Gap between items incl. auto distribution"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout spacing between children is a numeric gap or an Auto value that distributes children across the container (space-between semantics)."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-negative-gap"
  name: "Negative gap overlapping stacks"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Gap between items accepts negative values so children overlap (e.g. avatar stacks) while remaining ordered by the layout flow."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.auto-layout-canvas-stacking"
  name: "Canvas stacking order (first/last on top)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout frames expose a canvas-stacking setting that renders overlapping children with either the first or last item on top."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.auto-layout-padding"
  name: "Auto layout padding (uniform/axis/per-side)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout padding is settable uniformly, per axis (horizontal/vertical), or per individual side (top/right/bottom/left)."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-alignment"
  name: "Auto layout child alignment box + baseline"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A nine-position alignment box sets how children align inside the auto layout frame, with a separate text-baseline alignment toggle for aligning mixed-height text rows."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-resizing-hug-fill-fixed"
  name: "Resizing modes: hug contents / fill container / fixed"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout participants have per-axis resizing modes - hug contents (frame sizes to children), fill container (child stretches to parent), fixed - forming the responsive sizing contract."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-min-max"
  name: "Min/max width and height clamps"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout objects accept minimum and maximum width/height values that clamp hug and fill resizing so responsive components cannot collapse or overgrow."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-absolute-position"
  name: "Ignore auto layout (absolute position) children"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Individual children can be excluded from the auto layout flow (ignore auto layout, formerly absolute position), staying inside the frame with constraint-based positioning, e.g. notification badges."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-stroke-inclusion"
  name: "Strokes included/excluded from layout size"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "An advanced auto layout setting chooses whether child stroke weights are counted in layout spacing calculations or ignored."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.auto-layout-nesting"
  name: "Nested auto layout frames"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Auto layout frames nest arbitrarily deep, combining per-level direction, gap, padding, and resizing to express complete responsive component trees."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.auto-layout-suggested"
  name: "Suggested auto layout (AI-assisted conversion)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The editor can suggest converting a plain frame's contents into an equivalent auto layout structure automatically (provider-dependent AI assist; conversion result is local document state)."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040451373"
  source_ids: [DEEP-S01]
  verification_status: VERIFIED
- id: "figma.deep.design-core.layout-grid-uniform"
  name: "Uniform square layout grid"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Frames can carry a uniform square grid overlay with configurable cell size, color, and opacity for icon/pixel alignment work."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450513-create-layout-guides"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450513"
  source_ids: [DEEP-S03]
  verification_status: VERIFIED
- id: "figma.deep.design-core.layout-grid-columns-rows"
  name: "Column/row layout guides with fixed and stretch modes"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Column and row guides support count, fixed width/height with left/center/right (or top/center/bottom) anchoring plus offset, or stretch mode with margin and gutter, each with color and opacity."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450513-create-layout-guides"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450513"
  source_ids: [DEEP-S03]
  verification_status: VERIFIED
- id: "figma.deep.design-core.layout-grid-styles"
  name: "Reusable grid styles"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Layout guide configurations can be saved as named grid styles and reapplied across frames and files like any other shared style."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450513-create-layout-guides"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450513"
  source_ids: [DEEP-S03]
  verification_status: VERIFIED
- id: "figma.deep.design-core.multiple-layout-grids"
  name: "Multiple stacked layout grids per frame"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A single frame can stack several layout grids (e.g. columns plus rows plus uniform grid) that render simultaneously and toggle individually."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450513-create-layout-guides"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450513"
  source_ids: [DEEP-S03]
  verification_status: VERIFIED
- id: "figma.deep.design-core.ruler-guides"
  name: "Ruler guides on canvas and inside frames"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Draggable ruler guides live either on the page canvas or scoped inside a specific frame, snap objects during moves, and can be cleared per scope."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040449713-add-guides-to-the-canvas-or-frames"
  source_url: "https://help.figma.com/hc/en-us/articles/360040449713"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.measure-distances"
  name: "Alt-hover distance measurement (red lines)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Holding a modifier while hovering shows live pixel distances between the selected object and hovered object or frame edges on all four sides."
  primitive_domain: diagnostics
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956974-measure-distances-between-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956974"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.snapping-alignment-guides"
  name: "Object snapping and alignment guide lines"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Dragged objects snap to other object edges/centers, frame edges, guides, and equal-spacing positions, with transient red alignment lines and spacing badges rendered during the drag."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.snap-to-pixel-grid"
  name: "Pixel grid display and snap-to-pixel"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A 1px pixel grid becomes visible at high zoom and a snap-to-pixel-grid preference rounds geometry to whole (or half) pixels during draw/move/resize."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.smart-selection-tidy"
  name: "Smart selection tidy-up and spacing handles"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Tidy up converts a rough selection into an evenly spaced row/column/grid smart selection with draggable pink spacing handles and per-item reorder circles."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450233-arrange-layers-with-smart-selection"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450233"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.align-distribute-commands"
  name: "Align and distribute commands"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Selection-level commands align objects (left/center/right/top/middle/bottom) and distribute them with equal horizontal or vertical spacing relative to the selection or parent frame."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.multi-edit"
  name: "Multi-edit matching objects across frames"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Multi-edit mirrors move/resize/style/text/vector edits simultaneously onto matching objects in sibling frames or variants, using name/structure matching to pair targets."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.21635177948567-edit-objects-on-the-canvas-in-bulk"
  source_url: "https://help.figma.com/hc/en-us/articles/21635177948567"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.selection-descend-model"
  name: "Selection descend/deep-select model"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Click selects the top-level object, double-click or Enter descends one nesting level, modifier-click deep-selects the leaf under the cursor, and Shift-click adds/removes from the selection set."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040449873-select-layers-and-objects"
  source_url: "https://help.figma.com/hc/en-us/articles/360040449873"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.nudge-values"
  name: "Small/big nudge amounts"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Arrow keys move objects by a configurable small nudge and Shift+arrow by a configurable big nudge, with the same values reused for numeric field stepping."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.4404575206295-set-small-and-big-nudge-values"
  source_url: "https://help.figma.com/hc/en-us/articles/4404575206295"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.object-z-ordering"
  name: "Z-order commands (bring forward/send backward)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Objects reorder within their parent via bring-to-front, bring-forward, send-backward, send-to-back commands, mirrored by drag reordering in the layers panel."
  primitive_domain: layer_graph
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.lock-layers"
  name: "Layer locking"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Locked layers stay visible but ignore canvas clicks and drags; lock state is per-layer, inherited by children, and toggleable from the layers panel or shortcut."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041596573-lock-and-unlock-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360041596573"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.hide-layers"
  name: "Layer visibility toggling"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Per-layer visibility toggles remove objects from render and export while preserving them in the document tree; hidden state can be bound to boolean variables."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041112614-toggle-visibility-to-hide-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360041112614"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.scale-tool"
  name: "Scale tool vs resize semantics"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The scale tool (K) proportionally scales geometry plus applied properties like stroke weight, corner radius, effects, and font size, unlike bounding-box resize which stretches only dimensions."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040451453-scale-layers-while-maintaining-proportions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040451453"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.rotation-flip"
  name: "Rotation and flip transforms"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Objects rotate freely by handle drag or numeric angle and flip horizontally/vertically, with rotation preserved as a transform in the node's relative matrix."
  primitive_domain: layout
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956914-adjust-alignment-rotation-position-and-dimensions"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956914"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.math-in-fields"
  name: "Math expressions in numeric input fields"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Numeric property fields (x, y, w, h, rotation, etc.) accept arithmetic expressions (e.g. 100+8, /2) and relative operators that evaluate on commit."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.bulk-rename"
  name: "Batch rename with patterns"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Rename-layers batch dialog applies find/replace and pattern tokens (ascending/descending numbers, current name) across all selected layers in one operation."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039958934-rename-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360039958934"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.component-outlines-view"
  name: "Layer outline view mode"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Outline view renders only vector skeletons of all layers without fills/effects for structural inspection and precise node picking."
  primitive_domain: diagnostics
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.5724448965527-view-layer-outlines-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/5724448965527"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.find-replace-scope"
  name: "Find and replace with scope filters"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Find searches text, layer names, and asset types across the current page or whole file with match navigation and bulk text replace."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.9141292269847-find-and-replace-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/9141292269847"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.zoom-view-options"
  name: "Zoom modes (fit/selection/100%) and view options"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Viewport commands zoom to fit page, zoom to selection, zoom to 100%, and step zoom levels, with toggles for rulers, outlines, pixel preview, and multiplayer cursors."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041065034-adjust-your-zoom-and-view-options"
  source_url: "https://help.figma.com/hc/en-us/articles/360041065034"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.paste-semantics"
  name: "Paste semantics (in place, over selection, to replace)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Paste variants place copies at source coordinates (paste here/in place), inside a selected container, or replace the selected object while preserving position."
  primitive_domain: layer_graph
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.4409078832791-copy-and-paste-objects"
  source_url: "https://help.figma.com/hc/en-us/articles/4409078832791"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.duplicate-repeat"
  name: "Duplicate with offset repetition"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Duplicate (Ctrl/Cmd+D) repeats the previous duplicate's spatial offset, generating evenly spaced series from one move-after-duplicate."
  primitive_domain: layer_graph
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.keyboard-shortcut-panel"
  name: "Keyboard shortcut reference panel"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A built-in shortcut panel lists all keyboard shortcuts grouped by category and highlights which the user has already used."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040328653-use-figma-products-with-a-keyboard"
  source_url: "https://help.figma.com/hc/en-us/articles/360040328653"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.design-core.quick-actions-palette"
  name: "Quick actions command palette"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A searchable command palette runs any menu command, plugin, or widget by name, including parameterized plugin runs."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23570416033943-use-the-actions-menu-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23570416033943"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.vector-and-draw] Vector Networks, Strokes, Fills, Effects, Draw

```yaml
records:
- id: "figma.deep.vector-and-draw.vector-network-topology"
  name: "Vector network multi-edge topology"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Vector layers are networks, not single paths: any point can join three or more edges, edges are first-class selectable segments, and enclosed regions exist without a single closed path."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450213-vector-networks"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450213"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.vector-region-fill"
  name: "Paint-bucket region fills in vector networks"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "In vector edit mode a paint-bucket control fills or unfills individual enclosed regions of the network independently, so one vector layer can contain filled and unfilled regions."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450213-vector-networks"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450213"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.pen-tool"
  name: "Pen tool point/edge placement"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The pen tool places anchor points and straight/curved edges, connects to any existing network point (not just endpoints), and drag-creates mirrored tangent handles."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957634-edit-vector-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957634"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.bend-tool"
  name: "Bend tool curve toggling"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The bend tool converts straight edges to curves by dragging and toggles points between corner and smooth by click, adjusting tangent handles with optional independent-handle breaking."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957634-edit-vector-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957634"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.vector-handle-mirroring"
  name: "Tangent handle mirroring modes"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Anchor handles support mirroring modes (no mirroring, mirror angle, mirror angle and length) that govern how the two tangents of a point move relative to each other."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957634-edit-vector-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957634"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.pencil-tool"
  name: "Pencil freehand sketching"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The pencil tool draws freehand strokes that are auto-smoothed into vector paths using the active stroke fill, weight, and style."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.4402723791511-sketch-on-the-canvas-with-the-pencil-tool"
  source_url: "https://help.figma.com/hc/en-us/articles/31440438150935"
  source_ids: [DEEP-S37]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.brush-tool"
  name: "Brush tool painting"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "The brush tool paints textured, organic strokes on canvas with per-stroke fill, weight, and brush style selected from a secondary toolbar."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31440438150935"
  source_ids: [DEEP-S37]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.custom-brushes"
  name: "Custom brushes from vector layers"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "Any single vector layer (shape, path, flattened text) can be captured as a reusable custom brush style applied along painted strokes."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31440438150935"
  source_ids: [DEEP-S37]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.scatter-brushes"
  name: "Scatter brushes"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "Scatter brushes distribute brush stamps along the stroke path with spacing/scatter variation for depth and texture effects."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://www.figma.com/blog/figma-draw-scatter-brushes/"
  source_ids: [DEEP-S38]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.stroke-weight-per-side"
  name: "Stroke weight incl. per-side weights"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Stroke weight is set in pixels for the whole shape or independently per side (all/top/bottom/left/right/custom), and stroke weight is not counted in layer dimensions."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.stroke-align"
  name: "Stroke alignment inside/center/outside"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Strokes render inside, centered on, or outside the layer path, changing visual bounds without changing the stored geometry."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.stroke-caps-arrowheads"
  name: "Stroke caps and arrowhead endpoints"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Open-path endpoints choose caps: none, round, square, line arrow, triangle arrow, reverse triangle, diamond; closed/branching path endpoints are configured in advanced stroke settings."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.stroke-joins-miter"
  name: "Stroke joins and miter angle"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Path corners join with miter, bevel, or round styles, with a miter-angle threshold controlling when miters collapse to bevels."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.stroke-dashes"
  name: "Dash patterns: basic/dashed/dotted/custom"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Stroke styles include solid, dashed with dash/gap values, dotted (1px dashes with round caps), and custom multi-value dash-gap sequences, each with dash cap choice of none/round/square."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.stroke-width-profile"
  name: "Variable-width stroke profile (taper)"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "A width-profile control tapers stroke ends to simulate pressure, giving brush/calligraphy-like variable width along the path."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.brush-stroke-type"
  name: "Brush stroke type on paths"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "Any vector path's stroke can switch to a brush stroke type with hand-painted appearance and direction control, distinct from painting with the brush tool."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.dynamic-stroke"
  name: "Dynamic stroke (shake/wiggle)"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "Dynamic stroke renders a hand-drawn bumpy line with frequency, wiggle, and smoothen parameters applied non-destructively to the path."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04, DEEP-S38]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.multiple-stroke-fills"
  name: "Multiple fills per stroke"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A stroke accepts multiple stacked fills (solid/gradient/image/pattern) each with its own opacity and blend behavior, like layer fills."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360049283914"
  source_ids: [DEEP-S04]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.boolean-operations-live"
  name: "Live boolean groups (union/subtract/intersect/exclude)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Boolean operations create live, editable boolean groups whose children remain movable/editable while the composite outline updates, until explicitly flattened."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957534-boolean-operations"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957534"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.flatten"
  name: "Flatten to single vector layer"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Flatten merges a selection (including boolean groups and text outlines) into one vector network layer, baking composite geometry destructively."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.30101373312279-flatten-layers"
  source_url: "https://help.figma.com/hc/en-us/articles/30101373312279"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.outline-stroke"
  name: "Outline stroke conversion"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Outline stroke converts a stroked path into filled vector geometry matching the stroke's weight, align, caps, joins, and dashes."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.33052305733015-convert-strokes-to-vector-paths"
  source_url: "https://help.figma.com/hc/en-us/articles/33052305733015"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.corner-radius-per-corner"
  name: "Per-corner independent radius"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Rectangles and frames accept a uniform corner radius or four independent per-corner radii, and vector points accept per-point corner rounding."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.corner-smoothing"
  name: "Corner smoothing (squircle) control"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A corner-smoothing percentage blends rounded corners toward continuous-curvature squircles (60% approximates platform icon curvature)."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.shape-rectangle-line-ellipse"
  name: "Shape primitives: rectangle, line, ellipse"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Rectangle, line, and ellipse tools stamp parametric primitives that stay editable as shape properties before conversion to raw vector networks."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450133-shape-tools"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450133"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.shape-polygon-star"
  name: "Polygon and star parametric shapes"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Polygon shapes expose a side-count parameter and stars expose point-count plus inner-radius ratio, both adjustable via canvas handles."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450133-shape-tools"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450133"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.arc-parameters"
  name: "Ellipse arc parameters (start/sweep/ratio)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Ellipses carry arc controls - start angle, sweep, and inner ratio - producing arcs, semicircles, rings, and donut segments parametrically."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450173-arc-tool-create-arcs-semi-circles-and-rings"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450173"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.masks-use-as-mask"
  name: "Use-as-mask sibling masking"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A layer marked use-as-mask clips all above siblings in the same group to its region, keeping the mask non-destructive and reversible."
  primitive_domain: selection_mask
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450253-masks"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450253"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.mask-types"
  name: "Mask modes: alpha, vector, luminance"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Masks can operate in alpha (uses mask transparency), vector (hard outline), or luminance (uses brightness) modes selected on the mask layer."
  primitive_domain: selection_mask
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040450253-masks"
  source_url: "https://help.figma.com/hc/en-us/articles/360040450253"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.fill-solid"
  name: "Solid fill"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Solid fills apply a single color with per-fill opacity; objects stack multiple fills with individual visibility toggles, reorder handles, and per-fill removal."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041003694-guide-to-fills"
  source_url: "https://help.figma.com/hc/en-us/articles/360041003694"
  source_ids: [DEEP-S36]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.fill-gradient-types"
  name: "Gradient fills: linear/radial/angular/diamond"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Gradient fills support linear, radial, angular, and diamond geometry with multi-stop color ramps and on-canvas handle editing of position/rotation/extent."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.34208860210199-use-gradients-as-a-fill-or-stroke"
  source_url: "https://help.figma.com/hc/en-us/articles/34208860210199"
  source_ids: [DEEP-S36]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.fill-image-modes"
  name: "Image fill modes: fill/fit/crop/tile"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Image fills scale via fill (cover), fit (contain), crop (manual transform inside bounds), or tile (repeat at set scale) modes, with rotation in 90-degree steps."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041098433-adjust-the-properties-of-an-image"
  source_url: "https://help.figma.com/hc/en-us/articles/360041098433"
  source_ids: [DEEP-S36]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.image-adjustments"
  name: "Non-destructive image adjustments"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Image fills expose non-destructive adjustment sliders (exposure, contrast, saturation, temperature, tint, highlights, shadows) applied at render time."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041098433-adjust-the-properties-of-an-image"
  source_url: "https://help.figma.com/hc/en-us/articles/360041098433"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.fill-video"
  name: "Video fills"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Layers accept video or animated GIF fills that play in prototypes (upload is plan-gated, provider-dependent; playback semantics are a local render concern)."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041003694-guide-to-fills"
  source_url: "https://help.figma.com/hc/en-us/articles/360041003694"
  source_ids: [DEEP-S36]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.fill-pattern"
  name: "Pattern fills referencing canvas objects"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "Pattern fills tile another object (layer, group, or frame) from the same file as a live-source repeating fill or stroke with spacing/alignment controls."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.31616030150167-use-patterns-as-a-fill-or-stroke"
  source_url: "https://help.figma.com/hc/en-us/articles/31616030150167"
  source_ids: [DEEP-S43]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-drop-shadow"
  name: "Drop shadow effect (up to 8)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Up to eight drop shadows per layer, each with x/y offset, blur, spread (on rectangles/ellipses/frames/components), color+opacity, and a show-behind-transparent-areas toggle."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-inner-shadow"
  name: "Inner shadow effect (up to 8)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Up to eight inner shadows per layer with x/y offset, blur, spread, and color+opacity rendered inside the layer bounds."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-layer-blur"
  name: "Layer blur (uniform)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "One layer blur per layer applies a uniform gaussian blur to the layer's own rendered content."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-progressive-blur"
  name: "Progressive blur"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Layer and background blurs offer a progressive variant with controllable size, direction, and start/end intensity forming a blur gradient."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-background-blur"
  name: "Background blur"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "One background blur per layer blurs content behind the layer; the layer fill must be semi-transparent (0.10-99.99% opacity) for the effect to show."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-noise"
  name: "Noise effect (mono/duo/multi)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Up to two noise effects per layer with mono/duo/multi color modes, x/y noise size, density, and color+opacity controls."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-texture"
  name: "Texture effect"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "One texture effect per layer distorts the render with x/y size, radius (spread beyond bounds), and clip-to-shape toggle."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-glass"
  name: "Glass effect"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "One glass effect per layer simulates refractive material with light angle, light intensity, refraction, depth, dispersion, frost, and splay parameters."
  primitive_domain: raster
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.effect-styles"
  name: "Effect styles"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Effect stacks save as named, publishable effect styles reusable across files like color/text/grid styles."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360041488473"
  source_ids: [DEEP-S05]
  verification_status: VERIFIED
- id: "figma.deep.vector-and-draw.blend-modes-pass-through"
  name: "Blend modes incl. pass-through groups"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Layers, fills, and effects each take a blend mode from the standard set (darken, multiply, screen, overlay, etc.), and groups/frames default to pass-through so children blend with content below."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040667874-apply-blend-modes-to-layers-fills-and-effects"
  source_url: "https://help.figma.com/hc/en-us/articles/360040667874"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.color-picker-models"
  name: "Color picker models and inputs"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The color picker accepts HEX, RGB, HSL, HSB, and CSS inputs, supports an eyedropper, document/library color swatches, and per-channel scrubbing."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041003774-update-fills-using-the-color-picker"
  source_url: "https://help.figma.com/hc/en-us/articles/360041003774"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.document-color-profile"
  name: "Document color profile (sRGB / Display P3)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Files carry a color profile setting (sRGB or Display P3) that governs color interpretation, with unmanaged legacy behavior as a fallback."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360043042113-about-color-models"
  source_url: "https://help.figma.com/hc/en-us/articles/360043042113"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.shape-builder-modes"
  name: "Shape builder merge/extract modes"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Shape builder drags across overlapping vector regions to merge them or clicks to extract/delete regions, producing custom shapes without manual boolean stacking."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.31616004109847-create-custom-shapes-with-the-shape-builder-tool"
  source_url: "https://help.figma.com/hc/en-us/articles/31616004109847"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.offset-path-options"
  name: "Offset path distance/join options"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Offset path generates inset/outset copies of a path at a numeric distance with join-style handling of corners."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.33792861450263-offset-a-vector-path"
  source_url: "https://help.figma.com/hc/en-us/articles/33792861450263"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.simplify-path-strength"
  name: "Simplify path point reduction"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Simplify reduces anchor point count on a path with an adjustable strength while approximating the original curve."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.33792593975575-simplify-a-vector-path"
  source_url: "https://help.figma.com/hc/en-us/articles/33792593975575"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.eyedropper-anywhere"
  name: "Eyedropper screen sampling"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The eyedropper samples colors from anywhere on the canvas including rendered images and gradients, applying to the active fill or stroke."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.27643269375767-sample-colors-with-the-eyedropper-tool"
  source_url: "https://help.figma.com/hc/en-us/articles/27643269375767"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.image-crop-transform"
  name: "Image crop with in-bounds transform"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Crop mode repositions, scales, and rotates the image source within the layer bounds non-destructively, keeping the full source recoverable."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040675194-crop-an-image"
  source_url: "https://help.figma.com/hc/en-us/articles/360040675194"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.vector-and-draw.mixed-selection-color-listing"
  name: "Selection colors aggregate editing"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A selection-colors panel lists every distinct color/style used in a mixed selection and swaps each across all uses in one edit."
  primitive_domain: color
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360042553434-view-and-adjust-colors-in-a-mixed-selection"
  source_url: "https://help.figma.com/hc/en-us/articles/360042553434"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.typography] Typography

```yaml
records:
- id: "figma.deep.typography.core-properties"
  name: "Core text properties (family/weight/size)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text nodes set font family, weight/style, and size (density-independent pixels), with mixed values allowed across character ranges inside one text layer."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.line-height-letter-spacing"
  name: "Line height and letter spacing"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Line height is a fixed pixel value or percentage of font size; letter spacing (tracking) is numeric or percentage and can be negative."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.paragraph-spacing-indent"
  name: "Paragraph spacing and first-line indent"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Paragraph spacing sets distance between paragraphs and paragraph indentation offsets the first line, both per text node."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.alignment"
  name: "Horizontal + vertical text alignment"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text aligns horizontally left/center/right/justified and vertically top/middle/bottom within its box."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.auto-resize-modes"
  name: "Text auto-resize: auto width/auto height/fixed"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text boxes size by auto width (grow horizontally), auto height (wrap and grow vertically), or fixed size, switching automatically on manual resize."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.27378154668951-adjust-text-dimensions-and-resizing"
  source_url: "https://help.figma.com/hc/en-us/articles/27378154668951"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.truncation-max-lines"
  name: "Text truncation with max lines"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fixed/auto-height text can truncate with an ellipsis after a configurable maximum number of lines."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.27378154668951-adjust-text-dimensions-and-resizing"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.decoration-underline-options"
  name: "Underline styles (solid/dotted/wavy, thickness, offset, skip-ink)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Underline decoration supports solid, dotted, and wavy styles with configurable thickness, offset, and skip-ink behavior; strikethrough is a separate decoration."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.letter-case"
  name: "Letter case transforms incl. small caps"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Non-destructive case transforms render text as uppercase, lowercase, title case, or small caps without changing stored characters."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.vertical-trim"
  name: "Vertical trim (leading trim)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Vertical trim removes the extra space above cap height and below baseline so text boxes hug glyphs for precise spacing systems."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.hanging-lists-quotes"
  name: "Hanging lists and hanging quotes"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "List bullets and opening quotation marks can hang outside the text bounding box for optically aligned paragraphs."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956634-explore-text-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.numeric-opentype"
  name: "Numeric OpenType controls"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Numeric text options include fractions, superscript/subscript positioning, slashed zero, and number style choices (proportional/tabular, lining/old-style)."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.4913951097367-use-opentype-features"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956634"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.opentype-features"
  name: "OpenType feature exposure (stylistic sets, ligatures, letterforms)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Font-specific OpenType features - stylistic sets, alternates, ligatures, letterform variants - are individually toggleable in type details when the font declares them."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.4913951097367-use-opentype-features"
  source_url: "https://help.figma.com/hc/en-us/articles/4913951097367"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.variable-font-axes"
  name: "Variable font axis sliders"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Variable fonts expose their design axes (weight, width, optical size, slant, custom axes) as continuous sliders producing arbitrary instances."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.5579502031511-use-variable-fonts"
  source_url: "https://help.figma.com/hc/en-us/articles/5579502031511"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.lists"
  name: "Bulleted and numbered lists"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text supports bulleted and ordered lists with nesting levels, list-spacing control, and markdown-like shortcuts for list creation."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040449773-create-bulleted-and-numbered-lists"
  source_url: "https://help.figma.com/hc/en-us/articles/360040449773"
  source_ids: [DEEP-S06]
  verification_status: VERIFIED
- id: "figma.deep.typography.text-hyperlinks"
  name: "Hyperlinks on text ranges"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Character ranges accept hyperlinks to URLs or to frames/pages in the same file, clickable in prototypes and presentations."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360045942953-add-links-to-text"
  source_url: "https://help.figma.com/hc/en-us/articles/360045942953"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.text-styles"
  name: "Text styles with per-property overrides"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Named text styles capture the full typography property set, apply to whole layers or ranges, and update consumers on style edit; local overrides layer on top."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039957034-create-and-apply-text-styles"
  source_url: "https://help.figma.com/hc/en-us/articles/360039957034"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.typography-variables"
  name: "Variable binding to text properties and content"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Number variables bind to font size, line height, letter spacing, and paragraph spacing; string variables bind to text content and font family, enabling token-driven typography."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15343107263511-apply-variables-to-designs"
  source_url: "https://help.figma.com/hc/en-us/articles/15343107263511"
  source_ids: [DEEP-S07]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.font-loading"
  name: "Font sources: system, shared org fonts, local agent"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fonts come from a built-in library (provider-hosted Google Fonts set), locally installed fonts via desktop app or font agent, and org-shared uploaded fonts (provider-dependent sharing)."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039956894-add-a-font-to-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/360039956894"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.missing-fonts"
  name: "Missing font detection and replacement"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Files with unavailable fonts flag affected layers and offer a bulk replace-font dialog mapping missing families/styles to available ones."
  primitive_domain: typography
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.text-to-outlines"
  name: "Convert text to vector outlines"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text layers flatten into vector networks of glyph outlines for logo work and path editing, destructively losing text editability."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360047239073-convert-text-to-vector-paths"
  source_url: "https://help.figma.com/hc/en-us/articles/360047239073"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.text-on-path"
  name: "Text on path"
  record_role: "feature_deep_delta"
  source_product: figma_draw
  app_behavior: "Text can be attached to and flow along a vector path (TextPath object), keeping the text editable while following path geometry."
  primitive_domain: typography
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25, DEEP-S38]
  verification_status: VERIFIED
- id: "figma.deep.typography.font-picker-preview"
  name: "Font picker with previews and filters"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The font picker previews each family in its own glyphs, searches by name, and filters by source/classification for faster selection."
  primitive_domain: typography
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041308034-browse-and-apply-fonts"
  source_url: "https://help.figma.com/hc/en-us/articles/360041308034"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.typography.spell-check"
  name: "Spell check on text layers"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text editing underlines misspellings with correction suggestions per document language setting."
  primitive_domain: typography
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.components-and-variables] Components, Variants, Properties, Variables, Styles, Libraries

```yaml
records:
- id: "figma.deep.components-and-variables.component-creation"
  name: "Create component / multiple components from selection"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A selection converts into a single main component or into one component per selected top-level object in bulk."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360038663154-create-components-to-reuse-in-designs"
  source_url: "https://help.figma.com/hc/en-us/articles/360038663154"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.variant-sets"
  name: "Variant sets (component sets) with named properties"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Related components combine into a component set where each variant is addressed by property=value pairs (e.g. state=hover, size=large), switchable per instance."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360056440594-create-and-use-variants"
  source_url: "https://help.figma.com/hc/en-us/articles/360056440594"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.variant-conflict-detection"
  name: "Variant conflict/duplicate detection"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Component sets flag conflicting variants (identical property combinations) with error badges until property values are made unique."
  primitive_domain: diagnostics
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360056440594-create-and-use-variants"
  source_url: "https://help.figma.com/hc/en-us/articles/360056440594"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.property-boolean"
  name: "Boolean component property"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Boolean properties bind to layer visibility inside the component so instance consumers toggle sub-elements without variant explosion."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.5579474826519-explore-component-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/5579474826519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.property-text"
  name: "Text component property"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Text properties expose specific text layers' content as named instance-level fields editable from the instance panel."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.5579474826519-explore-component-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/5579474826519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.property-instance-swap"
  name: "Instance-swap property with preferred values"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Instance-swap properties expose a nested instance as a swappable slot, with an author-curated preferred-values list of allowed components."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.5579474826519-explore-component-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/5579474826519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.property-exposed-nested"
  name: "Exposed nested instance properties"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Nested instances inside a component can be marked exposed, surfacing their own properties on the outer instance's property panel."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.5579474826519-explore-component-properties"
  source_url: "https://help.figma.com/hc/en-us/articles/5579474826519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.slots"
  name: "Slots (structural placeholder children)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Slots mark regions of a component where consumers insert or replace arbitrary content per instance, complementing instance-swap for flexible composition."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.38231200344599-use-slots-to-build-flexible-components-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/38231200344599"
  source_ids: [DEEP-S25, DEEP-S47]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.override-model"
  name: "Instance override propagation model"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Instances inherit main-component changes except locally overridden properties (text, fills, effects, visibility, nested swaps); overrides persist across variant switches and component swaps when structure matches, and reset per-property or wholesale."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039150733-apply-changes-to-instances"
  source_url: "https://help.figma.com/hc/en-us/articles/360039150733"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.restore-main-component"
  name: "Restore deleted main component"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "When a main component is deleted, existing instances keep working and a restore command regenerates the main component from an instance."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360038665934-edit-main-components"
  source_url: "https://help.figma.com/hc/en-us/articles/360038665934"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.component-descriptions"
  name: "Component/style descriptions and doc links"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Components, styles, and variables carry descriptions and documentation links that surface in the assets panel, instance panel, and Dev Mode."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.7938814091287-add-descriptions-to-styles-components-and-variables"
  source_url: "https://help.figma.com/hc/en-us/articles/7938814091287"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.library-publishing"
  name: "Library publish with change review"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Publishing pushes selected components/styles/variables to a team library with a per-item change list and publish notes; consumers receive update review prompts (team library distribution is provider-dependent; the publish/subscribe model is a local concept)."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360025508373-publish-a-library"
  source_url: "https://help.figma.com/hc/en-us/articles/360025508373"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.library-update-review"
  name: "Accept/review incoming library updates"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Consuming files list pending library updates with previews and apply them selectively, keeping instance overrides intact."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039234193-review-and-accept-library-updates"
  source_url: "https://help.figma.com/hc/en-us/articles/360039234193"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.variable-type-color"
  name: "Color variables"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Color variables store color values bindable to fills and strokes, alias other color variables, and resolve per active mode."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-type-number"
  name: "Number variables"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Number variables bind to dimensions, min/max, gap, padding, corner radius, and typography numerics, and participate in prototype arithmetic expressions."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-type-string"
  name: "String variables"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "String variables bind to text content, font family names, and variant property values, and concatenate in prototype expressions."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-type-boolean"
  name: "Boolean variables"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Boolean variables bind to layer visibility and boolean component properties and drive prototype conditionals."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-collections-modes"
  name: "Collections with per-mode values"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Variables group into collections; each collection defines modes (e.g. light/dark, compact/comfortable) and every variable stores one value per mode, with mode count plan-gated (provider-dependent limit, local concept)."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.14506821864087-overview-of-variables-collections-and-modes"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-mode-inheritance"
  name: "Mode application and inheritance on frames"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A mode is set explicitly on any frame/section/page or inherited (auto) from ancestors, re-resolving all bound variables in that subtree."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15343816063383-modes-for-variables"
  source_url: "https://help.figma.com/hc/en-us/articles/15343816063383"
  source_ids: [DEEP-S07]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.variable-scoping"
  name: "Variable scoping to property surfaces"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Each variable's scope restricts which property pickers offer it (e.g. a color variable scoped to fills only), plus flags to hide from publishing."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-aliasing"
  name: "Variable aliasing chains"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Variables reference other variables as aliases (semantic token -> primitive token chains), resolving through modes at bind time."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.variable-code-syntax"
  name: "Per-platform code syntax on variables"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Variables store optional per-platform code syntax names (Web/Android/iOS) that Dev Mode and codegen emit instead of raw variable names."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15339657135383-guide-to-variables-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/15339657135383"
  source_ids: [DEEP-S07]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.extended-collections"
  name: "Extended variable collections (multi-brand)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A collection can extend another, inheriting its variables while overriding selected values, supporting multi-brand token systems."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.36346281624471-extend-a-variable-collection"
  source_url: "https://help.figma.com/hc/en-us/articles/36346281624471"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.color-styles"
  name: "Color (paint) styles"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Named paint styles capture full fill stacks (multiple paints incl. gradients/images) and apply to fills or strokes, distinct from single-value color variables."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15871097384471-the-difference-between-variables-and-styles"
  source_url: "https://help.figma.com/hc/en-us/articles/15871097384471"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.assets-panel-search"
  name: "Assets panel component browsing/search"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "An assets panel lists local and enabled-library components with text search, section grouping by library/page, and drag-to-insert instances."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039150173-create-and-insert-component-instances"
  source_url: "https://help.figma.com/hc/en-us/articles/360039150173"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.styles-vs-variables-contract"
  name: "Styles vs variables division of labor"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Styles bundle multi-property definitions (paint stacks, full typography, effect stacks, grids) while variables are single mode-aware values that can be consumed inside styles; both publish through libraries."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15871097384471-the-difference-between-variables-and-styles"
  source_url: "https://help.figma.com/hc/en-us/articles/15871097384471"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.components-and-variables.library-enable-per-file"
  name: "Per-file library enablement"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Each file toggles which published libraries are active in its assets panel, scoping the visible design-system surface per document."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041051154-guide-to-libraries-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/360041051154"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.slash-folder-naming"
  name: "Slash-path folder organization for styles/components"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Slash-separated names (e.g. Button/Primary/Large) group components and styles into nested picker folders without a separate folder entity."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360038663994-name-and-organize-components"
  source_url: "https://help.figma.com/hc/en-us/articles/360038663994"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.components-and-variables.instance-swap-picker"
  name: "Instance swap picker with override carry-over"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The instance panel swaps an instance for any other component via a searchable picker, carrying compatible overrides onto the new component."
  primitive_domain: component_system
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039150173-create-and-insert-component-instances"
  source_url: "https://help.figma.com/hc/en-us/articles/360039150173"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.prototyping] Prototyping: Triggers, Actions, Animation, Overlays, Flows

```yaml
records:
- id: "figma.deep.prototyping.trigger-click-tap"
  name: "Trigger: on click / on tap"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fires the interaction when the user clicks (desktop) or taps (touch) the hotspot object."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-drag"
  name: "Trigger: on drag"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fires while dragging the object in any direction with continuous movement mapping, typically paired with smart animate for swipeable UI."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-while-hovering"
  name: "Trigger: while hovering"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Shows the destination state while the cursor is over the hotspot and reverts on exit."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-while-pressing"
  name: "Trigger: while pressing"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Activates during click-and-hold or tap-and-hold and reverts when released."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-key-gamepad"
  name: "Trigger: keyboard/gamepad input"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fires on single keys, key combinations, or game controller buttons, enabling keyboard-driven and console-style prototypes."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-mouse-enter-leave"
  name: "Triggers: mouse enter / mouse leave"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Mouse-enter fires once when the cursor enters the hotspot area and mouse-leave when it exits, without auto-revert (unlike while-hovering)."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-mouse-down-up"
  name: "Triggers: mouse down / mouse up (touch down/up)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fire on press start and on release respectively, without auto-revert, for fine-grained press state machines."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-after-delay"
  name: "Trigger: after delay"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Frame-level timer trigger fires after a set dwell time on the frame, enabling splash screens, autoplay tours, and timed state machines."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.trigger-video-timestamps"
  name: "Triggers: when video hits / when video ends"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Video-bearing layers fire interactions at a specified playback timestamp or when playback ends."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035834-prototype-triggers"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035834"
  source_ids: [DEEP-S08]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-navigate-to"
  name: "Action: navigate to frame"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Replaces the current top-level frame with a destination frame, pushing onto prototype history."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-back"
  name: "Action: back"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Pops prototype navigation history to return to the previously shown frame."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-change-to"
  name: "Action: change to (variant switch)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Switches an instance (including nested instances) to another variant of its component set in place, the core of interactive components."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-scroll-to"
  name: "Action: scroll to object"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Scrolls the prototype viewport or a nested scroll container to a target object, instantly or animated with easing."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-open-link"
  name: "Action: open link"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Opens an external URL in a new tab from a prototype interaction."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-overlay-open-swap-close"
  name: "Actions: open/swap/close overlay"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Open overlay layers a frame above the current one, swap overlay replaces the active overlay without adding history, close overlay dismisses it."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-set-variable"
  name: "Action: set variable"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Writes a new value (literal, alias, or expression result) into a variable at runtime, re-resolving all bound properties."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-set-variable-mode"
  name: "Action: set variable mode"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Switches the active mode of a variable collection at runtime (e.g. light->dark) for a target scope."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15253268379799-variable-modes-in-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-conditional"
  name: "Action: conditional (if/else)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Conditional blocks evaluate variable-based boolean expressions and branch into different action lists, with multiple actions chainable on one trigger."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15253220891799-multiple-actions-and-conditionals"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.action-video-controls"
  name: "Actions: video playback control set"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Video actions play/pause/toggle playback, mute/unmute/toggle sound, seek to a specific time, and jump forward/backward by seconds."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040035874-prototype-actions"
  source_url: "https://help.figma.com/hc/en-us/articles/360040035874"
  source_ids: [DEEP-S09]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.expressions"
  name: "Runtime expressions (math, string, boolean)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Set-variable and conditional actions evaluate expressions over variables: arithmetic, string concatenation, comparisons, and boolean logic, including reading mode-specific values."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.15253194385943-use-expressions-in-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/15253194385943"
  source_ids: [DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.animation-instant-dissolve"
  name: "Animations: instant and dissolve"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Instant swaps frames with no transition; dissolve cross-fades with duration and easing."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040522373-prototype-animations"
  source_url: "https://help.figma.com/hc/en-us/articles/360040522373"
  source_ids: [DEEP-S10]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.animation-move-push-slide"
  name: "Animations: move in/out, push, slide in/out"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Directional transitions (left/right/up/down) move the destination over the origin, push the origin out, or slide with simultaneous dissolve, each with duration and easing."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040522373-prototype-animations"
  source_url: "https://help.figma.com/hc/en-us/articles/360040522373"
  source_ids: [DEEP-S10]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.smart-animate-matching"
  name: "Smart animate layer matching"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Smart animate pairs layers across origin/destination frames by name and hierarchy and tweens position, size, rotation, opacity, and fill differences; unmatched layers fade."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039818874-smart-animate-layers-between-frames"
  source_url: "https://help.figma.com/hc/en-us/articles/360039818874"
  source_ids: [DEEP-S10]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.easing-bezier-catalog"
  name: "Easing curve catalog incl. back variants"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Easing presets: linear, ease in, ease out, ease in and out, ease in back, ease out back, ease in and out back, plus a custom cubic bezier editor with four control points."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360051748654-prototype-easing-and-spring-animations"
  source_url: "https://help.figma.com/hc/en-us/articles/360051748654"
  source_ids: [DEEP-S11]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.spring-presets"
  name: "Spring presets (gentle/quick/bouncy/slow) + custom physics"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Physics-based spring animations offer gentle, quick, bouncy, and slow presets plus a custom spring parameterized by stiffness, damping, and mass."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360051748654-prototype-easing-and-spring-animations"
  source_url: "https://help.figma.com/hc/en-us/articles/360051748654"
  source_ids: [DEEP-S11]
  verification_status: VERIFIED
- id: "figma.deep.prototyping.overlay-positioning"
  name: "Overlay positioning and background settings"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Overlays position manually or via nine anchor presets, with optional background dim color and close-on-click-outside behavior."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039818254-create-overlays-in-your-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/360039818254"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.scroll-overflow"
  name: "Overflow scrolling (horizontal/vertical/both)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Frames declare overflow scrolling behavior - no scroll, horizontal, vertical, or both - creating nested scroll containers inside prototypes."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039818734-prototype-scroll-and-overflow-behavior"
  source_url: "https://help.figma.com/hc/en-us/articles/360039818734"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.sticky-scroll-position"
  name: "Fixed/sticky elements while scrolling"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Layers can be fixed relative to the viewport (stay while content scrolls) or sticky (stick to top when reached), reproducing app header/nav behavior."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039818734-prototype-scroll-and-overflow-behavior"
  source_url: "https://help.figma.com/hc/en-us/articles/360039818734"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.preserve-scroll"
  name: "Preserve scroll position across navigation"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Navigation interactions optionally keep the destination frame's scroll offset equal to the origin's, preserving continuity between screens."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360051747774-preserve-scroll-position-in-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/360051747774"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.flows-starting-points"
  name: "Flows and starting points"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Named flow starting points on frames define multiple entry paths per page; flows are listed in the sidebar and shareable as separate prototype links."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360039823894-create-and-manage-prototype-flows"
  source_url: "https://help.figma.com/hc/en-us/articles/360039823894"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.device-frames"
  name: "Prototype device frames and background"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Presentation settings wrap the prototype in a device chrome from a catalog (phones, tablets, desktops, watches, custom) with configurable background."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.21158597546391-set-prototype-device-and-background-settings"
  source_url: "https://help.figma.com/hc/en-us/articles/21158597546391"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.inline-preview"
  name: "Inline preview inside the editor"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A resizable preview panel plays the prototype inside the editor window without opening presentation view, updating live with edits."
  primitive_domain: prototype
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.presentation-view-options"
  name: "Presentation view options (fit/fill, hotspot hints, sidebar)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Presentation view offers zoom fit/fill/actual-size modes, optional hotspot hinting on misclick, flow sidebar, comments, and restart-flow controls."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040318013-play-your-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/360040318013"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.interactive-components"
  name: "Interactive components (default variant interactions)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Variant-to-variant interactions authored on a component set run inside every instance automatically, so hover/press/toggle states work without per-screen wiring."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360061175334-create-interactive-components-with-variants"
  source_url: "https://help.figma.com/hc/en-us/articles/360061175334"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.state-preservation"
  name: "Component state preservation across navigation"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Prototype settings choose whether interactive component states and variable values reset or persist when navigating between frames."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.14397859494295-state-management-for-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/14397859494295"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.connection-visualization"
  name: "Connection noodle visualization and bulk edit"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Prototype connections render as editable arrows (noodles) on canvas; selecting objects reveals their interactions for retargeting or deletion, with a view of all connections per page."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.4411431245335-view-prototype-connections"
  source_url: "https://help.figma.com/hc/en-us/articles/4411431245335"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.export-animations"
  name: "Export prototype animations to video/GIF"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Animated transitions/prototapes export as video or GIF assets for sharing outside the editor."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.41307983648407-export-animations-from-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/41307983648407"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.viewer-keyboard-navigation"
  name: "Presentation keyboard navigation"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Arrow keys and shortcuts step through flow frames, restart flows, and toggle UI chrome in presentation view."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040318013-play-your-prototypes"
  source_url: "https://help.figma.com/hc/en-us/articles/360040318013"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.mobile-device-viewing"
  name: "Prototype viewing on mobile devices"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Prototypes run full-screen on phones/tablets via the mobile app or browser with touch gesture triggers mapped natively (app distribution provider-dependent)."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040321093-view-prototypes-on-a-mobile-device"
  source_url: "https://help.figma.com/hc/en-us/articles/360040321093"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.prototyping.prototype-only-sharing"
  name: "Prototype-only share links"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Share links can expose only the running prototype without canvas/file access, keeping working files private during tests (link enforcement provider-dependent)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.dev-mode] Dev Mode, Code Connect, MCP Server

```yaml
records:
- id: "figma.deep.dev-mode.inspect-panel"
  name: "Inspect panel with code and list views"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Selecting a layer shows its properties as generated code or a structured property list, with layer name, type, and last-update metadata."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.code-snippet-languages"
  name: "Code snippets in multiple languages"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Built-in codegen emits CSS and iOS/Android platform snippets for the selection, extensible to other languages via codegen plugins."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.unit-settings"
  name: "Unit and root-size settings"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Measurement output switches between px and scaled units (e.g. rem with configurable root font size) affecting all inspect and code output."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.annotations"
  name: "Annotations (notes + property pills)"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Designers pin persistent annotations to layers combining free text with auto-updating property values (spacing, size, fill) that developers see in Dev Mode."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.measurements"
  name: "Persistent measurements"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Measurement objects between layers persist on the design (unlike hover red-lines) and stay in sync as geometry changes."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.dev-resources-links"
  name: "Dev resources (external links on nodes)"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Frames/components carry attached external resource links (GitHub, Jira, Storybook, docs) editable in-app and via REST API."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.ready-for-dev"
  name: "Ready-for-dev status marking"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Sections, frames, and components are markable ready-for-dev; a dedicated view filters marked designs and status changes emit notifications and webhook events."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.focus-view"
  name: "Focus view"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Focus view isolates one design at a time with full Dev Mode tooling, hiding surrounding canvas noise."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.compare-changes"
  name: "Compare changes between versions"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Developers diff a frame's current state against earlier versions side-by-side or overlaid to see what changed since last implementation."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.component-playground"
  name: "Component playground"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "A sandboxed playground lets developers flip a component's variants/properties to explore behavior without touching the file."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.instance-diff"
  name: "Instance vs main component comparison"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Dev Mode compares an instance to its main component and flags detached components so developers detect drift from the design system."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.asset-downloads"
  name: "Asset export from Dev Mode"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Icons, images, GIFs, and videos download at full resolution directly from the inspect surface without designer-authored export settings."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.variables-table"
  name: "Variables inspection + suggested variables"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Dev Mode shows applied variables with per-platform code syntax, a local collections table, and suggests matching variables for raw values."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.code-connect-concept"
  name: "Code Connect component-to-code mapping"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Code Connect replaces autogenerated snippets with the team's real component code and prop mappings in Dev Mode and MCP output (org/enterprise plan-gated, provider-dependent publishing; mapping model is a local concept)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/code-connect/"
  source_ids: [DEEP-S31]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.code-connect-frameworks"
  name: "Code Connect CLI frameworks + templates"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Code Connect CLI supports React/React Native, HTML-syntax frameworks (Web Components, Angular, Vue), SwiftUI, and Jetpack Compose, plus framework-agnostic template files; Code Connect UI maps one design component to many code implementations."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/code-connect/"
  source_ids: [DEEP-S31]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.vscode-extension"
  name: "VS Code extension"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "A VS Code extension embeds file inspection, notifications, and code suggestions from selected designs inside the IDE (provider-authenticated)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.mcp-server-deployment"
  name: "Dev Mode MCP server (desktop local + remote)"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "An MCP server exposes design context to AI coding agents, as a local server run by the desktop app or a hosted remote endpoint (remote hosting is provider-dependent; the MCP tool-contract surface is a local-mappable concept)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/32132100833559"
  source_ids: [DEEP-S13, DEEP-S14]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.mcp-get-design-context"
  name: "MCP tool: get_design_context / get_code"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Returns structured design context for the current selection or a node URL as framework-shaped code (React+Tailwind default, prompt-customizable)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/"
  source_ids: [DEEP-S14]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.mcp-get-variable-defs"
  name: "MCP tool: get_variable_defs"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Returns the variables and styles used within the selection (token names and values) for design-token-faithful codegen."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/"
  source_ids: [DEEP-S14]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.mcp-get-screenshot"
  name: "MCP tool: get_screenshot"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Renders a screenshot of the current selection so agents can visually verify implementations against the design."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/"
  source_ids: [DEEP-S14]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.mcp-code-connect-map"
  name: "MCP tool: get_code_connect_map"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Returns the mapping from selected design components to connected code components when Code Connect is configured."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/"
  source_ids: [DEEP-S14]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.mcp-canvas-writeback"
  name: "MCP canvas write-back (use_figma)"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "An MCP tool writes back to the canvas, creating or modifying design content from the AI client rather than only reading it."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/"
  source_ids: [DEEP-S14]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.plugins-in-dev-mode"
  name: "Dev Mode plugin surface (inspect capability)"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Plugins declaring the inspect capability run inside the Dev Mode inspect panel, and codegen-capability plugins add languages to the code dropdown."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.seat-gating"
  name: "Dev Mode seat gating"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Dev Mode requires a dev or full seat on paid plans (billing gate entirely provider-dependent; Studio's analog is a role-scoped inspect mode)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
- id: "figma.deep.dev-mode.layout-inspection-aids"
  name: "Layout aids: rulers, grids, outlines in Dev Mode"
  record_role: "feature_deep_delta"
  source_product: figma_dev_mode
  app_behavior: "Developers toggle layout guides, rulers, and layer outlines while inspecting, independent of designer view settings."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/15023124644247"
  source_ids: [DEEP-S12]
  verification_status: VERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.figjam] FigJam Whiteboard Objects and Facilitation

```yaml
records:
- id: "figma.deep.figjam.sticky-notes"
  name: "Sticky notes with author attribution"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Sticky notes are auto-sizing text cards with color options and an optional author-name footer for attribution during workshops."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.shapes-with-text"
  name: "Shapes with embedded text"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Diagram shapes (rectangles, circles, diamonds, etc.) carry inline editable text and resize around it, forming flowchart nodes."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.figjam.connectors"
  name: "Connectors with auto-routing"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Connector lines attach to object anchor magnets, reroute automatically as endpoints move, offer straight/elbow styles, arrowheads, and inline labels."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.figjam.marker-highlighter"
  name: "Marker and highlighter drawing"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Freehand marker and highlighter tools draw ink strokes with color/width choices tuned for whiteboarding rather than vector illustration."
  primitive_domain: vector
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: UNVERIFIED
- id: "figma.deep.figjam.washi-tape"
  name: "Washi tape decoration"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Washi tape strips are patterned decorative tape segments used to visually attach or decorate board content."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S15, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.figjam.stamps-emotes"
  name: "Stamps, emotes, and high-fives"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Participants drop stamp objects (dots, stars, hearts, plus-ones with avatar attribution) and fire transient emote animations including cursor high-fives."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.tables"
  name: "FigJam tables"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Table objects hold a cell grid with add/remove rows and columns, cell text, and cell fills for lightweight matrices."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S15, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.figjam.mindmaps"
  name: "Mind maps"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Mind map objects grow node trees via keyboard/plus-handle branching with automatic layout and rebalancing of sibling branches."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.sections"
  name: "Board sections"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Named sections group board content, collapse/expand, and act as navigation and voting targets during sessions."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.code-blocks"
  name: "Code blocks"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Code block objects display syntax-highlighted code snippets with language selection on the board."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.figjam.media-embeds"
  name: "Media, embeds, and link unfurls"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Boards host images, video/GIF media objects, iframe embeds of external tools, and unfurled link preview cards (external embed/unfurl content is provider-dependent)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S15, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.figjam.pages"
  name: "Pages in FigJam boards"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "A single FigJam file contains multiple pages/boards switchable from the sidebar for multi-exercise workshops."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.templates"
  name: "Template library + custom templates"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Boards start from a built-in template gallery and teams publish custom templates; quick-create shortcuts stamp common structures (community template distribution is provider-dependent)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.widgets"
  name: "Widgets on boards"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Interactive widget objects (polls, trackers, imports, games) built on the Widget API run inline on the board with synced multi-user state."
  primitive_domain: interactive
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/widgets/api/api-reference/"
  source_ids: [DEEP-S15, DEEP-S32]
  verification_status: VERIFIED
- id: "figma.deep.figjam.voting"
  name: "Voting sessions"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "A facilitator starts a voting session where participants place a limited number of anonymous votes on objects, with reveal and tally at session end."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.timer-music"
  name: "Timer with music"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "A shared countdown timer visible to all participants supports optional background music tracks during timed exercises."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.cursor-chat"
  name: "Cursor chat"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Pressing a shortcut attaches a transient chat bubble to the user's live cursor for lightweight in-canvas messaging."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.open-sessions"
  name: "Open sessions for external visitors"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Time-boxed open sessions let anonymous visitors join and edit a board without accounts (identity/access handling is provider-dependent)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.spotlight"
  name: "Spotlight facilitation"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "A presenter spotlights themselves to pull all participants' viewports along with their navigation until spotlight ends."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.jambot-ai"
  name: "Jambot and FigJam AI generation"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "AI features generate boards/diagrams from prompts, and the Jambot widget chains canvas content through LLM actions like summarize, ideate, and sort stickies (all provider-dependent AI)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.imports-competitors"
  name: "Imports: Mural, Lucid, Jamboard, spreadsheets"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Dedicated importers migrate Mural content, Lucid documents, and Google Jamboards into board objects, and spreadsheets/CSV import as stickies or tables (importer services are provider-dependent; resulting objects are local)."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.figjam.leaf.import-export"
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.exports"
  name: "Board export (PNG/PDF/CSV)"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Boards or selections export as images/PDF, and sticky-note content exports to CSV for downstream synthesis."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.figjam.leaf.import-export"
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: UNVERIFIED
- id: "figma.deep.figjam.stencils-quick-create"
  name: "Stencils and quick create"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Quick-create and stencil shortcuts stamp preconfigured object clusters (flows, grids of stickies) to accelerate board construction."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: VERIFIED
- id: "figma.deep.figjam.text-object"
  name: "Freestanding board text with formatting"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Board text objects support sizes, emphasis, lists, links, and color independent of stickies and shapes."
  primitive_domain: typography
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: UNVERIFIED
- id: "figma.deep.figjam.copy-to-design"
  name: "Copy board content into design files"
  record_role: "feature_deep_delta"
  source_product: figjam
  app_behavior: "Board objects copy/paste into design files (and design frames paste onto boards) with type conversion between whiteboard and design nodes."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.figjam.leaf.import-export"
  source_url: "https://help.figma.com/hc/en-us/categories/360002051633"
  source_ids: [DEEP-S15]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.slides-sites-buzz-make] Slides, Sites, Buzz, Make

```yaml
records:
- id: "figma.deep.slides.deck-object-model"
  name: "Slide deck object model (slides in ordered grid)"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "A deck is an ordered collection of fixed-size slide frames arranged in a canvas grid of rows, with dedicated slide/grid node types distinct from design frames."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.slides.grid-vs-slide-view"
  name: "Grid view vs single-slide view"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "The editor toggles between a bird's-eye grid of the whole deck and a focused single-slide view where notes and fine design edits happen."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17]
  verification_status: VERIFIED
- id: "figma.deep.slides.slide-panel-reorder"
  name: "Slide list reordering"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "A left-sidebar slide list drives ordering, insertion from template layouts, and navigation across the deck."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17]
  verification_status: VERIFIED
- id: "figma.deep.slides.templates-themes"
  name: "Deck templates and theme styles"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Decks apply templates with curated colors/fonts/layouts, mix layouts from multiple templates, restyle globally when the theme changes, and paid teams publish custom templates (template distribution provider-dependent)."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17]
  verification_status: VERIFIED
- id: "figma.deep.slides.toolbar-objects"
  name: "Slide content objects (text/shapes/media/tables)"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Slides take text with hyperlinks and slide-to-slide links, connector-capable shapes, images/videos with crop/border/overlay/blur, and tables."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17]
  verification_status: VERIFIED
- id: "figma.deep.slides.interactive-elements"
  name: "Live interactive elements (polls, alignment scales, prototypes)"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Slides embed live audience interactions - polls, alignment scales, and clickable embedded prototypes - that run during presentation (audience state sync is provider-dependent; object model is local)."
  primitive_domain: interactive
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17, DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.slides.presenter-notes"
  name: "Presenter notes"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Per-slide presenter notes are authored in slide view and shown in a presenter-only window during present-with-notes mode."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24245848829847"
  source_ids: [DEEP-S44]
  verification_status: VERIFIED
- id: "figma.deep.slides.present-modes"
  name: "Present modes and audience view"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Presenting opens an audience-view window with optional separate notes tab; spotlight, audio, and cursor chat run alongside for co-presentation (multi-user presence is provider-dependent)."
  primitive_domain: interactive
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24338209202327"
  source_ids: [DEEP-S18]
  verification_status: VERIFIED
- id: "figma.deep.slides.design-libraries"
  name: "Design libraries inside decks"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Decks consume components, styles, and variables from design libraries so brand systems flow into presentations."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/24170630629911"
  source_ids: [DEEP-S17]
  verification_status: VERIFIED
- id: "figma.deep.slides.export"
  name: "Deck export (PDF/PPTX)"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Decks export to PDF and PPTX for use outside the editor, alongside .deck local file copies as the compatibility target."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/24146015318551"
  source_ids: [DEEP-S49]
  verification_status: UNVERIFIED
- id: "figma.deep.sites.site-file-model"
  name: "Site file with pages and canvas editing"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "A site file holds webpages edited on a design-style canvas with the same layout primitives (auto layout, components) plus web-specific publishing semantics."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31230436657815"
  source_ids: [DEEP-S19]
  verification_status: VERIFIED
- id: "figma.deep.sites.blocks"
  name: "Blocks (prebuilt drag-in sections)"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Ready-made responsive blocks (heroes, galleries, footers) and embed blocks (video, maps) drag onto pages to assemble layouts quickly."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31242773374615"
  source_ids: [DEEP-S45]
  verification_status: VERIFIED
- id: "figma.deep.sites.breakpoints"
  name: "Primary/secondary responsive breakpoints"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Pages define breakpoints at screen widths; the primary (desktop by default) cascades changes to secondary breakpoints, which hold per-width overrides."
  primitive_domain: layout
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31242797809815"
  source_ids: [DEEP-S21]
  verification_status: VERIFIED
- id: "figma.deep.sites.interactions-presets"
  name: "Site interaction presets"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "An interaction panel attaches animations, transitions, and transforms (hover/press/scroll-triggered effects like marquee and parallax) to page content without code."
  primitive_domain: interactive
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/35895820755095"
  source_ids: [DEEP-S46]
  verification_status: VERIFIED
- id: "figma.deep.sites.code-layers"
  name: "Code layers (AI-generated or hand-written React)"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Code layers embed custom functionality authored by describing behavior to AI or editing React source directly in a code editor (AI generation provider-dependent; code artifact local)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31242824165143"
  source_ids: [DEEP-S20]
  verification_status: VERIFIED
- id: "figma.deep.sites.cms"
  name: "CMS content collections"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "A CMS defines structured content collections bound into page layouts so non-designers manage content separately from design (hosted CMS storage provider-dependent; schema/binding model local)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31230436657815"
  source_ids: [DEEP-S19]
  verification_status: VERIFIED
- id: "figma.deep.sites.preview"
  name: "Site preview across breakpoints"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "A preview mode renders the page with live interactions and resizable viewport to test breakpoint behavior before publishing."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/35895705647127"
  source_ids: [DEEP-S19]
  verification_status: VERIFIED
- id: "figma.deep.sites.publish-hosting"
  name: "Publish, hosting, custom domains"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Sites publish to vendor hosting with a generated URL, republish on change, and connect custom domains (hosting/domains fully provider-dependent; Studio maps to local static export/deploy adapters)."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31230436657815"
  source_ids: [DEEP-S19]
  verification_status: VERIFIED
- id: "figma.deep.sites.design-file-insert"
  name: "Insert design-file content and libraries into sites"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Existing design frames and design-system libraries insert into site pages, carrying components/styles into the web layout."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31242773374615"
  source_ids: [DEEP-S45]
  verification_status: VERIFIED
- id: "figma.deep.buzz.asset-templates"
  name: "Brand asset templates with guidelines"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "Design teams publish asset templates whose guidelines lock structure/brand elements (pink outline + lock badge) while exposing editable fields to marketers."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31271566667543"
  source_ids: [DEEP-S22, DEEP-S24]
  verification_status: VERIFIED
- id: "figma.deep.buzz.simplified-editor"
  name: "Simplified constrained editor"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "The editing surface restricts non-designers to swapping text, images, and colors within template constraints; locked objects are unselectable."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31271589645079"
  source_ids: [DEEP-S22]
  verification_status: VERIFIED
- id: "figma.deep.buzz.bulk-create"
  name: "Bulk create from CSV/XLSX"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "Spreadsheet import maps columns onto named template fields and generates one asset per row for campaign-scale production; locked fields must be unlocked to participate."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31271824185623"
  source_ids: [DEEP-S23]
  verification_status: VERIFIED
- id: "figma.deep.buzz.ai-image-tools"
  name: "Buzz AI image and copy tools"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "Make-an-image generation/editing (gpt-image-1), resolution boost, background removal, copy tone adjustment, translation, and shortening operate inside asset editing (all provider-dependent AI)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31271566667543"
  source_ids: [DEEP-S22]
  verification_status: VERIFIED
- id: "figma.deep.buzz.brand-controls"
  name: "Team brand controls"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "Admins define team-level brand asset sources (templates, logos, colors, fonts) that scope what Buzz users can apply (org distribution provider-dependent; control model local)."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/33155043230487"
  source_ids: [DEEP-S24]
  verification_status: VERIFIED
- id: "figma.deep.buzz.multi-size-export"
  name: "Multi-size asset generation and export"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "Assets produce channel-sized variants (social formats) and export as image/video files for campaign delivery."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31271589645079"
  source_ids: [DEEP-S22]
  verification_status: UNVERIFIED
- id: "figma.deep.make.prompt-to-app"
  name: "Prompt-to-app chat generation"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "A chat interface generates working web apps/prototypes from natural-language prompts, iterating through follow-up messages with chat context management (LLM generation provider-dependent)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.plan-mode"
  name: "Plan mode before generation"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Plan mode drafts an implementation plan for review before the agent writes code, steering scope ahead of generation (provider-dependent)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.code-editing"
  name: "Direct code editing of generated output"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Generated app source is fully viewable and hand-editable in a built-in code editor alongside the AI chat, with live preview."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.design-import"
  name: "Attach design files/frames as generation context"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Design frames attach to prompts so generated apps match existing designs, including design-system package guidance via make kits."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.backends-integrations"
  name: "Backend connections (Supabase) and web search"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Prototypes connect to hosted backends (e.g. Supabase) for auth/data and can search the web during generation (all provider-dependent integrations)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.github-push"
  name: "Push generated code to GitHub"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Make projects push their codebase to GitHub repositories for continued development outside the tool (provider-dependent integration)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.publish-hosting"
  name: "Publish Make apps to hosted URLs"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Finished apps publish to vendor-hosted URLs, update in place, embed in decks, and share to community (hosting and community distribution provider-dependent)."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.templates-remix"
  name: "Make templates and remixing"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Users start from templates or remix existing published prototypes/web apps as new editable projects."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.make.model-selection-credits"
  name: "AI model selection and credit budget"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Users choose among available AI models and consume plan-based AI credits per generation (entirely provider-dependent billing/model surface)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: VERIFIED
- id: "figma.deep.slides.slide-transitions"
  name: "Per-slide transition animations"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Slides define entrance/advance transitions (including smart-animate-style matching) applied when presenting."
  primitive_domain: interactive
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/24146015318551"
  source_ids: [DEEP-S49]
  verification_status: UNVERIFIED
- id: "figma.deep.slides.skip-hide-slides"
  name: "Skip/hide slides from presentation"
  record_role: "feature_deep_delta"
  source_product: figma_slides
  app_behavior: "Individual slides can be excluded from the presented sequence while remaining editable in the deck."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/24146015318551"
  source_ids: [DEEP-S49]
  verification_status: UNVERIFIED
- id: "figma.deep.sites.seo-page-settings"
  name: "Per-page SEO/meta settings"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Pages carry title, description, and social preview metadata emitted into published HTML (publishing provider-dependent; metadata model local)."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31823555275671"
  source_ids: [DEEP-S19]
  verification_status: UNVERIFIED
- id: "figma.deep.sites.republish-unpublish"
  name: "Republish updates and unpublish"
  record_role: "feature_deep_delta"
  source_product: figma_sites
  app_behavior: "Published sites update by republishing changed pages and can be taken offline; org admins can gate web publishing (provider-dependent)."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31230436657815"
  source_ids: [DEEP-S19]
  verification_status: VERIFIED
- id: "figma.deep.buzz.local-file-copy"
  name: ".buzz local file copy"
  record_role: "feature_deep_delta"
  source_product: figma_buzz
  app_behavior: "Buzz files save/load as .buzz local copies, a Studio import/export compatibility target."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31271566667543"
  source_ids: [DEEP-S22]
  verification_status: UNVERIFIED
- id: "figma.deep.make.local-file-copy"
  name: ".make local file copy"
  record_role: "feature_deep_delta"
  source_product: figma_make
  app_behavior: "Make projects save/load as .make local copies containing prompt history and code, a Studio import/export compatibility target."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/categories/31304285531543"
  source_ids: [DEEP-S16]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.collaboration-and-files] Multiplayer, Comments, Versions, File Organization

```yaml
records:
- id: "figma.deep.collaboration-and-files.multiplayer-presence"
  name: "Live multiplayer editing with presence cursors"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "All open collaborators edit the same document concurrently with named live cursors, per-user selection highlights, and avatar stack (vendor sync is cloud-hosted, provider-dependent; Handshake maps this onto its local-first CRDT layer)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.observation-mode"
  name: "Observation mode (follow a collaborator)"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Clicking a collaborator's avatar locks the local viewport to follow their navigation until the observer interacts, used for demos and reviews."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.spotlight-me"
  name: "Spotlight my cursor across products"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "A presenter forces all participants into follow mode via spotlight, available across design, whiteboard, and slides surfaces."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.audio-huddles"
  name: "In-file audio conversations"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Collaborators join an in-file voice channel while working (audio infrastructure fully provider-dependent; Studio would map to an optional adapter)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.comment-threads"
  name: "Comment threads with mentions and attachments"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Comments pin to canvas points or regions, thread replies, support @mentions and emoji reactions, and can be resolved/reopened with notification fan-out (notification delivery provider-dependent)."
  primitive_domain: collaboration
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041068574-add-comments-to-files"
  source_url: "https://help.figma.com/hc/en-us/articles/360041068574"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.comment-filtering"
  name: "Comment list filtering/sorting"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "The comment sidebar filters by resolved/unresolved, mentions-only, and sorts by date/position for triage."
  primitive_domain: collaboration
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360041547593-view-and-manage-comments"
  source_url: "https://help.figma.com/hc/en-us/articles/360041547593"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.version-checkpoints"
  name: "Autosave checkpoints every 30 minutes"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "The version system records an autosave checkpoint roughly every 30 minutes of editing plus on-demand named versions with descriptions."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360038006754"
  source_ids: [DEEP-S39]
  verification_status: VERIFIED
- id: "figma.deep.collaboration-and-files.version-restore"
  name: "Version restore with dual checkpoints"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Restoring a version adds two checkpoints - the pre-restore current state and the restored state - so restores are themselves reversible."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360038006754"
  source_ids: [DEEP-S39]
  verification_status: VERIFIED
- id: "figma.deep.collaboration-and-files.version-duplicate"
  name: "Duplicate a version to a new file"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Any historical version duplicates into a standalone file, e.g. to freeze a handoff snapshot."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360038006754"
  source_ids: [DEEP-S39]
  verification_status: VERIFIED
- id: "figma.deep.collaboration-and-files.version-retention"
  name: "Version retention window by plan"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Free-tier files keep 30 days of version history while paid plans keep full history (retention policy provider-dependent; Studio local storage has no such limit)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360038006754"
  source_ids: [DEEP-S39]
  verification_status: VERIFIED
- id: "figma.deep.collaboration-and-files.branch-model"
  name: "Branching with review and merge"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Branches fork a file for isolated exploration, request reviews, diff changes against main, merge back with conflict handling, and pull updates from main (branch storage provider-dependent; branch/merge semantics local-mappable)."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360063144053-guide-to-branching"
  source_url: "https://help.figma.com/hc/en-us/articles/360063144053"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.file-hierarchy"
  name: "Teams / projects / files / drafts hierarchy"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Files organize under projects within teams, with a personal drafts space; moving between spaces changes sharing defaults (org structure provider-dependent; hierarchy concept local)."
  primitive_domain: document
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040328273-plans-and-teams-in-figma"
  source_url: "https://help.figma.com/hc/en-us/articles/360040328273"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.file-browser-search"
  name: "File browser with search and recents"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "A file browser lists recents, favorites, and project trees with cross-file search over names and content (search index provider-dependent in vendor; local index in Studio)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.trash-restore"
  name: "Deleted file trash and restore"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Deleted files sit in a recoverable trash before permanent deletion (retention rules provider-dependent)."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.sharing-link-permissions"
  name: "Share links with role scoping"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Share links grant view/edit/dev-mode roles to anyone-with-link, password-protected, or invited users, with separate prototype-only links (identity and enforcement provider-dependent; permission model is the local-relevant concept)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.community-publish"
  name: "Community publish and duplicate posture"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Files, templates, plugins, and widgets publish to a public community where users duplicate them into their own space (marketplace fully provider-dependent; duplication produces local documents)."
  primitive_domain: collaboration
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.community.leaf.category"
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.local-file-copies"
  name: "Local copies: .fig/.jam/.deck save and open"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Files save to local .fig (design), .jam (whiteboard), .deck (slides) archives and reimport by drag-in, the primary offline interchange and Studio compatibility target."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.8403626871063-save-a-local-copy-of-files"
  source_url: "https://help.figma.com/hc/en-us/articles/8403626871063"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.offline-editing"
  name: "Offline editing with reconnect sync"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Open files remain editable offline and replay queued changes on reconnect (vendor sync provider-dependent; offline-capable document core is the local-relevant behavior)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040328553"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.notifications-center"
  name: "Notification center (mentions/comments/updates)"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "A notifications inbox aggregates mentions, comment replies, review requests, and library updates with email mirroring (delivery provider-dependent; the event feed is local-mappable)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.favorites"
  name: "Favorite/starred files"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Users star files/projects into a personal favorites list in the file browser sidebar."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.move-duplicate-files"
  name: "Move and duplicate files across projects"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Files move between projects/teams via drag or dialog and duplicate in place, with permission implications surfaced on move."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.collaboration-and-files.ask-to-edit"
  name: "Ask-to-edit access requests"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Viewers request edit access from a file's owner in-context, generating an approval workflow (identity/approval provider-dependent; request-grant model local-relevant)."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.plugin-api] Plugin API Surface

```yaml
records:
- id: "figma.deep.plugin-api.node-document-page"
  name: "DocumentNode and PageNode containers"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "The scene graph roots at DocumentNode containing PageNodes; all other nodes are scene nodes under pages, with dynamic page loading in current manifests."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-frame"
  name: "FrameNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "FrameNode exposes container geometry, fills/strokes/effects, clipsContent, layoutMode and full auto layout properties, layout grids, and children management."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-group-section"
  name: "GroupNode and SectionNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "GroupNode is a child-derived bounds wrapper without own layout; SectionNode is a named canvas organizer with fill and dev-status metadata."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-component-set"
  name: "ComponentNode and ComponentSetNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "ComponentNode is a publishable main component with componentPropertyDefinitions; ComponentSetNode wraps variant children keyed by property=value names."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-instance"
  name: "InstanceNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "InstanceNode links to its main component, exposes componentProperties and overrides, supports swapComponent and detachInstance."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-slot"
  name: "SlotNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "SlotNode represents slot placeholders inside components for per-instance content insertion, mirroring the slots product feature."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-boolean-operation"
  name: "BooleanOperationNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "BooleanOperationNode holds a booleanOperation type (UNION/INTERSECT/SUBTRACT/EXCLUDE) over live children."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-vector"
  name: "VectorNode with vectorNetwork/vectorPaths"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "VectorNode exposes geometry both as vectorNetwork (vertices, segments, regions with fills) and as vectorPaths (SVG-like path data with winding rules), both writable."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-text"
  name: "TextNode with range-level styling"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "TextNode reads/writes characters plus per-range fonts, sizes, fills, decorations, hyperlinks, list options, and OpenType settings via getRange/setRange APIs after loadFontAsync."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-text-path"
  name: "TextPathNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "TextPathNode models text flowed along a vector path as a distinct scene node type."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-geometry-primitives"
  name: "Rectangle/Ellipse/Line/Polygon/Star nodes"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Parametric shape nodes expose their shape parameters (cornerRadius per corner, arcData, pointCount, innerRadius) plus shared geometry/paint mixins."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-slice"
  name: "SliceNode export regions"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "SliceNode defines an invisible export rectangle with its own exportSettings, decoupling export regions from visible layers."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-figjam-set"
  name: "FigJam node set (Sticky/Connector/ShapeWithText/Table/CodeBlock/Stamp/Highlight/WashiTape)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Whiteboard object types are first-class scene nodes with their own APIs (sticky text/author, connector endpoints and magnets, table cells), editable by plugins in the figjam editorType."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-media-embed"
  name: "MediaNode, EmbedNode, LinkUnfurlNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Media (video/GIF), iframe embeds, and unfurled link cards are addressable scene nodes with source metadata."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-slides-set"
  name: "Slides node set (SlideNode/SlideRowNode/SlideGridNode/InteractiveSlideElementNode)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Slide decks expose slides, their row/grid arrangement, and interactive slide elements (polls/scales) as dedicated node types for the slides editorType."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-transform-group"
  name: "TransformGroupNode"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "TransformGroupNode wraps children under a shared transform, a newer grouping primitive distinct from plain groups."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.global-figma-object"
  name: "Global figma object (editorType, mode, root, currentPage)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins run against a sandboxed global exposing document root, currentPage, editorType (figma/figjam/dev/slides/buzz), run mode (default/textreview/inspect/codegen/linkpreview/auth), notify, and closePlugin."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.developers"
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.figma-ui-iframe"
  name: "Plugin UI iframe + postMessage bridge"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugin UI renders arbitrary HTML in a sandboxed iframe (figma.showUI) that exchanges messages with the main-thread sandbox via figma.ui.postMessage/onmessage; main thread has no DOM access."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.figma-viewport"
  name: "figma.viewport control"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins read and set viewport center/zoom and scroll-and-zoom-into-view on node sets."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.client-storage"
  name: "figma.clientStorage persistence"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Async key-value storage persists plugin data on the local client, separate from document data (setPluginData) which travels with the file."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.variables-api"
  name: "figma.variables plugin API"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins create/read variables and collections, bind variables to node properties, and resolve values per mode programmatically."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26, DEEP-S07]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.team-library-import"
  name: "Team library access + import by key"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "figma.teamLibrary enumerates available library variable collections, and importComponentByKeyAsync/importStyleByKeyAsync pull published assets into the file (library backend provider-dependent)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.codegen-api"
  name: "figma.codegen for Dev Mode codegen plugins"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Codegen plugins register languages and return code blocks per selection via figma.codegen.on('generate'), with user preferences for units and custom settings."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26, DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.node-factories"
  name: "Node factory methods"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "figma.create* factories mint rectangles, lines, ellipses, polygons, stars, vectors, text, frames, components, pages, slices, stickies, connectors, tables, images (createImage/Async), videos, and nodes from JSX."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.events"
  name: "Plugin event subscription"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins subscribe to run, selectionchange (via currentPage), documentchange, canvasviewchange, textreview, stylechange, and drop events with on/once/off."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.export-async"
  name: "Node exportAsync rendering"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Any node renders to PNG/JPG/SVG/PDF bytes via exportAsync with export settings, giving plugins the same render pipeline as manual export."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.manifest-editor-types"
  name: "Manifest editorType targeting"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "A plugin manifest declares which editors it runs in - figma, figjam, dev, slides, buzz - with documentAccess dynamic-page required for new plugins."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.manifest-network-permissions"
  name: "Manifest networkAccess and permissions"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Manifests whitelist network domains (allowedDomains + reasoning) and request permissions: currentuser, activeusers, fileusers, payments, teamlibrary."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.manifest-capabilities"
  name: "Manifest capabilities (textreview/codegen/inspect/vscode)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Capability flags opt plugins into text review pipelines, Dev Mode codegen (with codegenLanguages/preferences), inspect-panel embedding, and VS Code contexts."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.parameters-quick-run"
  name: "Parameter-driven quick-run plugins"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins declare typed input parameters gathered through the quick-actions bar (with suggestions and freeform flags) before or instead of opening UI."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.relaunch-buttons"
  name: "Relaunch buttons on nodes"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "setRelaunchData pins plugin re-run buttons with descriptions onto specific nodes so documents advertise plugin actions contextually."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S27]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.widget-model"
  name: "Widget API model (declarative components + synced state)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Widgets are declarative component trees (AutoLayout, Frame, Text, SVG, Image, Input) whose state lives in useSyncedState/useSyncedMap (last-writer-wins per key) with usePropertyMenu for config and click handlers for interactivity, running identically for all collaborators."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/widgets/api/api-reference/"
  source_ids: [DEEP-S32, DEEP-S50]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.dev-tooling"
  name: "Plugin dev tooling (typings, hot reload, console)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugin development uses published TypeScript typings, a manifest build hook, hot reload in the desktop app, and the editor's developer console for debugging."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/"
  source_ids: [DEEP-S27]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.private-plugins"
  name: "Private org plugin/widget distribution"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Organizations publish plugins/widgets privately to their members outside the public community (distribution provider-dependent; a local plugin registry is the Studio analog)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/"
  source_ids: [DEEP-S27]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.plugin-data"
  name: "Node plugin data (private + shared)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "setPluginData/getPluginData store per-node key-value strings namespaced to the plugin, with sharedPluginData namespaces readable across plugins; data travels with the document."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.traversal-helpers"
  name: "Tree traversal helpers (findAll/findOne/findAllWithCriteria)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Container nodes expose findAll/findOne/findChild/findAllWithCriteria for filtered subtree queries, with skipInvisibleInstanceChildren tuning performance."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25, DEEP-S26]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.local-styles-api"
  name: "Local styles API (create/list paint/text/effect/grid styles)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins enumerate and create local paint, text, effect, and grid styles and assign style ids to node properties."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.font-listing"
  name: "Font enumeration and loading"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "listAvailableFontsAsync enumerates all loadable fonts and loadFontAsync must precede text mutations using a font."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.user-apis"
  name: "figma.currentUser and activeUsers"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "With permissions, plugins read the current user's id/name/color and the list of active users in the file for presence-aware tooling."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/manifest/"
  source_ids: [DEEP-S26, DEEP-S27]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.payments-api"
  name: "figma.payments plugin monetization"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "The payments API gates paid plugin features via checkout status queries (marketplace billing entirely provider-dependent)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: VERIFIED
- id: "figma.deep.plugin-api.timer-api"
  name: "figma.timer shared timer control"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Plugins start/pause/stop the shared board timer programmatically in whiteboard contexts."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/figma/"
  source_ids: [DEEP-S26]
  verification_status: UNVERIFIED
- id: "figma.deep.plugin-api.get-css-async"
  name: "Node getCSSAsync extraction"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Nodes emit their computed CSS property map via getCSSAsync, matching Dev Mode's CSS output for programmatic consumers."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/plugins/api/nodes/"
  source_ids: [DEEP-S25]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.rest-api-and-platform] REST API, Webhooks, Embeds, Formats, Export

```yaml
records:
- id: "figma.deep.rest-api-and-platform.files-endpoint"
  name: "Files endpoint (document JSON tree)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "GET file returns the full document as a JSON node tree with geometry, styles, and component metadata; depth and geometry query params trim the payload (hosted API provider-dependent; the serialized node-tree schema is a Studio compatibility/reference target)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.developers"
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.file-nodes-endpoint"
  name: "File nodes endpoint (subtree fetch)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "GET file nodes returns only requested node IDs and their subtrees for partial reads of large files."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.images-render-endpoint"
  name: "Images render endpoint"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "GET images renders nodes server-side to PNG/JPG/SVG/PDF at a requested scale and returns temporary URLs; a companion endpoint lists image fill sources."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.comments-endpoints"
  name: "Comments + reactions endpoints"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Comment endpoints list/create/delete comments anchored to canvas coordinates or nodes, with separate comment-reaction endpoints."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.versions-endpoint"
  name: "File versions endpoint"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "GET versions lists a file's version history entries (id, label, description, user, timestamp) usable as version pins on other endpoints."
  primitive_domain: document
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.projects-endpoints"
  name: "Teams/projects/files listing endpoints"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Endpoints enumerate a team's projects and each project's files for workspace crawling (org structure provider-dependent)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.components-styles-endpoints"
  name: "Components/component-sets/styles endpoints"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Published team components, component sets, and styles are listable and fetchable by key, powering external design-system indexes."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.variables-endpoints"
  name: "Variables REST endpoints (enterprise)"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Variables endpoints query local/published variables and bulk create/update/delete variables and collections; write access is enterprise plan-gated (provider-dependent gate; token CRUD is a local concept)."
  primitive_domain: component_system
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28, DEEP-S30]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.webhooks-v2"
  name: "Webhooks v2 CRUD and event catalog"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Webhooks register per team/project/file contexts with passcodes and deliver events: FILE_UPDATE, FILE_VERSION_UPDATE, FILE_DELETE, LIBRARY_PUBLISH, FILE_COMMENT, DEV_MODE_STATUS_UPDATE (hosted delivery provider-dependent; the event taxonomy maps to local event-ledger triggers)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/webhooks/"
  source_ids: [DEEP-S29]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.dev-resources-endpoints"
  name: "Dev resources endpoints"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Dev resource endpoints CRUD external links attached to nodes so integrations sync tickets/repos into Dev Mode."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.analytics-endpoints"
  name: "Library analytics + activity log endpoints"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Endpoints expose library usage analytics and org activity logs for reporting (enterprise, provider-dependent; Studio analog is local usage diagnostics)."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28, DEEP-S30]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.auth-tokens"
  name: "Auth: personal access tokens + OAuth2"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "API access authenticates via personal access tokens or OAuth2 apps with granular scopes (file_content:read, file_comments:read/write, file_variables:read/write, file_dev_resources, webhooks, library_analytics:read and others), replacing the deprecated broad files:read scope (auth service provider-dependent; scope taxonomy is a local capability-gate reference)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/scopes/"
  source_ids: [DEEP-S30]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.embed-kit"
  name: "Embed Kit 2.0 + Embed API"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Files/prototypes embed in external sites via iframe with registered origins and client ID; the Embed API exchanges postMessage commands (NAVIGATE_FORWARD/BACKWARD, RESTART) and emits prototype state events (embed hosting provider-dependent)."
  primitive_domain: interactive
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/embeds/embed-kit-2.0/"
  source_ids: [DEEP-S33, DEEP-S34]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.fig-format-posture"
  name: ".fig binary format posture (undocumented)"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "The .fig local-copy format is an undocumented binary (compressed kiwi-schema message plus assets); Studio treats it strictly as an import/export compatibility target requiring fixtures and unsupported-feature receipts, never as an internal format."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/8403626871063"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.sketch-import"
  name: "Sketch file import"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: ".sketch files import as editable documents converting artboards, symbols, and styles with documented fidelity limits."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040514273-import-sketch-files"
  source_url: "https://help.figma.com/hc/en-us/articles/360040514273"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.export-settings-model"
  name: "Per-layer export settings model"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Layers carry stacked export settings, each with format (PNG/JPG/SVG/PDF), scale or width/height constraint, and filename suffix, batch-exportable from an export list."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.13402894554519-export-formats-and-settings"
  source_url: "https://help.figma.com/hc/en-us/articles/13402894554519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.svg-export-options"
  name: "SVG export options"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "SVG export toggles include-id attributes, outline-text (vs embedded font references), and simplify-stroke behavior affecting fidelity of round-trips."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.13402894554519-export-formats-and-settings"
  source_url: "https://help.figma.com/hc/en-us/articles/13402894554519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.pdf-export"
  name: "PDF export of frames and slides"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Frames export to single or combined multi-page PDFs (used for decks and print handoffs) with raster/vector content preserved where possible."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.13402894554519-export-formats-and-settings"
  source_url: "https://help.figma.com/hc/en-us/articles/13402894554519"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.copy-as"
  name: "Copy as CSS/SVG/PNG"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Context-menu copy-as commands put CSS properties, SVG markup, or rasterized PNG of the selection on the clipboard for cross-tool paste."
  primitive_domain: export
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.360040030374-copy-assets-between-design-tools"
  source_url: "https://help.figma.com/hc/en-us/articles/360040030374"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.rate-limits"
  name: "API rate limiting posture"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "REST endpoints enforce per-token rate limits with 429 responses and retry-after guidance (limits provider-dependent and plan-tiered)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.image-fills-endpoint"
  name: "Image fills download endpoint"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "A dedicated endpoint returns download URLs for all image fills present in a document, keyed by imageRef."
  primitive_domain: export
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.file-metadata-endpoint"
  name: "File metadata endpoint"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "A lightweight meta endpoint returns file name, editor type, thumbnail, last-modified, and touch metadata without the node tree."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: UNVERIFIED
- id: "figma.deep.rest-api-and-platform.users-me-endpoint"
  name: "Users/me identity endpoint"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "GET me returns the authenticated user's id, handle, email, and avatar for token introspection."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
- id: "figma.deep.rest-api-and-platform.payments-endpoint"
  name: "Payments API for resource monetization"
  record_role: "feature_deep_delta"
  source_product: figma_api
  app_behavior: "Payments endpoints let published plugin/widget developers query purchase state for users (marketplace billing entirely provider-dependent)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://developers.figma.com/docs/rest-api/"
  source_ids: [DEEP-S28]
  verification_status: VERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.ai-features] AI Feature Surface (All Provider-Dependent)

```yaml
records:
- id: "figma.deep.ai-features.first-draft"
  name: "First Draft (prompt-to-design)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Generates editable design mockups from text prompts using library-based or wireframe styles (provider-dependent AI; output is ordinary local document content)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.semantic-asset-search"
  name: "AI asset/design search (text + image query)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Search finds designs and components from a description, a screenshot, or part of a design via semantic/visual matching (provider-dependent indexing)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.replace-content"
  name: "Replace placeholder content"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Fills selected layers with realistic generated text/data replacing lorem-ipsum placeholders (provider-dependent)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.add-interactions"
  name: "AI prototype wiring (add interactions)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Automatically connects frames into an interactive prototype by inferring navigation from the designs (provider-dependent)."
  primitive_domain: prototype
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.rename-layers"
  name: "AI rename layers"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Batch-renames selected layers with semantically meaningful names inferred from content (provider-dependent)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.text-rewrite-translate"
  name: "Rewrite/translate/shorten text"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Adjusts tone, translates language, or shortens copy of selected text layers in place (provider-dependent)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.make-edit-image"
  name: "Make and edit images"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Generates new images or edits existing ones from text prompts inside the canvas (provider-dependent image models)."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.remove-background"
  name: "Remove background"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Isolates image subjects by deleting the background of an image fill in one action (provider-dependent; local segmentation is the Studio analog)."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.boost-resolution"
  name: "Boost image resolution"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Upscales low-resolution images to sharper versions (provider-dependent; local super-resolution is the Studio analog)."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.expand-image"
  name: "Expand image (outpainting)"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Extends image content beyond original borders generatively (provider-dependent)."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.isolate-erase"
  name: "Isolate and erase objects"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Marks image regions to isolate as separate elements or erase with content fill (provider-dependent)."
  primitive_domain: raster
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.vectorize-image"
  name: "Vectorize raster images"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "Converts raster images into editable vector layers (provider-dependent service; local raster-to-vector tracing is the Studio analog)."
  primitive_domain: vector
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.23870272542231-use-ai-tools-in-figma-design"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.figma-agent"
  name: "Figma Agent in design files"
  record_role: "feature_deep_delta"
  source_product: figma_design
  app_behavior: "A conversational agent performs multi-step design edits and answers inside design files (beta rollout from May 2026; fully provider-dependent; Studio analog is its model-tool-contract surface)."
  primitive_domain: automation
  dedupe_status: deepens_existing
  deepens_leaf_id: "figma.platform.leaf.37998629035799-work-with-the-figma-agent-in-design-files"
  source_url: "https://help.figma.com/hc/en-us/articles/23870272542231"
  source_ids: [DEEP-S35]
  verification_status: VERIFIED
- id: "figma.deep.ai-features.ai-admin-controls"
  name: "AI enablement and data-training controls"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Admins toggle AI feature availability and content-training consent org-wide (entirely provider-dependent policy surface)."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us"
  source_ids: [DEEP-S47]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.organization-admin] Organization/Admin (Provider Posture; Local Concepts Noted)

```yaml
records:
- id: "figma.deep.organization-admin.saml-sso"
  name: "SAML SSO"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Org sign-in delegates to SAML identity providers (Okta, Entra ID, OneLogin, Google, custom) - provider posture: omitted for local-first Studio; only the authenticated-identity concept carries over."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040532333"
  source_ids: [DEEP-S41]
  verification_status: VERIFIED
- id: "figma.deep.organization-admin.scim-provisioning"
  name: "SCIM automatic provisioning"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "SCIM pushes user creation/deactivation, seat types, and group sync from identity providers - provider posture: omitted; group-based access is the local-relevant concept."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360048514653"
  source_ids: [DEEP-S40]
  verification_status: VERIFIED
- id: "figma.deep.organization-admin.domain-capture"
  name: "Domain capture and 2FA enforcement"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Orgs claim email domains to force accounts into the org and enforce two-factor auth - provider posture: omitted."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
- id: "figma.deep.organization-admin.admin-dashboard"
  name: "Org admin dashboard (members/teams/billing)"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "A central console manages members, seat assignments, teams, workspaces, billing groups, and shared resources - provider posture: omitted except the seat/role permission model as a local concept."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: VERIFIED
- id: "figma.deep.organization-admin.seat-model"
  name: "Seat types (full/dev/collab/view) permission model"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Access tiers gate editing, Dev Mode, and viewing per user per product - billing is provider-dependent, but the role-capability matrix is directly relevant to Studio's local permission model."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360040328273"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
- id: "figma.deep.organization-admin.library-analytics"
  name: "Library/design-system analytics"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Usage analytics show component/style/variable adoption, insertions, and detaches across the org - provider posture: hosted analytics omitted; local usage diagnostics over Studio's own registry is the analog."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: VERIFIED
- id: "figma.deep.organization-admin.activity-log"
  name: "Org activity log"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "An audit log records how members interact with files and resources, queryable in-app and via API - provider posture: hosted audit omitted; Studio's event ledger is the local analog."
  primitive_domain: diagnostics
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: VERIFIED
- id: "figma.deep.organization-admin.shared-fonts-admin"
  name: "Org shared font management"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Admins upload brand fonts shared to all org files - provider posture: hosted distribution omitted; project-local font bundles are the Studio analog."
  primitive_domain: typography
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
- id: "figma.deep.organization-admin.plugin-approval"
  name: "Plugin/widget allowlisting"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Admins restrict which community plugins/widgets members may run and approve requests - provider posture: hosted policy omitted; a local extension allowlist is the Studio analog."
  primitive_domain: automation
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
- id: "figma.deep.organization-admin.sharing-policy-defaults"
  name: "Org sharing/public-link policy defaults"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Org settings constrain default link sharing, public sharing, guest access, and web publishing for Sites/Make - provider posture: omitted; the policy-constraint concept informs Studio share defaults."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/31242876956183"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
- id: "figma.deep.organization-admin.workspaces"
  name: "Workspaces (enterprise org subdivisions)"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "Enterprise orgs subdivide into workspaces grouping teams, members, and default resources - provider posture: omitted; only hierarchical grouping carries over as a local concept."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
- id: "figma.deep.organization-admin.guest-management"
  name: "Guest access management"
  record_role: "feature_deep_delta"
  source_product: figma_platform
  app_behavior: "External guests receive scoped access to specific teams/files with an admin roster of all guests - provider posture: omitted; scoped-external-access is the local-relevant concept."
  primitive_domain: collaboration
  dedupe_status: new_surface
  source_url: "https://help.figma.com/hc/en-us/articles/360039829474"
  source_ids: [DEEP-S42]
  verification_status: UNVERIFIED
```

### [SFR-FIGMA-DEEP-DELTA.sources] Sources

```yaml
sources:
- { id: DEEP-S01, url: "https://help.figma.com/hc/en-us/articles/360040451373-Create-dynamic-designs-with-auto-layout", note: "Auto layout guide; fetched 2026-07-09." }
- { id: DEEP-S02, url: "https://help.figma.com/hc/en-us/articles/360039957734-Apply-constraints-to-define-how-layers-resize", note: "Constraints; fetched 2026-07-09." }
- { id: DEEP-S03, url: "https://help.figma.com/hc/en-us/articles/360040450513-Create-layout-guides", note: "Layout guides/grids; fetched 2026-07-09." }
- { id: DEEP-S04, url: "https://help.figma.com/hc/en-us/articles/360049283914", note: "Stroke properties incl. brush/dynamic/width profile; fetched 2026-07-09." }
- { id: DEEP-S05, url: "https://help.figma.com/hc/en-us/articles/360041488473-Apply-shadow-or-blur-effects", note: "Effects: shadows, blurs, progressive, noise, texture, glass; fetched 2026-07-09." }
- { id: DEEP-S06, url: "https://help.figma.com/hc/en-us/articles/360039956634-Explore-text-properties", note: "Full text property set; fetched 2026-07-09." }
- { id: DEEP-S07, url: "https://help.figma.com/hc/en-us/articles/15339657135383-Guide-to-variables-in-Figma", note: "Variables guide; fetched 2026-07-09." }
- { id: DEEP-S08, url: "https://help.figma.com/hc/en-us/articles/360040035834-Prototype-triggers", note: "Trigger catalog; fetched 2026-07-09." }
- { id: DEEP-S09, url: "https://help.figma.com/hc/en-us/articles/360040035874-Prototype-actions", note: "Action catalog; fetched 2026-07-09." }
- { id: DEEP-S10, url: "https://help.figma.com/hc/en-us/articles/360040522373-Prototype-animations", note: "Animation types; fetched 2026-07-09." }
- { id: DEEP-S11, url: "https://help.figma.com/hc/en-us/articles/360051748654-Prototype-easing-and-spring-animations", note: "Easing + spring catalog; fetched 2026-07-09." }
- { id: DEEP-S12, url: "https://help.figma.com/hc/en-us/articles/15023124644247-Guide-to-Dev-Mode", note: "Dev Mode guide; fetched 2026-07-09." }
- { id: DEEP-S13, url: "https://help.figma.com/hc/en-us/articles/32132100833559-Guide-to-the-Dev-Mode-MCP-Server", note: "MCP server help guide." }
- { id: DEEP-S14, url: "https://developers.figma.com/docs/figma-mcp-server/tools-and-prompts/", note: "MCP tool catalog; located via search 2026-07-09." }
- { id: DEEP-S15, url: "https://help.figma.com/hc/en-us/categories/360002051633-FigJam", note: "FigJam category TOC; fetched 2026-07-09." }
- { id: DEEP-S16, url: "https://help.figma.com/hc/en-us/categories/31304285531543-Figma-Make", note: "Make category TOC; fetched 2026-07-09." }
- { id: DEEP-S17, url: "https://help.figma.com/hc/en-us/articles/24170630629911-Explore-Figma-Slides", note: "Slides editor model; fetched 2026-07-09." }
- { id: DEEP-S18, url: "https://help.figma.com/hc/en-us/articles/24338209202327-Present-a-slide-deck", note: "Slides present modes." }
- { id: DEEP-S19, url: "https://help.figma.com/hc/en-us/articles/31230436657815-Explore-Figma-Sites", note: "Sites overview incl. CMS, publish." }
- { id: DEEP-S20, url: "https://help.figma.com/hc/en-us/articles/31242824165143-Guide-to-code-layers-in-Figma-Sites", note: "Sites code layers." }
- { id: DEEP-S21, url: "https://help.figma.com/hc/en-us/articles/31242797809815-Add-or-delete-breakpoints-in-a-webpage", note: "Sites breakpoints." }
- { id: DEEP-S22, url: "https://help.figma.com/hc/en-us/articles/31271566667543-Guide-to-Figma-Buzz", note: "Buzz guide incl. AI tools." }
- { id: DEEP-S23, url: "https://help.figma.com/hc/en-us/articles/31271824185623-Bulk-create-assets-in-Figma-Buzz", note: "Buzz bulk create CSV/XLSX." }
- { id: DEEP-S24, url: "https://help.figma.com/hc/en-us/articles/33155043230487-Set-up-brand-controls-for-your-team-with-Figma-Buzz", note: "Buzz brand controls/locking." }
- { id: DEEP-S25, url: "https://developers.figma.com/docs/plugins/api/nodes/", note: "Full scene node type list; fetched 2026-07-09." }
- { id: DEEP-S26, url: "https://developers.figma.com/docs/plugins/api/figma/", note: "Global figma object properties/methods/events; fetched 2026-07-09." }
- { id: DEEP-S27, url: "https://developers.figma.com/docs/plugins/manifest/", note: "Manifest fields, capabilities, permissions; fetched 2026-07-09." }
- { id: DEEP-S28, url: "https://developers.figma.com/docs/rest-api/", note: "REST API domain list + auth methods; fetched 2026-07-09." }
- { id: DEEP-S29, url: "https://developers.figma.com/docs/rest-api/webhooks/", note: "Webhooks v2; event types confirmed via docs/search 2026-07-09." }
- { id: DEEP-S30, url: "https://developers.figma.com/docs/rest-api/scopes/", note: "Granular OAuth scopes incl. enterprise variables gate." }
- { id: DEEP-S31, url: "https://developers.figma.com/docs/code-connect/", note: "Code Connect frameworks, UI vs CLI." }
- { id: DEEP-S32, url: "https://developers.figma.com/docs/widgets/api/api-reference/", note: "Widget API components/hooks/functions." }
- { id: DEEP-S33, url: "https://developers.figma.com/docs/embeds/embed-kit-2.0/", note: "Embed Kit 2.0 origin registration." }
- { id: DEEP-S34, url: "https://developers.figma.com/docs/embeds/embed-api/", note: "Embed API postMessage commands/events." }
- { id: DEEP-S35, url: "https://help.figma.com/hc/en-us/articles/23870272542231-Use-AI-tools-in-Figma-Design", note: "Current AI tool list incl. Figma Agent beta; fetched 2026-07-09." }
- { id: DEEP-S36, url: "https://help.figma.com/hc/en-us/articles/360041003694-Guide-to-fills", note: "Fill type taxonomy incl. pattern/video; fetched 2026-07-09." }
- { id: DEEP-S37, url: "https://help.figma.com/hc/en-us/articles/31440438150935-Draw-with-illustration-tools", note: "Draw pencil/brush tools; located via search 2026-07-09." }
- { id: DEEP-S38, url: "https://www.figma.com/blog/introducing-figma-draw/", note: "Draw feature set announcement (brushes, texture, pattern fills, noise, progressive blur, dynamic strokes)." }
- { id: DEEP-S39, url: "https://help.figma.com/hc/en-us/articles/360038006754-View-a-file-s-version-history", note: "Version checkpoints, restore, retention; confirmed via search 2026-07-09." }
- { id: DEEP-S40, url: "https://help.figma.com/hc/en-us/articles/360048514653-Set-up-automatic-provisioning-via-SCIM", note: "SCIM provisioning." }
- { id: DEEP-S41, url: "https://help.figma.com/hc/en-us/articles/360040532333-Guide-to-SAML-SSO", note: "SAML SSO guide." }
- { id: DEEP-S42, url: "https://help.figma.com/hc/en-us/articles/360039829474-Guide-to-managing-a-Figma-organization", note: "Org admin: workspaces, analytics, activity log." }
- { id: DEEP-S43, url: "https://help.figma.com/hc/en-us/articles/31616030150167-Use-patterns-as-a-fill-or-stroke", note: "Pattern fills." }
- { id: DEEP-S44, url: "https://help.figma.com/hc/en-us/articles/24245848829847-Add-and-view-presenter-notes", note: "Slides presenter notes." }
- { id: DEEP-S45, url: "https://help.figma.com/hc/en-us/articles/31242773374615-Insert-blocks-embeds-webpages-and-design-libraries-into-a-site", note: "Sites blocks/embeds/library insert." }
- { id: DEEP-S46, url: "https://help.figma.com/hc/en-us/articles/35895820755095-Figma-Sites-collection-Add-interactions-to-a-website", note: "Sites interactions." }
- { id: DEEP-S47, url: "https://help.figma.com/hc/en-us", note: "Figma Help Center root; general evidence anchor for rows drafted from known help topics not individually fetched this pass (marked UNVERIFIED)." }
- { id: DEEP-S48, url: "https://www.figma.com/release-notes/", note: "Release notes; recency cross-check." }
- { id: DEEP-S49, url: "https://help.figma.com/hc/en-us/categories/24146015318551-Figma-Slides", note: "Slides category TOC." }
- { id: DEEP-S50, url: "https://developers.figma.com/docs/widgets/api/figma-widget/", note: "figma.widget global." }
```





