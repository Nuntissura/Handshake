---
file_id: "handshake-gui-paper-control-room"
file_kind: "operator-local-design-exploration"
status: "draft"
created_at: "2026-05-17"
updated_at: "2026-05-17"
owner: "operator"
authority: "non-authoritative"
---

# Handshake GUI: Paper Control Room

<topic id="scope-and-status" status="draft" version="1" summary="Clarifies that this note captures exploratory GUI direction and is not binding product law." updated_at="2026-05-17">

## Scope And Status

This note captures the Operator's exploratory GUI direction for Handshake after comparing a VS Code-like mockup, a cropped folder drawer example, and a brutalist paper reference image.

This file is not a Work Packet, Master Spec amendment, validator verdict, implementation order, or final style guide. It is a product-design exploration that can later be promoted into a style guide, refinement, or GUI Work Packet.

The working design name is **Paper Control Room**.

</topic>

<topic id="operator-intent" status="draft" version="1" summary="Records the Operator's stated GUI intent." updated_at="2026-05-17">

## Operator Intent

The Operator wants Handshake to act similarly to a VS Code-style workspace, especially:

- Lockable and resizable tiled windows.
- Windows scoped per project.
- Multiple sub-windows inside each project window, arranged in grids like VS Code.
- More pane types than a normal code editor, including webviewer, stage, text, JSON, terminal, traces, artifacts, and other file-type viewers.
- A much stronger GUI direction than the current Handshake GUI.
- A brutalist paper feel rather than a generic dark app.
- Minimal, slightly floating surfaces with sharp corners.
- Stronger use of the bottom bar.
- A bottom drawer that can stow away items such as text, stage captures, terminal output, snippets, and temporary working material.
- A folder/file drawer treatment where off-white label backgrounds are only as wide as the text or name, with variable widths per item instead of full-row fills.
- File and folder label text aligned toward the right, using the provided paper-strip reference as the visual cue.

</topic>

<topic id="core-verdict" status="draft" version="1" summary="States the recommended direction and main pushback." updated_at="2026-05-17">

## Core Verdict

The VS Code idea is strong structurally, but Handshake should not copy VS Code's mood. The layout model is the valuable part: a project-scoped, resizable, lockable, tiled control room.

Handshake should not become a freeform desktop full of loose floating windows. That would create focus, state, traceability, and validation problems. The better model is:

- A top-level **Project Workspace** per active project.
- A persistent tiled layout inside each workspace.
- Typed panes for each working surface.
- Lock states and saved layouts.
- Optional detachable or floating panes later, after the tiled layout and state model are solid.

The visual target should be **brutalist paper workbench**, not cyber terminal. The current mockup's dark grid, narrow rails, and IDE structure are useful. The texture, glow, and sci-fi tone should be reduced.

</topic>

<topic id="layout-model" status="draft" version="1" summary="Defines the proposed workspace, pane grid, and project-scoped layout model." updated_at="2026-05-17">

## Layout Model

### Project Workspace

Each project should open into its own workspace shell. The workspace owns:

- Project identity and status.
- Saved pane layout.
- Open panes and tabs.
- Bottom drawer contents.
- Active model/session traces.
- Artifact and validation context.

The same project should reopen with its prior layout unless the Operator chooses a preset or reset.

### Pane Grid

The central workspace should behave like a professional IDE-style docking grid:

- Split horizontally or vertically.
- Resize dividers.
- Tab multiple panes in one region.
- Move panes between regions.
- Lock panes in place.
- Lock panes against close/replacement.
- Save and restore layout state.
- Support future popout/floating windows without making them the default.

Pane examples:

- `Stage`
- `Webviewer`
- `Text`
- `Markdown`
- `JSON`
- `Terminal`
- `Trace`
- `Memory`
- `Artifact`
- `Validation`
- `Prompt`
- `Model Output`
- `Diff`
- `File Viewer`
- `Image Viewer`
- `Dataset/Table`

### Locking Semantics

Locking should be more than a visual pin:

- `layout_lock`: pane cannot be moved or resized.
- `content_lock`: pane content cannot be replaced by another file/artifact.
- `close_lock`: pane cannot be closed without confirmation.
- `project_scope_lock`: pane can only show content from the active project.
- `authority_lock`: pane displays authoritative state and cannot be edited directly.

</topic>

<topic id="bottom-drawer" status="draft" version="1" summary="Defines the bottom drawer as a typed stash shelf, not just a terminal panel." updated_at="2026-05-17">

## Bottom Drawer

The bottom bar should become an active command and stash surface. The bottom drawer should not be only a terminal. It should act like a **stash shelf** or **pastebin drawer** for work-in-progress material.

The drawer should support:

- Text snippets.
- Terminal output blocks.
- Stage captures.
- Webviewer captures.
- JSON fragments.
- Prompt fragments.
- Model output drafts.
- File references.
- Artifact references.
- Validation evidence.
- Temporary notes.

Drawer items should be typed. A drawer item is draft or stowed material until it is promoted into project state. This matches Handshake's authority model: loose material can be useful without becoming truth.

Useful drawer actions:

- Stow current selection.
- Pin item.
- Promote item.
- Send item to pane.
- Copy item into prompt/context.
- Attach item to validation evidence.
- Convert item to artifact.
- Discard item.

High-ROI design move: show the bottom drawer as a horizontal shelf of paper tags/cards, not as another full-height panel that competes with the main workspace.

</topic>

<topic id="visual-language" status="draft" version="1" summary="Captures the brutalist paper style direction." updated_at="2026-05-17">

## Visual Language

Working visual direction: **brutalist paper control room**.

Core traits:

- Sharp corners.
- Hard dividers.
- Minimal radii, ideally `0px` to `3px`.
- Off-white paper labels on ink/dark surfaces.
- Slight elevation only where it helps layering.
- No glossy app chrome.
- No gradient-orb decoration.
- No soft SaaS card language.
- No rounded pill-heavy interface.
- High contrast between structure and labels.
- Dense but readable workspace.
- Functional ugliness with discipline, not accidental ugliness.

The design should feel closer to:

- A filing cabinet.
- A technical drawing table.
- A paper strip schedule board.
- A brutalist IDE.
- A control room with physical labels.

The design should not feel like:

- A generic dark code editor.
- Cyberpunk dashboard.
- SaaS admin panel.
- Glassmorphism.
- Rounded card dashboard.
- Decorative bento UI.

Recommended palette direction:

- `ink`: near-black or blackened charcoal.
- `paper`: off-white, not pure white.
- `paper-muted`: slightly gray paper for inactive tags.
- `grid-line`: thin dark gray or black border.
- `accent`: one active marker color, likely orange/red.
- `warning`: red or vermilion.
- `ok`: muted green only where status needs it.

Typography direction:

- UI chrome: condensed or technical sans, all-caps labels used sparingly.
- Content/code: readable mono.
- Paper tags: mono or narrow sans, high contrast, compact line height.
- Avoid oversized hero typography inside the product UI.

</topic>

<topic id="folder-file-drawer" status="draft" version="1" summary="Defines the variable-width off-white label treatment for file and folder lists." updated_at="2026-05-17">

## Folder And File Drawer Treatment

The folder/file drawer should use the paper-strip reference:

- Rows stay aligned to a consistent grid.
- The off-white background belongs to the label only.
- The off-white background width follows the text width.
- Different folder/file names therefore create different label widths.
- Text should align toward the right edge of the drawer or row group.
- The row itself should not be fully filled by the selected background.
- A small left status marker can remain for active/changed/error states.

Suggested row anatomy:

```text
| left gutter | optional marker | flexible dark space | [ off-white text label ] |
```

Selection should not simply invert the whole row. Better states:

- Active file: orange/red left marker + paper label slightly brighter.
- Hover: thin outline or dark paper shadow, not full-row fill.
- Dirty file: small square/dot in the gutter.
- Locked file: small lock glyph in the gutter or label suffix.
- Folder group: uppercase label with file count as dim metadata.

Risk: variable-width labels can become chaotic if every item has a different anchor and no grid discipline. Mitigation: keep a stable row height, stable gutter, stable right anchor, max label width, and consistent truncation.

</topic>

<topic id="model-and-agent-usability" status="draft" version="1" summary="Explains why the GUI should support no-context models and parallel agents." updated_at="2026-05-17">

## Model And Agent Usability

The GUI should be designed for both the Operator and model agents.

Every pane should have:

- Stable pane ID.
- Pane type.
- Project ID.
- Current content ID.
- Lock state.
- Dirty state.
- Authority/advisory state.
- Last update timestamp.
- Optional related event IDs.

This lets a no-context model inspect the workspace without screen-reading guesswork.

The tiling model should expose a machine-readable layout snapshot:

```json
{
  "workspace_id": "project_demo",
  "layout_id": "layout_current",
  "panes": [
    {
      "pane_id": "pane_stage_main",
      "pane_type": "stage",
      "project_id": "project_demo",
      "content_id": "artifact_stage_latest",
      "lock_state": ["project_scope_lock"],
      "region": "center"
    }
  ]
}
```

This is important because Handshake is supposed to coordinate parallel model work. Visual layout should not become hidden state that only a human can interpret.

</topic>

<topic id="implementation-research" status="draft" version="1" summary="Records a small current scan of candidate layout libraries." updated_at="2026-05-17">

## Implementation Research Basis

This is not a final technology decision. It is a current scan of candidates worth testing before implementation.

Sources checked on 2026-05-17:

- Dockview documentation: <https://dockview.dev/docs/overview/introduction/>
- React Mosaic GitHub: <https://github.com/nomcopter/react-mosaic>
- FlexLayout GitHub: <https://github.com/caplin/FlexLayout>
- Golden Layout docs: <https://golden-layout.com/docs/>

Relevant patterns found:

- Dockview is closest to IDE-like docking. It advertises tabs, groups, drag and drop, floating panels, popout windows, serialization, and React support.
- React Mosaic is focused on tiling panes with drag-to-resize and drag-to-rearrange behavior. It is conceptually clean for a tiled workspace, but may need more custom work for IDE-grade pane types and bottom drawer integration.
- FlexLayout is a React docking layout manager with themes, tabs, splitters, popout support, and a mature model API. It is worth spiking if Dockview is too opinionated or mismatched.
- Golden Layout is a classic multi-window web layout manager, but its age and ecosystem fit need careful review before choosing it.

Recommended spike order:

1. Dockview.
2. FlexLayout.
3. React Mosaic.
4. Golden Layout only if the first three fail important constraints.

Selection criteria for the spike:

- Can persist and restore per-project layouts.
- Can lock panes or support lock behavior through controlled state.
- Can support typed pane registry.
- Can support bottom drawer and status bar integration.
- Can expose machine-readable layout snapshots.
- Can be styled into sharp brutalist paper UI without fighting default chrome.
- Can run inside the existing Handshake frontend stack.
- Can support visual testing and stable selectors.

</topic>

<topic id="risks-and-mitigations" status="draft" version="1" summary="Lists design and implementation risks with mitigations." updated_at="2026-05-17">

## Risks And Mitigations

Risk: freeform windows create chaos.
Mitigation: default to tiled layouts; add popout/floating later as advanced behavior.

Risk: VS Code imitation makes Handshake feel generic.
Mitigation: copy layout mechanics, not the visual mood. Use Paper Control Room as the style target.

Risk: brutalist style becomes unreadable.
Mitigation: keep brutalism in the frame, labels, and surface treatment; keep content typography clear and accessible.

Risk: variable-width paper labels make the file drawer messy.
Mitigation: right-anchor labels, keep stable row rhythm, cap widths, use truncation, and keep metadata in a separate dim column.

Risk: bottom drawer becomes a junk drawer.
Mitigation: make drawer items typed, searchable, promotable, and discardable.

Risk: models cannot operate the GUI reliably.
Mitigation: add stable pane IDs, layout snapshots, semantic pane types, and machine-readable drawer item records.

Risk: design exploration becomes accidental product law.
Mitigation: keep this file marked non-authoritative until promoted through a Work Packet, style guide, or Master Spec update.

</topic>

<topic id="high-roi-additions" status="draft" version="1" summary="Records cheap adjacent additions to carry into a later GUI work packet." updated_at="2026-05-17">

## High-ROI Additions

- Per-project saved layout presets. High ROI because layout persistence is already required for project windows and it reduces future workspace setup friction.
- Pane type registry. High ROI because Handshake will keep adding surfaces, and typed pane creation avoids one-off UI wiring.
- Machine-readable layout snapshot. High ROI because it supports model navigation, visual debugging, validation, and replay.
- Typed bottom drawer items. High ROI because the drawer can become artifact/prompt/validation infrastructure instead of disposable UI.
- Stable visual-debug selectors for panes, tabs, drawer items, and status bar regions. High ROI because GUI validation is already part of the Handshake direction.
- Paper label component. High ROI because the same component can serve file drawer rows, bottom drawer items, artifact chips, and schedule-like trace rows.
- Layout lock semantics. High ROI because locking is central to the Operator's request and also protects model-driven workflows from accidental pane churn.

</topic>

<topic id="open-questions" status="draft" version="1" summary="Questions to resolve before implementation." updated_at="2026-05-17">

## Open Questions Before Implementation

1. Should a project open as one workspace tab inside Handshake, a top-level app window, or both?
2. Should panes be allowed to float on day one, or should day one be tiled-only with saved layouts?
3. Should the bottom drawer be global, project-scoped, or both with separate shelves?
4. Which pane types are day-one minimum: Stage, Text, JSON, Terminal, Webviewer, Trace?
5. Should file drawer labels be right-aligned globally, or only in a special brutalist file-browser mode?
6. Should Paper Control Room become the default Handshake visual language, or one theme among several?
7. How much dark surface should remain versus shifting more of the app to off-white paper?
8. What current Handshake GUI surfaces are salvageable versus replaceable?
9. Which layout library best fits the product after a spike?

</topic>

<topic id="candidate-work-packet-shape" status="draft" version="1" summary="Sketches a possible future Work Packet without making one yet." updated_at="2026-05-17">

## Candidate Future Work Packet Shape

Possible future packet name:

```text
WP-1-Paper-Control-Room-GUI-Foundation-v1
```

Possible scope:

- Establish the Paper Control Room design tokens.
- Implement a project-scoped workspace shell.
- Spike and select a docking/tiling library.
- Implement typed pane registry.
- Implement basic pane types: Stage, Text, JSON, Terminal, Webviewer.
- Implement bottom drawer as typed stash shelf.
- Implement file drawer paper labels.
- Add layout snapshot debug output.
- Add visual screenshot proof for desktop and constrained viewport.

Non-goals:

- Full replacement of every Handshake GUI surface.
- Final style guide law.
- Full multi-window OS-level behavior.
- Production-grade every-file-type editor parity.
- Full model-agent workflow integration.

</topic>
