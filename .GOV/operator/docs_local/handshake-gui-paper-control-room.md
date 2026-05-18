---
file_id: "handshake-gui-paper-control-room"
file_kind: "operator-local-design-exploration"
status: "draft"
created_at: "2026-05-17"
updated_at: "2026-05-18"
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

<topic id="operator-intent" status="draft" version="3" summary="Records the Operator's stated GUI intent, including project-only top tabs, pane header semantics, and bottom search." updated_at="2026-05-18">

## Operator Intent

The Operator wants Handshake to act similarly to a VS Code-style workspace, especially:

- Lockable and resizable tiled windows.
- Windows scoped per project.
- Multiple sub-windows inside each project window, arranged in grids like VS Code.
- Top workspace tabs are **project tabs only**; switching a top tab switches project context.
- Top-right global module navigation should expose the main Handshake modules: `MAIN`, `CKC`, `INGEST`, `STAGE`, `LAB`, and `STUDIO`.
- Each project persists its own window layout.
- Windows inside a project can carry many file/view tabs, with side-scrollable tab behavior like VS Code.
- Pane headers should show the active file name or window name, not the module/type name.
- Pane-local tabs should carry the module/type label so the Operator can see what kind of surface each tab represents.
- The active pane-local tab should use the same background color as its pane header to show which tab owns the header.
- Every project pane header should expose the same control set on the right: `+`, drawer, pop out, options, search, and close.
- More pane types than a normal code editor, including webviewer, stage, text, JSON, terminal, traces, artifacts, and other file-type viewers.
- A much stronger GUI direction than the current Handshake GUI.
- A brutalist paper feel rather than a generic dark app.
- Minimal, slightly floating surfaces with sharp corners.
- Stronger use of the bottom bar.
- A bottom search bar integrated into the bottom rail.
- A bottom drawer that can stow away items such as text, stage captures, terminal output, snippets, and temporary working material.
- The bottom drawer label/header should include a `+` affordance in the right corner of that same title box.
- The bottom drawer quick categories should read `Agenda`, `Mail`, `Lists`, and `Notes`.
- The left activity rail should include Agenda, Mail, and Notes icons/affordances.
- A folder/file drawer treatment where off-white label backgrounds are only as wide as the text or name, with variable widths per item instead of full-row fills.
- File and folder label text aligned toward the right, using the provided paper-strip reference as the visual cue.
- A real project tree in the project section of the left sidebar.
- Active-window quick links in the windows section of the left sidebar, with the owning project greyed out to the left of the window name.
- Muted syntax coloring that fits the dark brutalist style.
- No gaps between tiled windows.
- Scrollbar/splitter rails that visually belong to their pane: same dark background family, thin rail, no white outline, and hover/grab color change.
- Scrollbar rails live inside the window/pane, not outside it; the rail is a thin line, while the scrubber/knob may be slightly thicker.
- Example project tabs include `Handshake`, `Fast Focus`, and `Buurtman`.
- Example project windows include CUI workflow, terminal, Stage website mockup, and Stage YouTube video mockup.
- CastKit/CUI should be explored as a full-feature app inside Handshake, not merely as a narrow side panel, because visual files, characters, worldbuilding, stories, and stage outputs route through it.

</topic>

<topic id="core-verdict" status="draft" version="2" summary="States the recommended direction and main pushback." updated_at="2026-05-18">

## Core Verdict

The VS Code idea is strong structurally, but Handshake should not copy VS Code's mood. The layout model is the valuable part: a project-scoped, resizable, lockable, tiled control room.

Handshake should not become a freeform desktop full of loose floating windows. That would create focus, state, traceability, and validation problems. The better model is:

- A top-level **Project Workspace** per active project.
- Top-level tabs switch projects only.
- A persistent tiled layout inside each workspace.
- Scrollable file/window tabs live inside project panes, not in the global project tab row.
- Typed panes for each working surface.
- Lock states and saved layouts.
- Optional detachable or floating panes later, after the tiled layout and state model are solid.

The visual target should be **brutalist paper workbench**, not cyber terminal. The current mockup's dark grid, narrow rails, and IDE structure are useful. The texture, glow, and sci-fi tone should be reduced.

</topic>

<topic id="layout-model" status="draft" version="3" summary="Defines project tabs, pane tabs, pane headers, pane grid, and project-scoped layout persistence." updated_at="2026-05-18">

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

### Project Tabs Versus Pane Tabs

The top workspace tab row is reserved for projects only.

Rules:

- A top tab represents one project workspace.
- Switching a top tab switches project context.
- Switching a top tab restores that project's saved window layout.
- The top tab row should not hold files, individual panes, or editor documents.
- File/view tabs belong inside the relevant project pane group.
- Pane-local tabs can be side-scrollable like VS Code when many files or views are open.
- Pane-local tab labels should include the module/type name, such as `CKC / Workflow`, `TERM / Build`, or `STAGE / Preview`.
- The pane header should show the active tab's file name, document name, or window name, such as `cui-workflow.json`, `castkit-route.term`, or `website-stage.html`.
- Changing the active pane-local tab should update the pane header title.
- The active pane-local tab should visually connect to the pane header by sharing the same dark header background.
- Each pane tab should retain its pane type, content id, dirty state, and project id.
- Closing a project tab should not discard the project layout unless explicitly reset or deleted.

This avoids mixing two tab concepts. The operator always knows whether they are switching projects or switching content inside the current project.

### Main Module Navigation

The top-right chrome should expose Handshake's primary modules:

- `MAIN`
- `CKC`
- `INGEST`
- `STAGE`
- `LAB`
- `STUDIO`

This module switcher is not the same as project tabs.

Rules:

- Module buttons switch the major Handshake application surface.
- Project tabs switch active project workspace.
- Pane-local tabs switch files, views, or tools inside a project window.
- The three navigation layers should remain visually distinct.
- The module switcher should sit in the top-right corner of the global shell.

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
- Preserve zero-gap tiling between adjacent windows.
- Use thin integrated splitter/scrollbar rails rather than floating white handles.
- Keep the pane-header control set consistent across all project panes: add/new tab, drawer, pop out, options, search, and close.
- Treat current text controls as placeholders for later icon buttons; the action order should remain stable.

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

### Scrollbar And Splitter Semantics

Scrollbar and splitter rails should feel embedded in the window surface.

Rules:

- Rail background stays in the same dark family as the pane background.
- Rail line is thin.
- Rail is inside the pane/window edge, not floating between windows.
- Scrubber/knob can be thicker than the rail for grab affordance.
- Scrubber has no white outline.
- Scrubber color changes on hover and grab.
- Hit area may be larger than the visible line so resizing remains easy.
- Visible handle state should distinguish idle, hover, grab, and disabled.

</topic>

<topic id="castkit-cui-app-surface" status="draft" version="2" summary="Records the current preference for CastKit/CUI as a full app inside Handshake." updated_at="2026-05-18">

## CastKit / CUI App Surface

Current operator preference: CastKit/CUI should be treated as a full-feature app inside Handshake, not just a side panel.

Reason:

- Visual files route through it.
- Character records route through it.
- Worldbuilding records route through it.
- Story material routes through it.
- Website and video stage outputs route through it.
- Validation and export flows likely need full workspace context.

Mockup implication:

- CUI can be one of the project windows/panes.
- CUI should support its own pane-local tabs such as `CKC / Workflow`, `CKC / Characters`, `CKC / World`, `CKC / Stories`, `CKC / Routes`, and `CKC / Exports`.
- CUI should be able to route material to Stage panes, terminal tasks, drawer stash items, and validation evidence.
- Treat CUI as an app surface hosted by the project workspace, not as an accessory inspector.

Risk:

- If CastKit/CUI becomes only a small panel, the visual/story routing flow will likely become cramped and force important context into hidden state.

Mitigation:

- Give CUI a full pane/window surface with project-scoped layout persistence and typed routes into Stage, drawer, terminal, and validation panes.

</topic>

<topic id="bottom-drawer" status="draft" version="3" summary="Defines bottom search and bottom drawer as separate but adjacent working surfaces." updated_at="2026-05-18">

## Bottom Drawer

The bottom bar should become an active command, search, and stash surface. The bottom drawer should not be only a terminal. It should act like a **stash shelf** or **pastebin drawer** for work-in-progress material.

Bottom search is a persistent rail-level search/command input. It should remain separate from drawer contents so search is always available even when the drawer is collapsed.

The bottom drawer title box should include a right-aligned `+` affordance so adding a drawer item feels attached to the drawer itself, not hidden in a separate toolbar.

Useful bottom search scopes:

- `project:`
- `file:`
- `pane:`
- `window:`
- `stash:`
- `trace:`
- `terminal:`
- `stage:`
- `layout:`

The drawer should support:

- Agenda items.
- Mail references.
- Lists.
- Notes.
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

High-ROI design move: show the bottom drawer as a horizontal shelf of typed cells/tags, not as another full-height panel that competes with the main workspace.

Current drawer category labels should be `Agenda`, `Mail`, `Lists`, and `Notes`; the older `Text`, `Json`, `Term`, and `Stage` labels are too implementation-type oriented for the Operator-facing bottom drawer.

</topic>

<topic id="visual-language" status="draft" version="2" summary="Captures the brutalist paper style direction, including muted syntax and zero-gap panes." updated_at="2026-05-18">

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
- Zero-gap tiled windows.
- Integrated thin scrollbars and splitters.
- Muted syntax colors, not neon code colors.

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

Syntax color direction:

- Keys: dusty slate or paper gray.
- Strings: muted olive/green.
- Numbers: muted ochre.
- Booleans/errors: muted rust/red.
- Comments/metadata: dim warm gray.
- Never use saturated neon blue, purple, or green as the default code palette.

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

<topic id="risks-and-mitigations" status="draft" version="2" summary="Lists design and implementation risks with mitigations." updated_at="2026-05-18">

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

Risk: top-level project tabs and pane-local tabs become conceptually blurred.
Mitigation: reserve the top tab row for projects only; put file, view, and editor tabs inside pane groups.

Risk: project layout persistence corrupts, drifts, or restores an impossible pane graph after files/panes change.
Mitigation: version layout snapshots, validate before restore, support reset layout, and keep a last-known-good fallback per project.

Risk: splitters and scrollbars become too subtle to grab.
Mitigation: keep a thin visible rail but use a larger invisible hit target and clear hover/grab color changes.

Risk: zero-gap tiling becomes visually dense and hard to scan.
Mitigation: use active-pane borders, header contrast, selected tab states, and stable pane titles instead of adding gaps.

Risk: bottom drawer and bottom search compete for the same mental role.
Mitigation: define bottom search as query/command and drawer as stowed content; do not overload drawer tabs as search tabs.

Risk: muted syntax coloring becomes too low contrast.
Mitigation: test JSON, terminal, trace, and diff panes at normal laptop scale and maintain minimum contrast tokens for syntax classes.

</topic>

<topic id="high-roi-additions" status="draft" version="2" summary="Records cheap adjacent additions to carry into a later GUI work packet." updated_at="2026-05-18">

## High-ROI Additions

- Per-project saved layout presets. High ROI because layout persistence is already required for project windows and it reduces future workspace setup friction.
- Pane type registry. High ROI because Handshake will keep adding surfaces, and typed pane creation avoids one-off UI wiring.
- Machine-readable layout snapshot. High ROI because it supports model navigation, visual debugging, validation, and replay.
- Typed bottom drawer items. High ROI because the drawer can become artifact/prompt/validation infrastructure instead of disposable UI.
- Stable visual-debug selectors for panes, tabs, drawer items, and status bar regions. High ROI because GUI validation is already part of the Handshake direction.
- Paper label component. High ROI because the same component can serve file drawer rows, bottom drawer items, artifact chips, and schedule-like trace rows.
- Layout lock semantics. High ROI because locking is central to the Operator's request and also protects model-driven workflows from accidental pane churn.
- Project tab state model. High ROI because it separates project switching from pane/content switching and prevents future navigation confusion.
- Pane-local scrollable tab strip. High ROI because it preserves the familiar VS Code workflow while keeping project tabs clean.
- Layout snapshot schema with version, project id, pane ids, split ratios, active pane, drawer state, and last-known-good fallback. High ROI because it reduces layout corruption and supports model inspection.
- Universal bottom search scopes. High ROI because one search field can find projects, files, panes, stashed drawer items, traces, terminal blocks, and layouts.
- Integrated splitter component with visible rail plus invisible grab target. High ROI because it preserves the visual style while keeping resizing usable.
- Muted syntax token set. High ROI because JSON, terminal, traces, diff, markdown, and code panes can share one restrained palette.
- Active-window quick-link list with grey project prefix. High ROI because it improves navigation across panes without turning the sidebar into another full tab system.

</topic>

<topic id="persistent-mockup-assets" status="draft" version="1" summary="Records the durable location of the HTML mockup and render helper." updated_at="2026-05-18">

## Persistent Mockup Assets

The exploratory HTML mockup and its durable support files live in the repo, not in `Handshake_Artifacts`, because `Handshake_Artifacts` is cleaned between Work Packets.

Persistent folder:

```text
.GOV/operator/docs_local/handshake-gui-mockup/
```

Durable files:

- `paper-control-room-mockup.html`
- `paper-control-room-mockup-playwright-firefox.png`
- `paper-control-room-mockup-firefox.png`
- `render-paper-control-room-firefox.js`
- `package.json`
- `fonts/ibm-plex-sans-condensed-latin-400-normal.woff2`
- `fonts/ibm-plex-sans-condensed-latin-500-normal.woff2`
- `fonts/ibm-plex-sans-condensed-latin-600-normal.woff2`
- `fonts/ibm-plex-sans-condensed-latin-700-normal.woff2`

Regenerate Firefox screenshot from the mockup folder:

```powershell
npm install --no-package-lock
npm run render:firefox
```

`node_modules` and Playwright browser caches are runtime dependencies and should not be committed.

</topic>

<topic id="open-questions" status="draft" version="2" summary="Questions to resolve before implementation." updated_at="2026-05-18">

## Open Questions Before Implementation

1. Should panes be allowed to float on day one, or should day one be tiled-only with saved layouts?
2. Should the bottom drawer be global, project-scoped, or both with separate shelves?
3. Which pane types are day-one minimum: Stage, Text, JSON, Terminal, Webviewer, Trace?
4. Should file drawer labels be right-aligned globally, or only in a special brutalist file-browser mode?
5. Should Paper Control Room become the default Handshake visual language, or one theme among several?
6. How much dark surface should remain versus shifting more of the app to off-white paper?
7. What current Handshake GUI surfaces are salvageable versus replaceable?
8. Which layout library best fits the product after a spike?
9. What exact layout snapshot schema should be the first durable format?

</topic>

<topic id="candidate-work-packet-shape" status="draft" version="2" summary="Sketches a possible future Work Packet without making one yet." updated_at="2026-05-18">

## Candidate Future Work Packet Shape

Possible future packet name:

```text
WP-1-Paper-Control-Room-GUI-Foundation-v1
```

Possible scope:

- Establish the Paper Control Room design tokens.
- Implement a project-scoped workspace shell.
- Implement top-level project tabs only.
- Implement pane-local scrollable tabs inside project windows.
- Spike and select a docking/tiling library.
- Implement typed pane registry.
- Implement basic pane types: Stage, Text, JSON, Terminal, Webviewer.
- Implement bottom drawer as typed stash shelf.
- Implement file drawer paper labels.
- Implement project tree and active-window quick links in the left sidebar.
- Implement bottom search.
- Implement integrated scrollbar/splitter rail states.
- Implement muted syntax token colors.
- Add layout snapshot debug output.
- Add visual screenshot proof for desktop and constrained viewport.

Non-goals:

- Full replacement of every Handshake GUI surface.
- Final style guide law.
- Full multi-window OS-level behavior.
- Production-grade every-file-type editor parity.
- Full model-agent workflow integration.

</topic>
