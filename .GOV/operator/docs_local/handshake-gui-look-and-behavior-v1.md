---
file_id: "handshake-gui-look-and-behavior-v1"
file_kind: "operator-design-spec"
updated_at: "2026-06-19"
status: "guideline-not-law"
authority: "non-authoritative"
owner: "operator"
linked_wps: ["WP-KERNEL-011-Native-WorkSurface-Shell-v1", "WP-KERNEL-012-Native-Editors-Obsidian-VSCode-Parity-v1"]
---

# Handshake GUI: Look and Behavior v1

> **This document is a design guideline, not the definitive look.** It describes intent, structure, and behavior targets. Implementation details may deviate where toolkit constraints, operator corrections, or emerging evidence require it. Only surfaces that exist in code are populated now; future surfaces (Photoshop/Affinity-class editors, spreadsheet, render engines) are designed-in extension points referenced but not specified here.

---

<topic id="status-and-scope" status="active" version="1" summary="Scope, authority, and relationship to WP-011 and WP-012." updated_at="2026-06-19">

## Status and Scope

This file is the `gui_reference` authority surface named in both WP-KERNEL-011 and WP-KERNEL-012. It is an **operator-authored prose surface** under GLOBAL-GOVARTIFACTS-019 and carries no normative weight by itself; authority lives in the WP packet contracts and MT contracts.

Its purpose is to give every MT author and model a shared, stable picture of:

- What Handshake's native work surface looks like visually.
- How the three navigation layers behave.
- What pane types exist and how they compose.
- What menus, settings, context menus, keyboard shortcuts, and bottom surfaces look like.
- How theming works.
- How the work surface behaves under interaction (resize, drag, focus, keyboard).
- How LLM swarm agents see and steer the surface (AccessKit).

**What this file is not:**

- A final pixel-perfect style guide.
- A binding design law that blocks an MT from making a pragmatic deviation.
- A specification for surfaces not yet in code (those are extension points described in outline only).

When this file and an MT contract conflict, the MT contract wins. When this file and the operator's live correction conflict, the correction wins.

</topic>

<topic id="design-lineage" status="active" version="1" summary="Where the design comes from and what was deliberately changed from the mockup." updated_at="2026-06-19">

## Design Lineage

The visual starting point is the operator's **Paper Control Room** mockup (`handshake-gui-mockup/paper-control-room-mockup-firefox.png` and `handshake-gui-paper-control-room.md`). That mockup established all the structural mechanics that are kept here:

- Project tabs / module switcher / pane-local tabs as three distinct navigation layers.
- Dockable tiled work-surface with zero-gap panes.
- Bottom search rail + bottom drawer stash shelf.
- Left rail: project tree, active windows, stash, Agenda/Mail/Notes affordances.
- Typed pane headers with a consistent right-side control set.
- Muted syntax palette; thin integrated scrollbar/splitter rails.

**What is deliberately dropped from the brutalist mockup mood:**

The mockup used a heavy brutalist treatment: all-caps white-on-black everything, neon-orange accents, harsh paper-label corners, and a strong "physical control board" texture. The operator's instruction is to keep the layout mechanics but shift to **dark + light themes only, clean and readable**. The result is a disciplined, functional UI that references the paper-control-room idea in its structural bones (sharp corners, dense labels, integrated rails, project-scoped tiling) without the theatrical visual aggression of the mockup.

In practice this means:

- Corners remain sharp (0–2px radius) but borders are quiet, not aggressive.
- Labels are compact but not all-caps by default; section headers may use uppercase sparingly.
- Accent color exists (one muted warm tone) but does not dominate every surface.
- The "paper label" file-drawer treatment is kept as an option but does not apply everywhere.
- Dark theme is the default; light theme is a first-class alternate, not an afterthought.

</topic>

<topic id="overall-layout" status="active" version="1" summary="The global shell structure: title bar, left rail, center work-surface, status bar." updated_at="2026-06-19">

## Overall Layout

The global shell is a single native window divided into five fixed structural regions. No region scrolls or floats outside the window by default. Pop-out windows are individual panes detached into their own OS window; they share the same region model.

```
+-------------------------------------------------------------------+
| TITLE BAR: app name | project tabs          | module switcher     |
+------+------------------------------------------------------------+
| LEFT |  CENTER: dockable tiled pane work-surface                  |
| RAIL |  (pane grid: split/resize/tab/drag/pop-out)               |
|      |                                                            |
|      |                                                            |
|      +------------------------------------------------------------+
|      |  BOTTOM: search rail + drawer shelf                        |
+------+------------------------------------------------------------+
| STATUS BAR                                                         |
+-------------------------------------------------------------------+
```

### Title Bar

- Left side: native window controls (close/minimize/maximize). App icon or wordmark `HANDSHAKE` in condensed text.
- Center: active **project tab row** (project names only, scrollable, no files or views here).
- Right: **module switcher** buttons (`MAIN` `CKC` `INGEST` `STAGE` `LAB` `STUDIO`).
- Far right: global system status chip (workspace health, layout save indicator).

The title bar height is the minimum required to fit the project tab row comfortably. On Windows it doubles as the native drag region.

### Left Rail

Narrow fixed-width column (approx. 200–280px, operator-resizable via a drag handle). Contains:

1. **Project tree** section: tree of folders/files for the active project. Collapsed by default to top-level folders; expandable.
2. **Windows** section: quick-link list of all open panes in the active project. Each row shows `[project-prefix greyed]  pane-name`. Clicking navigates to and focuses that pane.
3. **Stash** section: count of bottom-drawer stash items. Clicking expands the bottom drawer to the Stash category.
4. **Agenda / Mail / Notes** icons at the bottom of the rail: icon buttons that open the corresponding bottom-drawer category or open a pane.
5. **Search** icon: opens the bottom search rail input in focused mode.

The left rail can be collapsed to an icon-only strip. It never hides completely by default.

### Center Work-Surface

The main dockable pane grid. See the Pane Model topic for full details.

### Bottom Zone

Two sub-rows stacked vertically:

- **Search rail** (always visible, single line): scoped search/command input. Never collapses.
- **Bottom drawer shelf** (toggleable, horizontal card shelf): stash categories and typed item cards. Collapses to zero height; toggle is the drawer label row which always stays visible as a 1-row affordance.

### Status Bar

Single-row bar at the very bottom. Left to right:

- Active project id + workspace status (CLEAN / DIRTY / ERROR).
- Layout save status (SAVED / SAVING / ERROR).
- Active pane type + current document name.
- Cursor/position info (line/col) when an editor pane is active.
- Swarm agent status (IDLE / N ACTIVE / ERROR).
- Theme badge (DARK / LIGHT).
- Right-click on any status bar segment opens a context menu for that segment.

</topic>

<topic id="three-navigation-layers" status="active" version="1" summary="Project tabs vs module switcher vs pane-local tabs: rules, visuals, and behavior." updated_at="2026-06-19">

## Three Navigation Layers

There are exactly three navigation layers. They must remain visually distinct at all times. Mixing them is a design defect.

### Layer 1 — Project Tabs (top center)

- One tab per open project workspace.
- Switching a project tab restores that project's last saved pane layout.
- Tab shows: project name. Optional dirty indicator dot.
- Tab row is horizontally scrollable when many projects are open.
- Right-clicking a project tab: context menu with `Close project`, `Rename project`, `Reset layout`, `Duplicate layout`, `Save layout preset`, `Close other projects`.
- Adding a project: `+` button at the end of the tab row, or `FILE > Open Project`.
- Closing a project does not discard its saved layout.
- Keyboard: `Ctrl+Tab` / `Ctrl+Shift+Tab` cycle project tabs. `Ctrl+1..9` jump by index.

### Layer 2 — Module Switcher (top right)

- Six labeled buttons: `MAIN` `CKC` `INGEST` `STAGE` `LAB` `STUDIO`.
- Switching module changes which set of pane types are available and changes the active module context shown in pane headers.
- The module switcher is global, not project-scoped. All panes reflect the active module.
- Active module button is visually highlighted (filled background vs outline).
- Keyboard: `Ctrl+Shift+1..6` by module index.
- Modules correspond to the existing React `ModuleId` type and `MODULE_DEFINITIONS` in `App.tsx`.

Module content summary (matching legacy React `MODULE_DEFINITIONS`):

| Module | Primary surfaces |
|--------|-----------------|
| MAIN | Workspace / Notes / Journal / Wiki / Problems / Jobs / Timeline |
| CKC | Atelier / Kernel DCC / Code Symbol / Source Control / Notes / Journal / Wiki |
| INGEST | Media Downloader / Fonts / Flight Recorder / Visual Debugger |
| STAGE | Fonts / Inference Lab / Visual Debugger / Flight Recorder |
| LAB | Inference Lab / Model Runtime / Swarm / Kernel DCC / Manual |
| STUDIO | Model Runtime / Swarm / Inference Lab / Kernel DCC / Manual |

### Layer 3 — Pane-Local Tabs (inside each pane)

- A horizontal tab strip lives at the top of each pane, just below the pane header.
- Each tab shows: `MODULE / SURFACE-TYPE` label (e.g. `CKC / Atelier`, `MAIN / Journal`, `INGEST / Fonts`).
- The pane header title shows the active content name (document name, file name, or surface name).
- The active tab shares the same background tone as the pane header, creating a visual "ownership" connection.
- Tab strip is horizontally scrollable when many tabs are open; tabs do not wrap.
- Tabs can be reordered by drag within the same pane.
- Tabs can be moved between panes by drag onto another pane's header or tab strip.
- Right-clicking a pane-local tab: per-tab context menu (see Context Menus topic).
- Dirty state: small dot in the tab label.
- Pinned state: pin glyph suffix; pinned tabs do not close on `Close Others`.
- Keyboard within a pane: `Ctrl+PgUp/PgDn` cycle pane tabs.

</topic>

<topic id="pane-model" status="active" version="1" summary="Pane types, pane header anatomy, split/resize/drag/pop-out behavior, and lock semantics." updated_at="2026-06-19">

## Pane Model

### Typed Pane Registry

Every pane has a declared `pane_type` from a registry. The registry is the canonical list of surfaces the work-surface can host. MTs add types to the registry; unknown types render a placeholder with the type name and an error message.

Currently registered types (from legacy React surface inventory + new native targets):

| pane_type | Description |
|-----------|-------------|
| `workspace` | Rich-text + knowledge editor (Obsidian/Notion class, WP-012 E2) |
| `code` | Code editor (VS Code class, WP-012 E1) |
| `canvas` | Loom canvas / knowledge graph board (WP-012 E3) |
| `loom-block` | Single Loom block inspector |
| `loom-daily-journal` | Daily journal surface |
| `loom-wiki-page` | Wiki page projection |
| `loom-search` | Loom Search V2 surface |
| `terminal` | Embedded terminal (alacritty_terminal) |
| `stage` | Stage preview / capture board |
| `webviewer` | In-app browser pane (bundled webview, isolated) |
| `json` | JSON viewer/editor |
| `diff` | Diff / merge editor |
| `trace` | Flight Recorder / event trace viewer |
| `atelier` | CKC Atelier (media/character/worldbuilding) |
| `kernel-dcc` | Kernel DCC projection (governance taskboard) |
| `inference-lab` | Inference Lab |
| `model-runtime` | Model Runtime panel |
| `swarm` | Swarm Operator Surface |
| `source-control` | Source Control panel |
| `code-symbol` | Code Symbol inspector |
| `fonts` | Font Manager |
| `media-downloader` | Media Downloader |
| `flight-recorder` | Flight Recorder View |
| `visual-debugger` | Visual Debugger |
| `user-manual` | Built-in User Manual |
| `problems` | Problems / diagnostics list |
| `jobs` | Jobs list |
| `timeline` | Event timeline |
| `image-viewer` | Image / asset viewer |
| `dataset-table` | Tabular dataset viewer |
| `artifact` | Artifact inspector |
| `validation` | Validation evidence surface |
| `prompt` | Prompt fragment editor |
| `model-output` | Model output viewer |
| `placeholder` | Empty / loading placeholder |

**Extension point:** future types (`photo-editor`, `spreadsheet`, `render-engine`) plug into the same registry without changing shell code.

### Pane Header Anatomy

Each pane has a header row that persists regardless of the active tab:

```
[ pane-type-icon ] [ active-content-title ]   [+] [drawer] [pop-out] [options] [search] [x]
```

- **pane-type-icon**: small glyph indicating the active tab's pane type.
- **active-content-title**: name of the current document, file, or surface (updated when the active tab changes). Not the module name.
- **`+`**: add a new tab to this pane (opens a type picker).
- **drawer**: toggle a pane-local side drawer / inspector panel (not the bottom drawer).
- **pop-out**: detach this pane into its own OS window. The window shows the same header. Merge back via the same button or by dragging back into the main grid.
- **options**: pane-local options menu (lock state, layout presets for this pane, content settings).
- **search**: focus search inside the active pane content.
- **x**: close the active tab (or the pane if only one tab remains; confirm if `close_lock` is set).

Lock state badge: if any lock is active, a small badge appears in the header after the title: `[LAYOUT_LOCK]` `[CONTENT_LOCK]` `[CLOSE_LOCK]` `[AUTH_LOCK]`. These correspond to the five lock types defined in the operator's design doc.

### Lock Types

| lock_type | Effect |
|-----------|--------|
| `layout_lock` | Pane cannot be moved or resized by any actor including swarm agents. |
| `content_lock` | Pane content cannot be replaced by navigation or drag; read-only display. |
| `close_lock` | Pane cannot be closed without explicit confirmation (human or agent). |
| `project_scope_lock` | Pane only shows content from the active project. |
| `authority_lock` | Pane displays authoritative state; direct editing disabled; changes route through proper channels. |

Lock state is part of the persisted layout snapshot.

### Split and Resize

- Horizontal split: adds a pane to the right of the current pane.
- Vertical split: adds a pane below the current pane.
- Splitter rails: thin (2px visible), same dark family as the pane background. Hit target is wider (8–12px) for easy grab. Color transitions: idle (matches background), hover (slightly lighter), grab (accent tone). No white outlines.
- Minimum pane size: 80px width and 60px height.
- Resize is live (no ghost/preview mode needed, but not prohibited if toolkit prefers it).
- Keyboard: `Ctrl+Alt+Arrow` to move focus between panes. `Ctrl+Shift+Arrow` to resize the active pane by a fixed step.

### Tab Drag Within and Between Panes

- Tab drag within a pane: reorder by dragging the tab label left/right.
- Tab drag to another pane: drop on the target pane's tab strip or pane header; tab moves.
- Tab drag to empty space: creates a new pane region at the drop edge.
- Drag MIME type from legacy React: `application/x-handshake-document-tab` (keep compatible for any future hybrid period).
- Visual feedback during drag: drop target pane highlighted; invalid drop zones show no highlight.

### Pop-Out and Merge

- Pop-out: pane becomes a borderless native OS window. The original slot in the main grid shows a placeholder with a "merge back" affordance.
- Pop-out window shows the full pane (header + tabs + content + scrollbars) but no left rail or status bar.
- Merge: drag the pop-out window back onto the main window, or use "Merge back" button in the pop-out.
- Multiple simultaneous pop-outs are allowed.
- Pop-out windows do not steal focus from the main window when opened (HBR-QUIET).

### Scrollbar and Splitter Rail Rules

Identical rules apply to both scrollbars and splitters:

- Rail background: same dark tone as the pane, not a separate color.
- Rail line: thin (1–2px visible).
- Rail position: inside the pane edge, never floating between panes.
- Scrubber/knob: may be thicker than the rail (4–6px) for grab affordance.
- States: idle (dim, nearly invisible), hover (visible), grab (accent tone).
- Hit area: at least 8px wider/taller than the visible rail for reliable targeting.
- No white outline on any state.

### Pane State Model (machine-readable)

Every pane exposes a stable JSON state for AccessKit and model inspection:

```json
{
  "pane_id": "pane-a",
  "pane_type": "workspace",
  "project_id": "project-handshake",
  "module_id": "MAIN",
  "active_tab_id": "main/journal",
  "active_content_id": "KRD-20260619-001",
  "lock_state": [],
  "dirty": false,
  "region": "top-left",
  "pop_out": false,
  "layout_lock": false,
  "content_lock": false,
  "close_lock": false,
  "authority_lock": false,
  "last_updated_at": "2026-06-19T10:00:00Z"
}
```

The full workspace layout snapshot (for agent inspection and restore) adds:

```json
{
  "workspace_id": "project-handshake",
  "layout_id": "layout-current",
  "layout_version": 1,
  "snapshot_at": "2026-06-19T10:00:00Z",
  "split_weights": { "vertical": 0.55, "horizontal": 0.5 },
  "active_pane_id": "pane-a",
  "active_module": "MAIN",
  "drawers": { "left_rail": true, "bottom": true },
  "panes": [ /* pane state array */ ],
  "last_known_good_id": "layout-lkg-001"
}
```

Layout snapshots are versioned. On restore, the shell validates before applying; on failure it falls back to `last_known_good`.

</topic>

<topic id="file-drawer-treatment" status="active" version="1" summary="The paper-strip file/folder drawer visual style in the left rail project tree." updated_at="2026-06-19">

## File Drawer Treatment

The project tree in the left rail uses the **paper-strip label** visual pattern from the mockup. This is the one place where the paper aesthetic is applied most strongly.

Row anatomy:
```
| left-gutter (8px) | status-marker (4px) | flex dark space | [ text label ] |
```

- The `text label` has its own background (slightly lighter than the row background) sized to the text width only, not the full row width.
- Labels are right-aligned within the row, so different-length names create variable left edges.
- The row itself does not fill a solid selection background.

States:

| State | Visual |
|-------|--------|
| Normal | Dark row, text label with subtle background |
| Hover | Thin 1px outline on the label, no full-row fill |
| Active/open | Left-edge colored marker (accent tone) + label slightly brighter |
| Dirty | Small square dot in the gutter |
| Locked | Lock glyph appended to label |
| Error | Red left-edge marker |

Folder groups: uppercase label with file count as dim metadata suffix.

Max label width: capped at 85% of the rail width; truncate with ellipsis at the left if needed (right-most text stays visible since filenames are most distinct at the suffix).

Risk: variable-width labels can scan poorly. Mitigation: stable row height (24px), stable gutter, stable right anchor, consistent truncation, and a separate dim metadata column for count/status.

</topic>

<topic id="menus" status="active" version="1" summary="Top menu bar structure: FILE / EDIT / VIEW / GO / RUN / HELP and their items." updated_at="2026-06-19">

## Menus

The top menu bar lives in the title bar row, left-aligned before the project tabs. Native OS menu bar integration is used where the toolkit supports it; otherwise an in-app custom menu bar.

Menu bar order: `FILE` `EDIT` `VIEW` `GO` `RUN` `HELP`

### FILE

- New Document... `Ctrl+N`
- New Project... `Ctrl+Shift+N`
- Open Project... `Ctrl+O`
- Open File... `Ctrl+Shift+O`
- Close Tab `Ctrl+W`
- Close Project
- ---
- Save `Ctrl+S`
- Save All `Ctrl+Shift+S`
- Save As... `Ctrl+Shift+A`
- ---
- Export Document... (format picker: HTML / Markdown / TXT / JSON)
- Export Governance Pack...
- Export Debug Bundle...
- ---
- Settings / Options... `Ctrl+,`
- ---
- Quit `Ctrl+Q`

### EDIT

- Undo `Ctrl+Z`
- Redo `Ctrl+Y`
- ---
- Cut `Ctrl+X`
- Copy `Ctrl+C`
- Paste `Ctrl+V`
- Select All `Ctrl+A`
- ---
- Find / Replace `Ctrl+F` / `Ctrl+H`
- Find in Files `Ctrl+Shift+F`
- Replace in Files `Ctrl+Shift+H`
- ---
- Toggle Comment `Ctrl+/`
- Format Document `Alt+Shift+F`
- ---
- Command Palette `Ctrl+Shift+P`
- Quick Switcher `Ctrl+P`

### VIEW

- Toggle Left Rail `Ctrl+B`
- Toggle Bottom Drawer `Ctrl+J`
- ---
- Split Pane Horizontal `Ctrl+\`
- Split Pane Vertical `Ctrl+Shift+\`
- Close Active Pane
- ---
- Pop Out Active Pane `Ctrl+Shift+M`
- Merge All Pop-Outs
- ---
- Zoom In `Ctrl+=`
- Zoom Out `Ctrl+-`
- Reset Zoom `Ctrl+0`
- ---
- Toggle Full Screen `F11`
- ---
- Theme > Dark | Light | System
- ---
- Layout Presets > [list of saved presets] | Save Current as Preset... | Reset to Default
- ---
- Show Layout JSON (debug)
- Show AccessKit Tree (debug)
- Show Swarm Status

### GO

- Go to File... `Ctrl+P`
- Go to Symbol... `Ctrl+Shift+O`
- Go to Line... `Ctrl+G`
- Go to Definition `F12`
- Go to References `Shift+F12`
- ---
- Go to Next Diagnostic `F8`
- Go to Previous Diagnostic `Shift+F8`
- ---
- Navigate Back `Alt+Left`
- Navigate Forward `Alt+Right`
- ---
- Switch Project Tab > [project list]
- Switch Module > MAIN | CKC | INGEST | STAGE | LAB | STUDIO
- Focus Pane > [pane list]

### RUN

- Build `F5`
- Run Active Task `Ctrl+Shift+B`
- Stop Task `Ctrl+C` (in terminal context)
- ---
- Open Terminal `Ctrl+\`` (backtick)
- Clear Terminal
- ---
- Trigger AI Job...
- View Jobs
- View Problems
- View Timeline
- ---
- Loom AI Review...
- Run Validation...
- Export Validation Evidence...

### HELP

- User Manual `F1`
- Search Manual...
- ---
- Keyboard Shortcuts
- ---
- Show Debug Panel
- Show Visual Debugger
- Show Flight Recorder
- ---
- About Handshake
- Check for Updates...

</topic>

<topic id="settings-dialog" status="active" version="1" summary="Settings / Options dialog sections and their contents." updated_at="2026-06-19">

## Settings / Options Dialog

Opened via `FILE > Settings / Options...` or `Ctrl+,`. A modal dialog with a two-column layout: section list on the left, settings form on the right.

### General

- Language / locale (future extension point; English only now).
- Startup: reopen last project (toggle).
- Startup: restore last layout (toggle).
- Auto-save interval (seconds; 0 = off).
- Confirm before closing dirty documents (toggle).
- Show dirty indicator in pane tabs (toggle).

### Appearance

- **Theme**: Dark | Light | System. Applied instantly on change.
- Font size: UI chrome (px). Separate control from editor font size.
- Density: Compact | Default | Comfortable (adjusts row heights and padding).
- Left rail width (px, default 240).
- Show file icons in project tree (toggle).
- Show module badge in pane tabs (toggle).
- Syntax color scheme: Muted (default) | Standard | Custom (palette editor, future).

### Keybindings

- Editable keybinding table. Column: Action | Default Shortcut | Custom Shortcut.
- Search/filter by action name.
- Reset individual binding | Reset All.
- Key actions covered: all commands listed in menus plus editor-specific actions.
- The existing React `workspaceSettings.keybindings` map (`app.command_palette.open`, `app.quick_switcher.open`, and all defined editor chords) is the source for the default list.

### Layout

- Default layout preset: picker.
- Save current layout as preset: named input + Save.
- Manage presets: list with rename/delete.
- Reset to default layout button.
- Show layout save status in status bar (toggle).

### Pane Defaults

- Default pane type when adding a new pane (picker from registry).
- Default module for new panes (picker).
- Confirm before closing last tab in pane (toggle).

### Accessibility / Model Surface

- Enable AccessKit (toggle; default on; disable only for debugging).
- AccessKit log level: Off | Errors | Verbose.
- Expose MCP swarm tool surface: Off | Localhost | Named pipe.
- MCP bind address or pipe name (text input; visible only when surface is enabled).
- Session token display (read-only; regenerate button).
- Screenshot capture path: Enabled | Disabled.

### Backend / Workspace

- Active PostgreSQL connection string (read-only display; managed by handshake_core).
- Workspace health check: button that triggers a liveness probe and displays result.
- Event ledger (Flight Recorder) retention: days (numeric).
- Layout snapshot retention: count of last-known-good snapshots to keep.

### About

- Version string, build date, commit hash.
- Bundled dependency versions (egui/wgpu or GPUI, AccessKit, Alacritty, etc.).
- License (read-only text area).
- Open logs folder button.

</topic>

<topic id="context-menus" status="active" version="1" summary="Right-click context menus for every surface: tabs, pane headers, file rows, drawer items, status bar, canvas nodes." updated_at="2026-06-19">

## Context Menus

No context menus exist in the current React app. This is the first time a context-menu system is being defined; WP-011 Cluster C5 builds the infrastructure and first-pass per-surface menus.

### Project Tab Context Menu

Right-click on a project tab:

- Rename project...
- Close project
- Close other projects
- ---
- Reset layout
- Save layout as preset...
- Apply layout preset > [preset list]
- ---
- Duplicate project workspace
- Open project folder

### Pane-Local Tab Context Menu

Right-click on a pane tab:

- Close tab `Ctrl+W`
- Close other tabs
- Close tabs to the right
- ---
- Pin tab | Unpin tab
- Move tab to > [pane list]
- Duplicate tab in new pane
- ---
- Pop out this tab as new pane
- Copy tab identifier

### Pane Header Context Menu

Right-click on the pane header (not on a tab):

- Split right
- Split below
- Close pane
- ---
- Lock layout | Unlock layout
- Lock content | Unlock content
- Lock close | Unlock close
- Lock to project | Unlock from project
- Lock authority | Unlock authority
- ---
- Pop out pane
- Copy pane state JSON

### Editor Body Context Menu

Right-click inside a rich-text or code pane:

- Cut | Copy | Paste
- Select All
- ---
- Format Selection
- Toggle Comment
- ---
- Go to Definition
- Find References
- Peek Definition
- ---
- Add to Stash (stow selection to bottom drawer)
- Copy as Loom Block reference
- Open in new pane
- ---
- Fold | Unfold (code pane only)
- Format Document (code pane only)

### File / Project Tree Row Context Menu

Right-click on a file or folder row in the left rail:

- Open in active pane
- Open in new pane (split right)
- Open in new pane (split below)
- Open in pop-out
- ---
- Copy path
- Copy relative path
- Reveal in file system
- ---
- Rename...
- Delete... (with confirmation)
- ---
- Add to Stash
- Copy Loom Block reference (if file is indexed in Loom)

### Canvas / Loom Node Context Menu

Right-click on a node in a canvas or Loom graph view:

- Open block
- Open in new pane
- ---
- Copy block id
- Edit tags...
- Pin | Unpin
- ---
- Add connection to...
- Remove node from canvas
- ---
- Send to Stage
- Add to Stash
- Attach to validation evidence

### Bottom Drawer Item Context Menu

Right-click on a stash card:

- Open
- Pin | Unpin
- ---
- Send to pane > [pane list]
- Copy into prompt / context
- Attach to validation evidence
- Convert to artifact
- Promote (promote to project state)
- ---
- Discard

### Status Bar Segment Context Menu

Right-click on a status bar segment:

- Copy segment text
- Open related panel (e.g. "Open Jobs panel" from jobs segment)
- Toggle segment visibility

### Console / List Row Context Menu (Problems, Jobs, Timeline)

Right-click on a row:

- Copy row text
- Copy row as JSON
- ---
- Open related pane
- Jump to source (if file-linked)
- ---
- Add to Stash
- Attach to evidence

### Source Control Change Row Context Menu

Right-click on a changed file row in the Source Control pane:

- Open file
- Open diff
- Stage change | Unstage change
- Discard change (with confirmation)
- Copy file path

</topic>

<topic id="bottom-search-and-drawer" status="active" version="1" summary="Bottom search rail scopes and behavior; bottom drawer categories, item types, and actions." updated_at="2026-06-19">

## Bottom Search and Drawer

The bottom zone is two stacked sub-rows. They are siblings, not nested. The search rail is always visible; the drawer shelf is toggleable.

### Bottom Search Rail

A single persistent input with a scope selector and result area that appears above the input when active.

Scopes (selectable via `scope:` prefix or a scope picker to the left of the input):

| Scope | What it searches |
|-------|-----------------|
| `project:` | Project names, project metadata |
| `file:` | Files in the active project tree (full-text filename match) |
| `pane:` | Open pane names, pane types, active content titles |
| `window:` | All open panes across all projects |
| `stash:` | Bottom drawer stash items (text search within item content) |
| `trace:` | Flight Recorder events |
| `terminal:` | Terminal output blocks |
| `stage:` | Stage captures |
| `layout:` | Saved layout presets |
| `loom:` | Full Loom Search V2 (delegates to the `loom-search` pane surface) |
| (default, no scope prefix) | All of the above, blended |

Result appearance: a floating panel above the search rail, max ~300px tall, showing typed result rows (scope badge + name + preview). Keyboard: `Arrow Up/Down` navigate results; `Enter` opens; `Escape` dismisses.

The search input is accessible as `AccessKit role: SearchBox, id: "bottom-search-input"`.

### Bottom Drawer Shelf

Toggle affordance: a 1-row label bar labeled `BOTTOM DRAWER` with a right-aligned `+` button. Clicking the label row collapses/expands the shelf. The `+` adds a new stash item without expanding.

When expanded, the shelf shows:

- **Category tabs** (horizontal): `STASH` `AGENDA` `MAIL` `LISTS` `NOTES`. Selecting a category filters the card row.
- **Summary column** (left, narrow): count per type within the active category (e.g. TEXT: 0005, STAGE: 0002, TERM: 0004, JSON: 0003).
- **Card row** (horizontal scrolling): typed item cards side by side.

Item card anatomy:

```
[ TYPE-BADGE ]  [ STATUS-BADGE ]
Title / content preview (2-3 lines)
[ primary-action ]  [ secondary-action ]
```

- TYPE-BADGE: `TEXT` `STAGE` `TERM` `JSON` `PROMPT` `ARTIFACT` `FILE` `EVIDENCE` etc.
- STATUS-BADGE: `DRAFT` `CAPTURE` `BLOCK` `LAYOUT` `PINNED` etc.
- Primary/secondary actions vary by type: `SEND` `PROMOTE` `OPEN` `PIN` `COPY` `ATTACH` `VIEW` `DISCARD`.

Available actions on drawer items:

| Action | Effect |
|--------|--------|
| Stow | Add current selection to stash |
| Pin | Prevent item from being discarded by auto-cleanup |
| Promote | Convert item to project state (document, artifact, etc.) |
| Send to pane | Open item in a specified pane |
| Copy to prompt | Copy item content into the active prompt fragment |
| Attach to evidence | Link item as validation evidence |
| Convert to artifact | Wrap item as a formal Handshake artifact |
| Discard | Delete item (confirm if pinned) |

Drawer items are transient by design. They are work-in-progress staging material. They do not become authoritative state until Promoted.

Drawer shelf height: operator-resizable (drag the top edge of the label bar). Min collapsed height: 1 row (label bar only). Default expanded height: approx. 180px (fits 3 card action rows).

</topic>

<topic id="theming" status="active" version="1" summary="Dark and light themes: palette tokens, syntax colors, component rules." updated_at="2026-06-19">

## Theming

Two themes ship: **Dark** (default) and **Light**. Both are first-class; Light is not a toggled inversion. System theme detection is a future extension point.

### Dark Theme Palette

| Token | Purpose | Approximate value |
|-------|---------|------------------|
| `bg-base` | Main app background | `#0e0e0e` (near-black) |
| `bg-surface` | Pane/panel backgrounds | `#141414` |
| `bg-elevated` | Dropdowns, popups, dialogs | `#1c1c1c` |
| `bg-selected` | Active/selected rows | `#202020` |
| `border-default` | Grid lines, pane borders, separators | `#2a2a2a` |
| `border-active` | Active pane border | `#3a3a3a` |
| `text-primary` | Main readable text | `#d4d4d4` |
| `text-secondary` | Labels, metadata, dim UI text | `#7a7a7a` |
| `text-placeholder` | Placeholder / hint text | `#4a4a4a` |
| `accent` | Active markers, highlights, focus rings | `#c07040` (muted burnt orange) |
| `accent-dim` | Inactive accent / hover | `#7a4a28` |
| `ok` | Success, clean states | `#5a7a50` (muted green) |
| `warning` | Warnings | `#9a6a30` (muted ochre) |
| `error` | Errors, dirty/broken states | `#8a3030` (muted rust) |
| `scrollbar-rail` | Rail background | matches `bg-surface` |
| `scrollbar-knob` | Knob idle | `#2e2e2e` |
| `scrollbar-knob-hover` | Knob hover | `#444444` |
| `scrollbar-knob-grab` | Knob grab | `#c07040` (accent) |

### Light Theme Palette

| Token | Purpose | Approximate value |
|-------|---------|------------------|
| `bg-base` | Main app background | `#f0ede8` (warm off-white) |
| `bg-surface` | Pane/panel backgrounds | `#faf8f5` |
| `bg-elevated` | Dropdowns, popups, dialogs | `#ffffff` |
| `bg-selected` | Active/selected rows | `#e8e4de` |
| `border-default` | Grid lines | `#c8c4bc` |
| `border-active` | Active pane border | `#a09890` |
| `text-primary` | Main readable text | `#1a1a1a` |
| `text-secondary` | Labels, metadata | `#5a5450` |
| `text-placeholder` | Placeholder | `#9a9490` |
| `accent` | Active markers | `#9a4a18` (darker burnt orange for contrast) |
| `accent-dim` | Inactive accent / hover | `#c07040` |
| `ok` | Success | `#3a6030` |
| `warning` | Warnings | `#7a5020` |
| `error` | Errors | `#7a2020` |

### Syntax Color Tokens (Muted Palette — Both Themes)

The syntax palette is intentionally restrained. No neon. All colors are desaturated equivalents of conventional code palettes.

| Role | Dark | Light |
|------|------|-------|
| Keys / identifiers | `#8a9aaa` (dusty slate) | `#3a5060` |
| Strings | `#7a8a60` (muted olive) | `#3a5030` |
| Numbers | `#a08040` (muted ochre) | `#7a5820` |
| Booleans / keywords | `#9a5050` (muted rust) | `#6a2828` |
| Comments / metadata | `#5a5a5a` (warm gray) | `#8a8480` |
| Types / classes | `#7a7aaa` (dusty slate-blue) | `#404080` |
| Functions | `#a08060` (muted tan) | `#6a4820` |
| Errors / warnings | `#9a4040` / `#9a7040` | `#7a2020` / `#7a5010` |

Operators, punctuation, and brackets inherit `text-secondary` and do not get a distinct color unless specifically needed for a pane type.

### Theming Rules

- Theme tokens must be applied via a token system (Rust const map or a design-token file), not hardcoded hex in widget code.
- Every widget reads a token; no widget hardcodes a color value.
- Theme switches apply without restart (live token swap, immediate re-render).
- Light theme must pass WCAG AA contrast on all text/background pairings.
- Dark theme must pass WCAG AA on all text/background pairings.
- Syntax tokens may dip below WCAG AA only for comments and metadata; all other syntax roles must pass AA.

### Typography

- UI chrome: `IBM Plex Sans Condensed` (bundled; weights 400/500/600/700). Fallback: system condensed sans.
- Code / terminal / JSON panes: `IBM Plex Mono` or a bundled monospace. Fallback: system monospace.
- Paper labels in file drawer: same mono or narrow sans, compact line height.
- No oversized hero text inside the product UI.
- Base UI font size: 13px (operator-adjustable via Settings).
- Code editor font size: 13px by default, independent operator setting.

</topic>

<topic id="behavior-interaction" status="active" version="1" summary="Resize, split, tab-drag, pop-out, keyboard, focus rules, and QUIET compliance." updated_at="2026-06-19">

## Behavior and Interaction

### Focus Rules (HBR-QUIET)

- The app never steals OS focus when a swarm agent performs an action.
- Pop-out windows do not auto-focus on creation.
- Automated model-driven actions do not raise the window above the operator's foreground window.
- All model-driven interactions that require focus (e.g. simulating keystrokes) must acquire a named lease first; the shell can reject the lease if the operator has a modal open.
- Focus within the app: clicking a pane focuses it; keyboard pane navigation (`Ctrl+Alt+Arrow`) focuses without mouse. Tab key moves focus within the active pane in standard order (header controls, tab strip, content, scrollbar).

### Keyboard Shortcuts Summary

| Action | Shortcut |
|--------|---------|
| Command Palette | `Ctrl+Shift+P` |
| Quick Switcher | `Ctrl+P` |
| Open Terminal | Ctrl+Backtick |
| Find | `Ctrl+F` |
| Find in Files | `Ctrl+Shift+F` |
| Save | `Ctrl+S` |
| Save All | `Ctrl+Shift+S` |
| New Document | `Ctrl+N` |
| Close Tab | `Ctrl+W` |
| Settings | `Ctrl+,` |
| Toggle Left Rail | `Ctrl+B` |
| Toggle Bottom Drawer | `Ctrl+J` |
| Split Pane Horizontal | `Ctrl+\` |
| Split Pane Vertical | `Ctrl+Shift+\` |
| Pop Out Pane | `Ctrl+Shift+M` |
| Cycle Project Tabs | `Ctrl+Tab` / `Ctrl+Shift+Tab` |
| Jump Project Tab by Index | `Ctrl+1..9` |
| Cycle Pane Tabs | `Ctrl+PgUp/PgDn` |
| Navigate Panes | `Ctrl+Alt+Arrow` |
| Resize Pane (step) | `Ctrl+Shift+Arrow` |
| Navigate Back / Forward | `Alt+Left` / `Alt+Right` |
| Undo / Redo | `Ctrl+Z` / `Ctrl+Y` |
| Full Screen | `F11` |
| Help | `F1` |
| Build | `F5` |
| Next Diagnostic | `F8` |

All keybindings are rebindable via the Settings > Keybindings panel. The default map is the starting point for the `workspaceSettings.keybindings` structure from the legacy React app.

### Drag and Drop Summary

| Gesture | Effect |
|---------|--------|
| Drag pane-local tab left/right within strip | Reorder tab |
| Drag pane-local tab to another pane's header | Move tab to that pane |
| Drag pane-local tab to empty edge | Create new pane region |
| Drag file from project tree to pane | Open file in that pane |
| Drag stash card to a pane | Open stash item in that pane |
| Drag CKC media/character to a note pane | Embed as Loom block reference |
| Drag pane pop-out window back to main | Merge back into grid |

### Resize Behavior

- Pane resizing is via splitter drag. Minimum sizes are enforced.
- When a pane is resized below minimum, it collapses to the minimum; adjacent panes expand.
- Layout snapshots record split weights as floats (0.0–1.0) normalized per split axis.
- `layout_lock` prevents resize by any actor.
- Keyboard resize step: 5% of the split axis length per keypress.

### Undo Scope

- Editor-level undo (`Ctrl+Z`) undoes within the active document in the active pane.
- Layout undo is not in scope for WP-011; layout changes are immediately persisted (no undo).
- Future: one shared undo stack across editor surfaces (WP-012 interconnection contract).

### Loading States

- On project switch: pane grid shows a skeleton/placeholder while layout hydrates from backend.
- On layout restore failure: the grid shows a clear error state with a "Reset to default layout" affordance.
- On pane content loading: pane body shows a spinner with the content id.
- Status bar always reflects current state (LOADING / READY / ERROR).

</topic>

<topic id="agent-vision-and-steering" status="active" version="1" summary="How LLM swarm agents see and steer the work surface via AccessKit, MCP tools, and the screenshot path." updated_at="2026-06-19">

## Agent Vision and Steering

Handshake is designed for human + swarm co-creation. Every interactive widget must be visible and steerable by an LLM agent without screen-reading guesswork. This is enforced by the AccessKit contract in WP-011 Cluster C7 and the model accessibility contract in the WP-011 packet.

### AccessKit Requirements

Every interactive widget sets:

- `author_id`: stable logical string id derived from an id registry (e.g. `"pane-a.tab.main-journal"`, `"bottom-search-input"`, `"module-switcher.CKC"`). NOT frame counters.
- `role`: AccessKit role matching the widget type (e.g. `Tab`, `Button`, `TextInput`, `Tree`, `TreeItem`, `ScrollBar`).
- `name` / `label`: human-readable description.
- `value`: current value where applicable (active tab name, input text, toggle state).
- `supported_actions`: list of actions the agent can dispatch (`Click`, `Focus`, `SetValue`, `Select`, `Scroll`).

Non-interactive display widgets (labels, static text, icons) set a role but no actions.

### UI Tree Snapshot

An in-process AccessKit consumer serializes the full UI tree to JSON on demand. Format:

```json
{
  "snapshot_at": "2026-06-19T10:00:00Z",
  "root": {
    "id": "app-root",
    "role": "Window",
    "name": "Handshake",
    "children": [
      {
        "id": "project-tab-row",
        "role": "TabList",
        "children": [
          { "id": "project-tab.handshake", "role": "Tab", "name": "Handshake", "value": "selected", "actions": ["Click", "Focus"] },
          { "id": "project-tab.fast-focus", "role": "Tab", "name": "Fast Focus", "value": "", "actions": ["Click", "Focus"] }
        ]
      },
      {
        "id": "module-switcher",
        "role": "ToolBar",
        "children": [
          { "id": "module-switcher.MAIN", "role": "Button", "name": "MAIN", "value": "active", "actions": ["Click"] }
        ]
      },
      {
        "id": "pane-grid",
        "role": "Group",
        "children": [ /* pane subtrees */ ]
      }
    ]
  }
}
```

### Action Channel

An agent dispatches actions via a structured channel:

```json
{ "op": "click", "target": "module-switcher.CKC" }
{ "op": "set_value", "target": "bottom-search-input", "data": "project:handshake" }
{ "op": "focus", "target": "pane-a.tab.main-journal" }
{ "op": "scroll", "target": "pane-a.content", "data": { "delta_y": 100 } }
```

### MCP-Style Swarm Tools

Exposed on localhost / named pipe with a session token:

| Tool | Purpose |
|------|---------|
| `list_widgets` | Returns the full AccessKit tree snapshot as JSON |
| `click_widget` | Dispatches a Click action by stable id |
| `set_value` | Dispatches a SetValue action by stable id |
| `focus_widget` | Dispatches a Focus action by stable id |
| `screenshot` | Captures the current app window as PNG (windows-capture/xcap) |
| `get_pane_state` | Returns the JSON state of a named pane |
| `get_layout_snapshot` | Returns the full workspace layout JSON |
| `stow_selection` | Stows the current selection to the bottom drawer stash |
| `dispatch_command` | Executes a named command from the command registry |

### Agent Co-Creation Patterns

These are design intentions, not constraints on MT scope:

- An agent can open a project, navigate to a module, open a document pane, and author text entirely via the MCP tools.
- An agent can observe the terminal pane via `list_widgets` (terminal output lines exposed as tree items with their text).
- An agent can read a JSON pane's content by targeting its content tree nodes.
- An agent can stash a selection, promote it to a document, and send it to Stage — all via structured actions with no screen reading.
- Multiple parallel agents acquire named leases before mutating shared state. The shell serializes conflicting mutations and attributes them.

### HBR Obligations on This Surface

| HBR Tag | Obligation |
|---------|-----------|
| HBR-VIS | Every widget exposes AccessKit tree with stable ids; screenshot capture path exists. |
| HBR-SWARM | MCP swarm tools are implemented; concurrency-safe under N agents + operator; actions are attributable. |
| HBR-QUIET | Agent actions never steal OS focus or hijack keyboard; focus-audit must pass. |
| HBR-INT | All agent interactions are logged to the Flight Recorder event ledger. |
| HBR-MAN | The built-in User Manual pane documents every agent tool and stable id pattern. |
| HBR-STOP | Agent activity stops cleanly on `TaskStop` signal; no orphaned state. |

</topic>

<topic id="pane-type-extension-points" status="active" version="1" summary="How future creative surfaces plug in as designed-in extension points without changing shell code." updated_at="2026-06-19">

## Extension Points for Future Surfaces

These surfaces are **not built** in WP-011 or WP-012. They are listed here so MT authors know where extension seams must exist.

| Future surface | Extension seam required |
|----------------|------------------------|
| Photo / image editor (Photoshop/Affinity class) | Pane type `photo-editor`; a custom **wgpu 2D canvas host** (de-risked by WP-011 MT-001 probe e); model-steering via a parametric **non-destructive layer + adjustment + vector** document (model edits parameters; the raster is a deterministic render), NOT raw-pixel manipulation; AccessKit for the tool/panel chrome; same block/selection/event/accessibility substrate as editors (WP-012 interconnection). |
| **Tailor — native Marvelous Designer / cloth-garment engine (WP-KERNEL-010)** | Pane type `tailor` (3D garment viewport) on the shell's custom **wgpu viewport host** (de-risked by WP-011 MT-001 probe e); model-steering via the parametric **GarmentSpec** document (pattern pieces + seams + fabric + measurements) rendered/simulated mechanically by a **deterministic cloth solver** (`tailor-solver`, XPBD on wgpu/WGSL), NOT vertex manipulation; AccessKit for the panel/tool chrome; viewport/sim state exposed as model-readable visual capture (HBR-VIS); outputs glTF/USD geometry caches. Already specced in Master Spec §13 with 448 pre-created MTs; built later, gated on KERNEL-001..004. |
| Spreadsheet | Pane type `spreadsheet` in registry; same parametric-doc + wgpu-host substrate. |
| Render engine (ComfyUI integration) | Pane type `render-engine`; route captures to Stage pane and bottom drawer. |
| In-app update channel | Designed-in seam in the installer scaffold (WP-011 MT-004). |
| Custom syntax themes | Syntax token map accepts a named custom entry; palette editor in Settings > Appearance. |
| OS tray / notifications | Designed seam; no UI built yet. |

The pane registry, the command registry, the event ledger, the AccessKit id registry, and the shared selection/undo substrate are the five extension surfaces every future pane type plugs into. None of these should be coded in a way that requires forking for a new pane type.

</topic>

<topic id="legacy-react-contrast" status="active" version="1" summary="What exists today in the React app and what the native GUI replaces or preserves." updated_at="2026-06-19">

## Contrast with the Legacy React App

The legacy React app lives at `handshake_main/app/src/App.tsx` and related components. It is kept as a read-only reference and parity source during the native build. This section documents what exists today and what changes.

### What Exists in React (keep as parity reference)

- `ModuleId`: MAIN, CKC, INGEST, STAGE, LAB, STUDIO (preserve exactly).
- `PaneTabId` registry: 19 tab types (workspace, media-downloader, fonts, flight-recorder, kernel-dcc, inference-lab, model-runtime, swarm, problems, jobs, timeline, user-manual, code-symbol, source-control, loom-daily-journal, loom-block, loom-wiki-page, atelier, visual-debugger). All must be re-homed as native pane types.
- `PaneId`: pane-a, pane-b, pane-c, pane-d (4 fixed panes today; native replaces with dynamic docking).
- Layout persistence: `getWorkbenchLayoutState` / `saveWorkbenchLayoutState` backend APIs. Native must reuse these same APIs.
- Workspace settings: `getWorkspaceSettingsState` / `saveWorkspaceSettingsState`. Reuse.
- Document open/close/pin/dirty/move-between-panes logic (all replicated in native).
- `listWorkspaces` API for project list. Reuse.
- `CommandPalette`, `QuickSwitcher`, `WorkspaceSearchPanel`, `LoomSearchV2Panel` components: all become native panes/surfaces.
- `AiJobsDrawer`, `EvidenceDrawer`, `EvidenceSelection`: replicated in native.
- `SettingsMenu`: replaced by the native Settings dialog described in this doc.
- `SystemStatus`: replicated in native status bar.
- `ViewModeToggle`: merged into native theme switcher in status bar or settings.
- `FlightRecorderView`, `VisualDebuggerPanel`, `DebugPanel`: replicated as native pane types.

### What the Native GUI Adds That React Does Not Have

- True dockable/tileable pane grid (React has a fixed 2x2 grid with manual CSS split, not a proper docking engine).
- Context menus (none in React today).
- Pop-out panes into OS windows.
- Per-project persisted layout presets.
- Five lock types on panes.
- Bottom drawer as typed stash shelf with card UI (React has a drawer toggle but no typed card surface).
- AccessKit model surface (zero model-vision support in React today).
- Full native menus (FILE/EDIT/VIEW/GO/RUN/HELP).
- Dark + Light themes as first-class (React has ViewMode toggle but limited theming).
- Single-binary native installer (React runs in Tauri webview).

### React Source File Map for MT Authors

| Native surface | Primary React source to port from |
|----------------|----------------------------------|
| Project tab row | `App.tsx` (projects state, selectProject fn) |
| Module switcher | `App.tsx` (MODULE_DEFINITIONS, activeModule state) |
| Pane local tab strip | `App.tsx` (PaneState, setActiveTabForPane, tab drag/drop) |
| Pane content router | `App.tsx` (renderPaneContent or equivalent switch) |
| Workspace/document pane | `components/DocumentView.tsx` |
| Canvas/Loom pane | `components/CanvasView.tsx` |
| Sidebar | `components/WorkspaceSidebar.tsx` |
| Command palette | `components/CommandPalette.tsx` + `lib/app_command_registry.ts` |
| Quick switcher | `components/QuickSwitcher.tsx` |
| Loom Search V2 | `components/LoomSearchV2Panel.tsx` |
| Workspace search | `components/WorkspaceSearchPanel.tsx` |
| Settings | `components/SettingsMenu.tsx` + `lib/workspaceSettings.ts` |
| System status bar | `components/SystemStatus.tsx` |
| Flight Recorder | `components/FlightRecorderView.tsx` |
| Visual Debugger | `components/visual_debugger/` |
| Swarm surface | `components/swarm/` (SwarmOperatorSurface) |
| Atelier | `components/AtelierPanel.tsx` |
| Kernel DCC | `components/KernelDccProjectionView.tsx` |
| Inference Lab | `components/inference_lab/` |
| Model Runtime | `components/model_runtime_panel/` |
| Source Control | `components/SourceControlPanel.tsx` |
| Loom panels | `components/LoomBlockPanel.tsx`, `LoomDailyJournalPanel.tsx`, `LoomWikiPagePanel.tsx`, `LoomAiReviewPanel.tsx` |
| Jobs/Problems/Timeline | `components/operator/` (JobsView, ProblemsView, TimelineView) |
| Evidence Drawer | `components/operator/EvidenceDrawer.tsx` |
| Backend API bindings | `app/src/lib/api.ts` (all Tauri invoke wrappers; native replaces invoke with IPC to handshake_core) |

</topic>

<topic id="open-questions" status="active" version="1" summary="Design questions not yet resolved; MT authors should raise these as blockers if they hit them." updated_at="2026-06-19">

## Open Questions

These are unresolved before MT execution begins. MT authors should not block on them but should raise them as typed blockers if the question becomes critical to an MT.

1. Should panes be allowed to float freely on day one (WP-011), or is tiled-only with pop-out-to-OS-window the day-one model? (Current answer implied by WP-011 scope: tiled + pop-out; no freeform float.)
2. Is the bottom drawer global (persists across project switches) or project-scoped (resets on project switch)? (Current answer: project-scoped; separate shelves per project.)
3. Should the Light theme palette lean warm off-white (paper direction) or neutral white (clean modern)? Operator has not specified; warm off-white is assumed.
4. Syntax theme: should there be a "Standard" (VSCode-style saturated) option from day one, or is Muted the only option in WP-011/012 scope? (Current answer: Muted only; Standard is extension point.)
5. Should the file drawer paper-label treatment apply globally, or only to the project tree in the left rail? (Current answer: left rail project tree only.)
6. What is the exact named-pipe or localhost port for the MCP swarm tool surface? (Decided by WP-011 MT-025/026 implementation; document here when resolved.)
7. How many layout presets ship as built-in defaults? (At minimum: Default 2x2, Editor-focused 1+1, Terminal-heavy, and Canvas-only. Exact set is MT-009 scope.)

</topic>
