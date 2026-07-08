---
file_id: "illustrator-figma-parity-matrix"
topic_id: SFR-ILLUSTRATOR-FIGMA-PARITY
status: draft
summary: "Primitive-centered parity lanes for adding Illustrator and Figma clone coverage to local-first Rust Studio."
sources: 5
updated_at: "2026-07-05"
---

## [SFR-ILLUSTRATOR-FIGMA-PARITY] Illustrator/Figma Studio Parity Matrix

### [SFR-ILLUSTRATOR-FIGMA-PARITY.matrix] Matrix

```yaml
parity_lanes:
  - id: "parity.vector_authoring"
    studio_surface: "StudioVectorPathGraph"
    parity_scope: "Illustrator paths/live shapes/shape builder; Figma vector networks/Draw/shape builder/vectorize."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.canvas_layout"
    studio_surface: "StudioPageSpread"
    parity_scope: "Illustrator artboards/large canvas; Figma pages/frames/sections/boards/slides/sites/auto layout."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.design_systems"
    studio_surface: "StudioStyleRegistry"
    parity_scope: "Illustrator symbols/graphic styles; Figma components/variants/slots/styles/variables/libraries."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.typography"
    studio_surface: "StudioTextRunAndStory"
    parity_scope: "Illustrator type/glyph tools; Figma text/fonts/text styles/text-to-path."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.appearance_color"
    studio_surface: "StudioColorPipeline"
    parity_scope: "Illustrator fills/strokes/gradients/mesh/recolor; Figma fills/effects/patterns/blends/color profiles."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.interaction_motion"
    studio_surface: "StudioInteractiveDocumentSurface"
    parity_scope: "Figma prototypes/Motion/Slides and Illustrator web/export animation-adjacent output."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.file_compatibility"
    studio_surface: "StudioFileIO"
    parity_scope: "AI/AIT/PDF/SVG/EPS/DWG/DXF/PSD plus FIG/JAM/SKETCH/PPTX/media/static/animation exports."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.collaboration_local"
    studio_surface: "StudioCollaborationSession"
    parity_scope: "Figma multiplayer/FigJam meetings/comments/history and Illustrator projects converted to local CRDT/EventLedger workflows."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.ai_provider_local"
    studio_surface: "StudioModelToolContract"
    parity_scope: "Illustrator Firefly/partner models and Figma AI/Make/Weave reinterpreted as provider-neutral/local model commands."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
  - id: "parity.extensibility_dev"
    studio_surface: "StudioActionGraph"
    parity_scope: "Figma plugin/widget/API/MCP and Illustrator automation/plugins/scripts as local extension host targets."
    local_first_requirement: "No cloud dependency for core use; optional providers and compatibility shims must expose diagnostics."
    rust_forward_requirement: "Promote through typed Rust command contracts, fixtures, receipts, undo/replay, and UserManual topic."
```

### [SFR-ILLUSTRATOR-FIGMA-PARITY.sources] Sources

```yaml
sources:
  - { id: IFP-M01, path: "19-studio-local-first-rust-posture.md", note: "Local-first Rust posture." }
  - { id: IFP-M02, path: "20-illustrator-feature-map.md", note: "Illustrator feature map." }
  - { id: IFP-M03, path: "21-figma-feature-map.md", note: "Figma feature map." }
  - { id: IFP-M04, path: "22-illustrator-leaf-index.md", note: "Illustrator leaves." }
  - { id: IFP-M05, path: "23-figma-leaf-index.md", note: "Figma leaves." }
```
