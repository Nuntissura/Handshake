---
file_id: studio-app-feature-research-command-shortcut-capture
topic_id: SFR-CMDCAP
title: "Command / Shortcut / Scripting-DOM Capture (2026-07-20, ACTION-A3)"
status: draft
summary: "STRUCTURE + representative-sample capture of default shortcuts, Illustrator menu-command IDs, and InDesign scripting DOM from PUBLIC ONLINE sources (no vendor apps). Partial: canonical Adobe SPA pages time out for non-browser clients; full binding tables remain a browser-fetch residual."
sources: 82
updated_at: "2026-07-20"
---


## [SFR-CMDCAP] Command / Shortcut / Scripting-DOM Capture

### [SFR-CMDCAP.summary] Summary

```json
{
  "action": "ACTION-A3, replanned as online-source capture (operator has no vendor apps/subscriptions; installed-export path retired).",
  "coverage_meaning": {
    "FULL_LIST_ON_PAGE": "the source page carries the complete enumeration",
    "REPRESENTATIVE_SAMPLE": "exemplar entries verified verbatim from a reachable source; not exhaustive",
    "INDEX_ONLY": "canonical enumeration source identified (URL + category structure) but full table not transcribed this pass"
  },
  "total_groups": 82,
  "by_lane": {
    "photoshop/shortcuts": 11,
    "illustrator/menu_command_ids": 12,
    "illustrator/shortcuts": 11,
    "indesign/scripting_dom": 14,
    "indesign/shortcuts": 13,
    "affinity/shortcuts": 7,
    "figma/shortcuts": 14
  },
  "fetch_blocker": "The canonical Adobe helpx default-keyboard-shortcuts pages render as slow AEM/JS SPAs that time out for all non-browser clients (WebFetch 60s x4, curl exit 28, Invoke-WebRequest 90s); r.jina.ai returned 422; web.archive.org blocked in this environment. So Adobe shortcut lanes are INDEX_ONLY + REPRESENTATIVE_SAMPLE from third-party cheat-sheets, not verbatim full tables. Illustrator executeMenuCommand: the docsforadobe Application page documents the method but does not enumerate the ID catalog (community/SDK-maintained). Ties to SFR-REMAINING-GAP-003.",
  "residual": "Full verbatim binding tables + the complete executeMenuCommand ID catalog need a browser-capable fetch (or the SFR-REMAINING-GAP-003 browser-export fallback). This capture gives the deterministic source URLs + category structure a later browser pass can complete.",
  "authority": "Reference/provenance only."
}
```

### [SFR-CMDCAP.photoshop-shortcuts] photoshop / shortcuts (11 groups)

```json
{
  "groups": [
    {
      "group": "CANONICAL INDEX — Adobe helpx Default keyboard shortcuts (full page)",
      "source_url": "https://helpx.adobe.com/photoshop/using/default-keyboard-shortcuts.html",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [
        "Popular shortcuts",
        "Keys for selecting tools",
        "Keys for viewing images",
        "Keys for painting",
        "Keys for using blending modes",
        "Keys for selecting and moving objects",
        "Keys for transforming selections, selection borders, and paths",
        "Keys for editing paths",
        "Keys for selecting and editing text",
        "Keys for formatting type",
        "Keys for panels"
      ],
      "notes": "This is the operator-requested canonical enumeration source. The page renders as a slow AEM/JS SPA that hangs all non-browser clients: WebFetch timed out (60s) on 4 attempts, curl.exe exited 28 (timeout), and PowerShell Invoke-WebRequest timed out at 90s. Full per-binding tables were NOT transcribed here. The category/section names listed as exemplars are the well-documented H2 structure of this page but were NOT verified verbatim in this pass (Adobe page unreachable) — treat as UNVERIFIED heading labels pending a deterministic re-fetch. A later pass with a browser-capable fetch can pull the complete binding tables from this exact URL. No per-category counts are stated on this page. web.archive.org is blocked in this environment; r.jina.ai reader returned HTTP 422.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-01"
    },
    {
      "group": "Popular / Must-Know shortcuts",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Save As: Ctrl+Shift+S (Win) | Command+Shift+S (Mac)",
        "Undo: Ctrl+Z | Command+Z",
        "Step Back: Ctrl+Alt+Z | Command+Option+Z",
        "Duplicate (layer/selection): Ctrl+J | Command+J",
        "Default Colors: D | D",
        "Flip FG/BG Colors: X | X",
        "Free Transform: Ctrl+T | Command+T",
        "Fit to Screen: Ctrl+0 | Command+0"
      ],
      "notes": "Verified verbatim from PSTC reproduction. Mirrors Adobe's 'Popular shortcuts' section. Exemplars only; not exhaustive.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-02"
    },
    {
      "group": "Tools (Toolbar selection)",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Move Tool: V | V",
        "Marquee Selection: M | M",
        "Lasso Tool: L | L",
        "Magic Wand: W | W",
        "Crop Tool: C | C",
        "Brush Tool: B | B",
        "Eraser Tool: E | E",
        "Pen Tool: P | P",
        "Zoom Tool: Z | Z"
      ],
      "notes": "Corresponds to Adobe 'Keys for selecting tools'. Single-letter tool bindings verified across PSTC and academyclass.com. Full tool list (~60+ tool/cycle bindings) is on the canonical Adobe page.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-03"
    },
    {
      "group": "Painting",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Fill with Foreground: Alt+Backspace | Option+Delete",
        "Fill with Background: Ctrl+Backspace | Command+Delete",
        "Decrease Brush Size: [ | [",
        "Increase Brush Size: ] | ]",
        "Decrease Brush Hardness: Shift+[ | Shift+[",
        "Increase Brush Hardness: Shift+] | Shift+]",
        "Set opacity 10% increments: 1..0 | 1..0"
      ],
      "notes": "Corresponds to Adobe 'Keys for painting'. Verified verbatim from PSTC (opacity increments cross-checked with academyclass.com).",
      "id": "SFR-CMDCAP-photoshop-shortcuts-04"
    },
    {
      "group": "Selecting / Selections",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Select All: Ctrl+A | Command+A",
        "Deselect: Ctrl+D | Command+D",
        "Inverse: Shift+Ctrl+I | Shift+Command+I",
        "Feather Selection: Shift+F6 | Shift+F6"
      ],
      "notes": "Corresponds to Adobe 'Keys for selecting and moving objects'. Verified from PSTC; Select All/Deselect/Inverse cross-confirmed on academyclass.com.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-05"
    },
    {
      "group": "Type / Text",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Align Left: Ctrl+Shift+L | Command+Shift+L",
        "Align Center: Ctrl+Shift+C | Command+Shift+C",
        "Bold: Ctrl+Shift+B | Command+Shift+B",
        "Italic: Ctrl+Shift+I | Command+Shift+I"
      ],
      "notes": "Corresponds to Adobe 'Keys for selecting and editing text' + 'Keys for formatting type'. Verified verbatim from PSTC. Full type set (leading/kerning/tracking/baseline nudges) is on the canonical Adobe page.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-06"
    },
    {
      "group": "Blending modes",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Multiply: Shift+Alt+M | Shift+Option+M",
        "Screen: Shift+Alt+S | Shift+Option+S",
        "Overlay: Shift+Alt+O | Shift+Option+O",
        "Soft Light: Shift+Alt+F | Shift+Option+F"
      ],
      "notes": "Corresponds to Adobe 'Keys for using blending modes'. Verified verbatim from PSTC. Full blend-mode set (~25 modes: Normal/Dissolve/Darken/Lighten/Color Dodge/Burn/etc.) is on the canonical Adobe page.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-07"
    },
    {
      "group": "Layers",
      "source_url": "https://photoshoptrainingchannel.com/photoshop-keyboard-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Activate Layer Below: Alt+[ | Option+[",
        "Activate Layer Above: Alt+] | Option+]",
        "Move Layer Down: Ctrl+[ | Command+[",
        "Move Layer Up: Ctrl+] | Command+]",
        "New Layer (dialog): Ctrl+Shift+N | Command+Shift+N",
        "Duplicate Layer: Ctrl+J | Command+J",
        "Merge Selected: Ctrl+E | Command+E",
        "Group Layers: Ctrl+G | Command+G"
      ],
      "notes": "Layer navigation/move bindings verified from PSTC; New/Duplicate/Merge/Group cross-confirmed on academyclass.com. Layer-menu bindings live under Adobe's Application menu (Layer) shortcut tables.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-08"
    },
    {
      "group": "Viewing / Navigation",
      "source_url": "https://academyclass.com/blog/photoshop-keyboard-shortcuts-cheat-sheet/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Zoom In: Ctrl+= | Command+=",
        "Fit to Screen: Ctrl+0 | Command+0",
        "Show/Hide Rulers: Ctrl+R | Command+R"
      ],
      "notes": "Corresponds to Adobe 'Keys for viewing images'. Verified verbatim from academyclass.com.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-09"
    },
    {
      "group": "Image Adjustments",
      "source_url": "https://academyclass.com/blog/photoshop-keyboard-shortcuts-cheat-sheet/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Levels: Ctrl+L | Command+L",
        "Curves: Ctrl+M | Command+M",
        "Hue/Saturation: Ctrl+U | Command+U"
      ],
      "notes": "These map to Adobe's Application menu (Image > Adjustments) shortcut table rather than a standalone helpx section. Verified verbatim from academyclass.com.",
      "id": "SFR-CMDCAP-photoshop-shortcuts-10"
    },
    {
      "group": "Transform",
      "source_url": "https://academyclass.com/blog/photoshop-keyboard-shortcuts-cheat-sheet/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Free Transform: Ctrl+T | Command+T",
        "Increase Brush Size (transform-adjacent): ] | ]",
        "New Document: Ctrl+N | Command+N",
        "Open: Ctrl+O | Command+O",
        "Save: Ctrl+S | Command+S"
      ],
      "notes": "Corresponds to Adobe 'Keys for transforming selections, selection borders, and paths'. Free Transform verified on academyclass.com and PSTC. File commands included for context; academyclass.com states the app has 'over 500 built-in keyboard shortcuts' (secondary claim, NOT stated by Adobe on the canonical page — treat count as unverified).",
      "id": "SFR-CMDCAP-photoshop-shortcuts-11"
    }
  ]
}
```

### [SFR-CMDCAP.illustrator-menu_command_ids] illustrator / menu_command_ids (12 groups)

```json
{
  "groups": [
    {
      "group": "executeMenuCommand — enumeration source / catalog overview",
      "source_url": "https://ai-scripting.docsforadobe.dev/jsobjref/Application/",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [
        "app.executeMenuCommand(commandString)",
        "app.getMenuTitles()",
        "app.getMenuTitle(commandName, item)",
        "app.copy()",
        "app.cut()",
        "app.paste()",
        "app.redo()",
        "app.undo()"
      ],
      "notes": "docsforadobe Application page (jsobjref/Application/) is the canonical DOM home of app.executeMenuCommand(commandString) but the page marks itself 'work in progress' and does NOT itself enumerate the menu-command-ID strings. There is no single Menu-Commands page on ai-scripting.docsforadobe.dev; the ID catalog is community/SDK-maintained. Application object documents 26 properties and 34 methods (counts stated by page). No AI features covered.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-01"
    },
    {
      "group": "Menu-command-ID master list (SDK-extracted, community-maintained)",
      "source_url": "https://community.adobe.com/t5/illustrator-discussions/executemenucommand-command-list/td-p/13131490",
      "coverage": "INDEX_ONLY",
      "count": "506 (2017 SDK) / 530 (2022 Notion DB)",
      "exemplars": [
        "selectall",
        "deselectall",
        "group",
        "ungroup",
        "join",
        "outline",
        "transformagain",
        "Live Rasterize",
        "zoomin",
        "preview"
      ],
      "notes": "Counts STATED by community source: 506 commands extracted from the 2017 Illustrator SDK; a maintained Notion DB for Illustrator 2022 (26.4.1) lists 530 commands. ~90 of the 2017 commands no longer work in newer versions. This is the deterministic full-list fetch target; strings are cryptic/non-descriptive by design.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-02"
    },
    {
      "group": "File menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "new",
        "open",
        "save",
        "saveas",
        "export",
        "print",
        "quit"
      ],
      "notes": "Exemplar strings verbatim from gist mapping IDs to File:* menu paths. Not exhaustive; full File-menu set enumerated in the SDK/Notion catalog.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-03"
    },
    {
      "group": "Edit menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "undo",
        "redo",
        "cut",
        "copy",
        "paste",
        "clear",
        "preference",
        "color"
      ],
      "notes": "'preference' = Edit:Preferences:General; 'color' = Edit:Color Settings. Exemplars verbatim from gist.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-04"
    },
    {
      "group": "Object menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "transformagain",
        "transformmove",
        "transformrotate",
        "group",
        "ungroup",
        "lock",
        "unlockAll",
        "join"
      ],
      "notes": "'join' = Object:Path:Join; 'lock' = Object:Lock:Selection. One of the largest menu groups (Transform/Arrange/Path/Pathfinder submenus). Exemplars verbatim from gist.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-05"
    },
    {
      "group": "Type menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "outline",
        "fitHeadline",
        "type-horizontal",
        "type-vertical"
      ],
      "notes": "'outline' = Type:Create Outlines. Exemplars verbatim from gist.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-06"
    },
    {
      "group": "Select menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "selectall",
        "deselectall",
        "selectallinartboard"
      ],
      "notes": "'selectallinartboard' = Select:All on Active Artboard. Exemplars verbatim from gist.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-07"
    },
    {
      "group": "Effect menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Live 3DExtrude",
        "Live Pathfinder Add",
        "Live Rasterize"
      ],
      "notes": "Effect commands use a 'Live *' prefix convention. 'Live 3DExtrude' = Effect:3D:Extrude & Bevel. Exemplars verbatim from gist.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-08"
    },
    {
      "group": "View menu command IDs",
      "source_url": "https://gist.github.com/iconifyit/5c383d3fd01b3890e97a0e594290a764",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "preview",
        "zoomin",
        "zoomout",
        "showguide",
        "ruler"
      ],
      "notes": "'showguide' = View:Guides:Hide Guides; 'ruler' = View:Rulers:Show Rulers. Exemplars verbatim from gist.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-09"
    },
    {
      "group": "Scripting DOM — object classes (jsobjref index)",
      "source_url": "https://ai-scripting.docsforadobe.dev/jsobjref/javascript-object-reference/",
      "coverage": "FULL_LIST_ON_PAGE",
      "count": "unstated",
      "exemplars": [
        "Application",
        "Document",
        "Layer",
        "PageItem",
        "PathItem",
        "PathPoint",
        "CompoundPathItem",
        "GroupItem",
        "TextFrameItem",
        "TextRange",
        "Color / CMYKColor / RGBColor / SpotColor",
        "Swatch",
        "Artboard",
        "Preferences",
        "Matrix"
      ],
      "notes": "jsobjref index lists the full DOM class set: core (Application, Document(s), Artboard(s), Layer(s), PageItem(s)); shape/path (PathItem, PathPoint, CompoundPathItem, GroupItem, MeshItem, RasterItem, PlacedItem, SymbolItem, GraphItem); text (TextFrameItem, Story, TextRange, Characters, Paragraphs, Words, TextPath, TextFont); color/style (CMYK/RGB/Gray/Lab/Spot/Gradient/Pattern Color, Gradient, Brush, GraphicStyle, CharacterStyle, ParagraphStyle, Swatch, Spot, Pattern); save/export options (EPS/PDF/Illustrator/FXG SaveOptions, JPEG/PNG/GIF/SVG/TIFF/Photoshop/Flash ExportOptions); other (Variable, Dataset, Tag, Symbol, Preferences, View, Screen, Matrix, Ink).",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-10"
    },
    {
      "group": "Scripting DOM — Application object members",
      "source_url": "https://ai-scripting.docsforadobe.dev/jsobjref/Application/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "26 properties / 34 methods",
      "exemplars": [
        "activeDocument",
        "documents",
        "selection",
        "version",
        "preferences",
        "textFonts",
        "printerList",
        "open()",
        "executeMenuCommand()",
        "getRotationMatrix()",
        "getScaleMatrix()",
        "concatenateMatrix()",
        "copy()",
        "cut()",
        "paste()",
        "undo()",
        "redo()",
        "beep()",
        "saveWorkspace()",
        "switchWorkspace()"
      ],
      "notes": "Counts STATED by page: 26 properties, 34 methods. executeMenuCommand belongs here (per method signature app.executeMenuCommand(commandString)) though the WebFetch summary of this WIP page did not surface it explicitly.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-11"
    },
    {
      "group": "Scripting DOM — PathItem object members (exemplar leaf class)",
      "source_url": "https://ai-scripting.docsforadobe.dev/jsobjref/PathItem/",
      "coverage": "FULL_LIST_ON_PAGE",
      "count": "unstated",
      "exemplars": [
        "pathPoints",
        "closed",
        "filled",
        "fillColor",
        "stroked",
        "strokeColor",
        "strokeWidth",
        "area",
        "length",
        "geometricBounds",
        "setEntirePath()",
        "duplicate()",
        "move()",
        "resize()",
        "rotate()",
        "transform()",
        "zOrder()"
      ],
      "notes": "Representative leaf DOM class showing full member shape: ~50 properties (area, closed, filled, fillColor, stroked, strokeWidth, pathPoints, geometricBounds, opacity, position, zOrderPosition, etc.) and 8 methods (duplicate, move, remove, resize, rotate, setEntirePath, transform, translate, zOrder). Confirms per-object pages carry complete member lists for deterministic later fetch.",
      "id": "SFR-CMDCAP-illustrator-menu_command_ids-12"
    }
  ]
}
```

### [SFR-CMDCAP.illustrator-shortcuts] illustrator / shortcuts (11 groups)

```json
{
  "groups": [
    {
      "group": "Popular shortcuts",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Undo: Ctrl+Z",
        "Redo: Ctrl+Shift+Z",
        "Cut: Ctrl+X",
        "Copy: Ctrl+C",
        "Paste: Ctrl+V"
      ],
      "notes": "Per-category counts are not stated on the source page. Live helpx WebFetch timed out repeatedly (heavy JS page); category grouping and exemplars confirmed via KeyCombiner mirror (keycombiner.com/collections/illustrator/) which credits Adobe. Mac equivalents substitute Cmd for Ctrl.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-01"
    },
    {
      "group": "Work with documents",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Create a document: Ctrl+N",
        "Open a document: Ctrl+O",
        "Save changes made to the document: Ctrl+S",
        "Print: Ctrl+P",
        "Exit the application: Ctrl+Q"
      ],
      "notes": "File/document lifecycle commands.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-02"
    },
    {
      "group": "Select tools",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Selection tool: V",
        "Direct Selection tool: A",
        "Pen tool: P",
        "Type tool: T",
        "Rectangle tool: M",
        "Zoom tool: Z",
        "Hand tool: H"
      ],
      "notes": "Single-key toolbox activators.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-03"
    },
    {
      "group": "View artwork",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Toggle between screen modes: F",
        "Zoom in: Ctrl+=",
        "Zoom out: Ctrl+-",
        "Hide guides: Ctrl+;",
        "Show grid: Ctrl+'"
      ],
      "notes": "View, zoom, guides, grid, screen-mode navigation.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-04"
    },
    {
      "group": "Work with selections",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Select all: Ctrl+A",
        "Deselect: Ctrl+Shift+A",
        "Group the selected artwork: Ctrl+G",
        "Lock a selection: Ctrl+2",
        "Bring a selection forward: Ctrl+]"
      ],
      "notes": "Selection, grouping, locking, stacking-order operations.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-05"
    },
    {
      "group": "Draw",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Join two or more paths: Ctrl+J",
        "Create a compound path: Ctrl+8",
        "Switch through drawing modes: Shift+D"
      ],
      "notes": "Path drawing and compound-path commands.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-06"
    },
    {
      "group": "Edit shapes",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Create corner or smooth join: Ctrl+Alt+Shift+J",
        "Blend objects: Ctrl+Alt+B"
      ],
      "notes": "Anchor/segment editing and blend commands.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-07"
    },
    {
      "group": "Work with objects",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Toggle between fill and stroke: X",
        "Swap fill and stroke: Shift+X",
        "Make a clipping mask: Ctrl+7"
      ],
      "notes": "Fill/stroke, masking, object-level operations.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-08"
    },
    {
      "group": "Work with type",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Create outlines: Ctrl+Shift+O",
        "Open the Character panel: Ctrl+T",
        "Superscript: Ctrl+Shift+="
      ],
      "notes": "Type/character formatting commands.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-09"
    },
    {
      "group": "Use panels",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [],
      "notes": "Category exists on the source with multiple panel-toggle shortcuts; individual bindings not captured because live helpx fetch timed out and the mirror did not enumerate them. A later deterministic pass should fetch the 'Use panels' table from the helpx URL.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-10"
    },
    {
      "group": "Function keys",
      "source_url": "https://helpx.adobe.com/illustrator/using/default-keyboard-shortcuts.html",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [],
      "notes": "Category exists on the source listing F-key panel/tool toggles (e.g. Brushes, Color, Layers panels); individual bindings not captured to avoid fabrication. Fetch the 'Function keys' table from the helpx URL in a later pass.",
      "id": "SFR-CMDCAP-illustrator-shortcuts-11"
    }
  ]
}
```

### [SFR-CMDCAP.indesign-scripting_dom] indesign / scripting_dom (14 groups)

```json
{
  "groups": [
    {
      "group": "A-Z class-index enumeration source (overview)",
      "source_url": "https://www.indesignjs.de/extendscriptAPI/indesign-latest/",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [
        "Application",
        "Document",
        "Story",
        "Paragraph",
        "Table",
        "Cell",
        "PageItem",
        "TextFrame",
        "Page",
        "Spread"
      ],
      "notes": "Canonical Adobe InDesign 2026 (21.0.0.192) ExtendScript Object Model index; server-rendered alphabetical class list = deterministic full-enumeration anchor. The UXP DOM API (developer.adobe.com/indesign/uxp/dom/api/) exposes the SAME object model under first-letter URL namespaces, e.g. /dom/api/d/document/, /dom/api/s/story/. UXP DOM is versioned 3.0-21.0 mapped to app version. Source did NOT state a total class count; the '1,080-page' figure is a PDF page count, not a class count -> count left unstated (unverified).",
      "id": "SFR-CMDCAP-indesign-scripting_dom-01"
    },
    {
      "group": "A namespace (/dom/api/a/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/a/application/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Application",
        "Article",
        "Asset",
        "AnimationBehavior",
        "Assignment"
      ],
      "notes": "Root Application object plus article/asset/behavior classes. Verified names from ExtendScript index; UXP URL namespace uses first letter.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-02"
    },
    {
      "group": "B namespace (/dom/api/b/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/b/book/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Book",
        "Button",
        "Bookmark",
        "BackgroundTask",
        "BlendingSetting",
        "BevelAndEmbossSetting"
      ],
      "notes": "Book/document-set, interactive button, and effect-setting classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-03"
    },
    {
      "group": "C namespace (/dom/api/c/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/c/character/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Character",
        "CharacterStyle",
        "Cell",
        "CellStyle",
        "Color",
        "Column",
        "Condition",
        "CrossReference",
        "Change",
        "ColorGroup"
      ],
      "notes": "One of the densest letters: text-run (Character/CharacterStyle), table (Cell/CellStyle/Column), color, and conditional-text classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-04"
    },
    {
      "group": "D namespace (/dom/api/d/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/d/document/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Document",
        "Dialog",
        "DropShadowSetting",
        "DocumentPreference"
      ],
      "notes": "Top-level Document object plus scripting-UI Dialog and *Preference/effect classes. /d/document/ URL confirmed live from search index.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-05"
    },
    {
      "group": "E-F namespace (/dom/api/e/, /dom/api/f/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/f/font/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "EPS",
        "EPubExportPreference",
        "Endnote",
        "Event",
        "EventListener",
        "Font",
        "Fonts",
        "Footnote",
        "FeatherSetting",
        "FindGrepPreference"
      ],
      "notes": "Export/import formats, event model (Event/EventListener), typography (Font/Fonts), note classes, and GREP find/change preference classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-06"
    },
    {
      "group": "G-H namespace (/dom/api/g/, /dom/api/h/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/g/group/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Group",
        "Graphic",
        "Guide",
        "Gradient",
        "GradientStop",
        "GradientFeatherSetting",
        "GotoPageBehavior",
        "Hyperlink",
        "HtmlItem"
      ],
      "notes": "Grouped page items, gradients, layout guides, interactive behaviors, and hyperlink/HTML classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-07"
    },
    {
      "group": "I-L namespace (/dom/api/i/, /dom/api/l/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/i/insertionpoint/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Image",
        "InsertionPoint",
        "Ink",
        "Index",
        "ImportedPage",
        "Layer",
        "Line",
        "Link"
      ],
      "notes": "Placed graphics (Image/Link/ImportedPage), text-position InsertionPoint, print inks, index, and layer/line classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-08"
    },
    {
      "group": "M-N-O namespace (/dom/api/m/, /dom/api/n/, /dom/api/o/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/m/masterspread/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "MasterSpread",
        "Movie",
        "MixedInk",
        "MultiStateObject",
        "NestedStyle",
        "Note",
        "ObjectStyle",
        "ObjectStyleGroup",
        "Oval"
      ],
      "notes": "Master pages, rich media (Movie/MultiStateObject), mixed inks, nested/object styles, and shape classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-09"
    },
    {
      "group": "P namespace (/dom/api/p/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/p/paragraph/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Page",
        "PageItem",
        "Paragraph",
        "ParagraphStyle",
        "Path",
        "PathPoint",
        "Polygon",
        "PDF",
        "PrintPreference",
        "PDFExportPreference"
      ],
      "notes": "Densest layout letter: page geometry (Page/PageItem/Path/PathPoint/Polygon), paragraph text/styles, and PDF/print export preference classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-10"
    },
    {
      "group": "R-S namespace (/dom/api/r/, /dom/api/s/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/s/story/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Rectangle",
        "Row",
        "Story",
        "Spread",
        "Section",
        "Swatch",
        "State",
        "Sound",
        "SubmitFormBehavior",
        "SVG"
      ],
      "notes": "Rectangle/Row shapes-and-tables, plus the Story text container, Spread layout, Swatch color, and interactive State/Sound/behavior classes. /s/story/ confirmed live.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-11"
    },
    {
      "group": "T namespace (/dom/api/t/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/t/table/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Table",
        "TableStyle",
        "TableStyleGroup",
        "Text",
        "TextFrame",
        "TextVariable",
        "TextPath",
        "Tint",
        "Topic",
        "TrapPreset"
      ],
      "notes": "Table model (Table/TableStyle), core text objects (Text/TextFrame/TextPath/TextVariable), index Topic, and trap/tint print classes.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-12"
    },
    {
      "group": "U-W-X namespace (/dom/api/u/, /dom/api/w/, /dom/api/x/)",
      "source_url": "https://developer.adobe.com/indesign/uxp/dom/api/w/word/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Word",
        "Window",
        "WMF",
        "XMLElement",
        "XMLAttribute",
        "XMLTag",
        "XMLItem"
      ],
      "notes": "Text Word unit, scripting Window, WMF graphic, and the XML/structure suite (XMLElement/XMLAttribute/XMLTag) used for tagged-content workflows.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-13"
    },
    {
      "group": "Preferences cross-letter family (*Preference / *Setting classes)",
      "source_url": "https://www.indesignjs.de/extendscriptAPI/indesign-latest/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "DocumentPreference",
        "PDFExportPreference",
        "EPubExportPreference",
        "PrintPreference",
        "FindGrepPreference",
        "ChangeGrepPreference",
        "TextWrapPreference",
        "DropShadowSetting",
        "FeatherSetting",
        "GradientFeatherSetting"
      ],
      "notes": "Large behavioral sub-namespace spanning many letters: every *Preference and *Setting class controls export/print/find-change/effect state. Grouped here because it is a distinct DOM domain rather than a single URL letter; individual classes still live under their first-letter UXP path.",
      "id": "SFR-CMDCAP-indesign-scripting_dom-14"
    }
  ]
}
```

### [SFR-CMDCAP.indesign-shortcuts] indesign / shortcuts (13 groups)

```json
{
  "groups": [
    {
      "group": "Adobe helpx canonical enumeration page (Keys for... tables)",
      "source_url": "https://helpx.adobe.com/indesign/using/default-keyboard-shortcuts.html",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [],
      "notes": "CANONICAL enumeration source per task. Could NOT be fetched: WebFetch timed out at 60s on this URL and the /au/ and /in/ mirrors; web.archive.org is blocked in this environment. Adobe organizes this page into multiple 'Keys for <area>' tables (tools, selecting/moving objects, transforming objects, editing text, finding/changing text, working with type, tables, navigating documents, XML, indexing, panels, Control panel, etc.); the exact verbatim heading list is UNVERIFIED because the page did not load. A later pass on a faster fetch path should transcribe the per-table headings and rows deterministically from this URL.",
      "id": "SFR-CMDCAP-indesign-shortcuts-01"
    },
    {
      "group": "Tools",
      "source_url": "https://tutorialtactic.com/blog/adobe-indesign-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Selection tool = V",
        "Type tool = T",
        "Pen tool = P",
        "Direct Selection tool = A"
      ],
      "notes": "Single-key tool bindings (same on Win/Mac). Cross-referenced with Adobe view-select-tools doc examples (V/T/P/A).",
      "id": "SFR-CMDCAP-indesign-shortcuts-02"
    },
    {
      "group": "File Menu",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "New: Document = Ctrl+N / Cmd+N",
        "Open = Ctrl+O / Cmd+O",
        "Save = Ctrl+S / Cmd+S",
        "Export = Ctrl+E / Cmd+E",
        "Print = Ctrl+P / Cmd+P"
      ],
      "notes": "Menu-scoped command group in the InDesign Keyboard Shortcuts dialog product-area list.",
      "id": "SFR-CMDCAP-indesign-shortcuts-03"
    },
    {
      "group": "Edit Menu",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Copy = Ctrl+C / Cmd+C",
        "Paste = Ctrl+V / Cmd+V",
        "Undo = Ctrl+Z / Cmd+Z",
        "Find/Change = Ctrl+F / Cmd+F",
        "Select All = Ctrl+A / Cmd+A",
        "Duplicate = Ctrl+Shift+Alt+D / Cmd+Shift+Option+D"
      ],
      "id": "SFR-CMDCAP-indesign-shortcuts-04"
    },
    {
      "group": "Layout Menu",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Go to Page = Ctrl+J / Cmd+J",
        "Next Page = Shift+Page Down",
        "Previous Page = Shift+Page Up"
      ],
      "id": "SFR-CMDCAP-indesign-shortcuts-05"
    },
    {
      "group": "Type Menu",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Create Outlines = Shift+Ctrl+O / Shift+Cmd+O",
        "Show Hidden Characters = Alt+Ctrl+I / Option+Cmd+I"
      ],
      "id": "SFR-CMDCAP-indesign-shortcuts-06"
    },
    {
      "group": "Object Menu / Object Editing",
      "source_url": "https://tutorialtactic.com/blog/adobe-indesign-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Group = Ctrl+G / Cmd+G",
        "Drop Shadow = Ctrl+Alt+M / Cmd+Option+M",
        "Bring to Front = Ctrl+Shift+] / Cmd+Shift+]",
        "Bring to Front (blog variant) = Ctrl+Shift+[ / Cmd+Shift+["
      ],
      "notes": "Redokun and TutorialTactic both list this as its own group; Adobe's dialog splits Object Menu (command) from Object Editing (context).",
      "id": "SFR-CMDCAP-indesign-shortcuts-07"
    },
    {
      "group": "Table Menu / Text and Tables",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [],
      "notes": "Redokun lists 'Table Menu' and 'Text and Tables' as distinct groups; exemplar rows not captured in fetch. Fetch full rows from redokun or the Adobe 'Keys for tables' table.",
      "id": "SFR-CMDCAP-indesign-shortcuts-08"
    },
    {
      "group": "View Menu / Views, Navigation / Zoom",
      "source_url": "https://tutorialtactic.com/blog/adobe-indesign-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Smart Guides = Ctrl+U / Cmd+U",
        "Show Rulers = Ctrl+R / Cmd+R",
        "Hide Frame Edges = Ctrl+H / Cmd+H",
        "Zoom In = Ctrl++ / Cmd++",
        "Zoom Out = Ctrl+- / Cmd+-",
        "Fit Page in Window = Ctrl+0 / Cmd+0"
      ],
      "id": "SFR-CMDCAP-indesign-shortcuts-09"
    },
    {
      "group": "Window Menu / Panels / Panel Menus",
      "source_url": "https://tutorialtactic.com/blog/adobe-indesign-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Swatches = F5",
        "Layers = F7",
        "Paragraph Styles = F11",
        "Character Styles = Shift+F11"
      ],
      "notes": "Adobe's dialog exposes 'Panel Menus' as a separate context area from Window Menu commands.",
      "id": "SFR-CMDCAP-indesign-shortcuts-10"
    },
    {
      "group": "Application Menu",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Preferences: General = Ctrl+K / Cmd+K"
      ],
      "id": "SFR-CMDCAP-indesign-shortcuts-11"
    },
    {
      "group": "Formatting / Character-Editing (type styling)",
      "source_url": "https://tutorialtactic.com/blog/adobe-indesign-shortcuts/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Increase Font Size = Ctrl+Shift+> / Cmd+Shift+>",
        "Bold = Ctrl+Shift+B / Cmd+Shift+B",
        "Align Center = Ctrl+Shift+C / Cmd+Shift+C"
      ],
      "notes": "Corresponds to Adobe 'Keys for working with type' / 'Keys for type styles'.",
      "id": "SFR-CMDCAP-indesign-shortcuts-12"
    },
    {
      "group": "Structure Navigation (XML) / Help Menu",
      "source_url": "https://redokun.com/blog/indesign-shortcuts",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [],
      "notes": "Redokun lists 'Structure Navigation' and 'Help Menu' as distinct groups (maps to Adobe 'Keys for working with XML'); rows not captured. Fetch from redokun or Adobe page.",
      "id": "SFR-CMDCAP-indesign-shortcuts-13"
    }
  ]
}
```

### [SFR-CMDCAP.affinity-shortcuts] affinity / shortcuts (7 groups)

```json
{
  "groups": [
    {
      "group": "Publisher 2 — full shortcut category index",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/Workspace/shortcuts.html",
      "coverage": "FULL_LIST_ON_PAGE",
      "count": "unstated",
      "exemplars": [
        "Operations shortcuts",
        "Curve drawing shortcuts",
        "Curve editing with Node Tool",
        "Transforming shortcuts",
        "File shortcuts",
        "Tools shortcuts",
        "Edit shortcuts",
        "Page control shortcuts",
        "Object control shortcuts",
        "Text shortcuts (Navigation / Deleting / Formatting / Typography / Special characters)",
        "View shortcuts",
        "Selection shortcuts",
        "Workspace shortcuts",
        "Blend mode shortcuts",
        "Miscellaneous shortcuts",
        "macOS shortcuts"
      ],
      "notes": "Single shortcuts.html page lists the complete per-category binding tables for the Publisher persona. The ~N per-category counts observed in fetch are estimates, NOT source-stated, so left unstated. Page is an SPA under affinity.help but WebFetch renders category+binding content.",
      "id": "SFR-CMDCAP-affinity-shortcuts-01"
    },
    {
      "group": "Publisher 2 — File shortcuts (exemplar bindings)",
      "source_url": "https://affinity.help/publisher2/en-US.lproj/pages/Workspace/shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "New Document = Cmd+N",
        "Open Document = Cmd+O",
        "Save = Cmd+S",
        "Export = Shift+Cmd+S",
        "Print = Cmd+P",
        "Document Setup"
      ],
      "notes": "Bindings shown in macOS form on the page; Windows equivalents substitute Ctrl for Cmd. Sampled to anchor deterministic later transcription.",
      "id": "SFR-CMDCAP-affinity-shortcuts-02"
    },
    {
      "group": "Photo 2 — full shortcut category index",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/shortcuts.html",
      "coverage": "FULL_LIST_ON_PAGE",
      "count": "unstated",
      "exemplars": [
        "Editing shortcuts",
        "File shortcuts",
        "Tools shortcuts",
        "Edit shortcuts",
        "Layer Operations shortcuts",
        "View shortcuts",
        "Painting shortcuts",
        "Text shortcuts",
        "Blend mode shortcuts",
        "Selection shortcuts",
        "Workspace shortcuts",
        "Miscellaneous shortcuts"
      ],
      "notes": "Photo persona adds Painting and Layer Operations categories (raster focus) vs Designer/Publisher. Full binding tables present on the single shortcuts.html page.",
      "id": "SFR-CMDCAP-affinity-shortcuts-03"
    },
    {
      "group": "Photo 2 — Tools / File / Layer exemplar bindings",
      "source_url": "https://affinity.help/photo2/en-US.lproj/pages/Workspace/shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "View Tool = H",
        "Move Tool = V",
        "Crop Tool = C",
        "Paint Brush = B",
        "Zoom Tool = Z",
        "New Document = Cmd+N",
        "Save = Cmd+S",
        "Export = Shift+Cmd+S",
        "Group = Cmd+G",
        "Duplicate = Cmd+J",
        "Merge Down = Cmd+E",
        "Resize Document = Shift+Cmd+I",
        "Resize Canvas = Shift+Cmd+C",
        "Toggle Snapping = ;",
        "Zoom to Fit = Cmd+0"
      ],
      "notes": "macOS bindings as shown; Windows uses Ctrl for Cmd.",
      "id": "SFR-CMDCAP-affinity-shortcuts-04"
    },
    {
      "group": "Designer 2 — full shortcut category index",
      "source_url": "https://affinity.help/designer2/en-US.lproj/pages/Workspace/shortcuts.html",
      "coverage": "FULL_LIST_ON_PAGE",
      "count": "unstated",
      "exemplars": [
        "Operations shortcuts",
        "File shortcuts",
        "Tools shortcuts",
        "Edit shortcuts",
        "Object Control shortcuts",
        "View shortcuts",
        "Painting shortcuts",
        "Text shortcuts",
        "Selection shortcuts",
        "Workspace shortcuts",
        "Blend mode shortcuts",
        "Miscellaneous shortcuts"
      ],
      "notes": "Designer persona category set overlaps Publisher (vector focus: Operations, Object Control) and shares Painting with Photo. Full binding tables on the single shortcuts.html page.",
      "id": "SFR-CMDCAP-affinity-shortcuts-05"
    },
    {
      "group": "Designer 2 — Operations / Tools / Edit exemplar bindings",
      "source_url": "https://affinity.help/designer2/en-US.lproj/pages/Workspace/shortcuts.html",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "unstated",
      "exemplars": [
        "Constrain object movement H/V/diagonal = Shift+drag",
        "Select multiple objects = Shift+click",
        "Decrease/increase stroke width = [ or ]",
        "Change layer opacity = numeric keys (4=40%, 43=43%)",
        "New Document = Cmd+N",
        "Export = Cmd+Shift+S",
        "Move Tool = V",
        "Pen Tool = P",
        "Gradient Tool = G",
        "Undo = Cmd+Z / Redo = Cmd+Shift+Z",
        "Paste Style = Cmd+Opt+V",
        "Group = Cmd+G",
        "Duplicate = Cmd+J",
        "Move to Front = Cmd+]",
        "Zoom to Fit = Cmd+0"
      ],
      "notes": "macOS symbol form on page (Cmd/Opt/Shift); Windows uses Ctrl/Alt/Shift. Opacity-by-numeric-key and bracket stroke-width are Affinity-wide idioms.",
      "id": "SFR-CMDCAP-affinity-shortcuts-06"
    },
    {
      "group": "DOMAIN-PIN NOTES — dead / blocked sources",
      "source_url": "https://www.affinity.studio/help/workspace-shortcuts-editing/",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [
        "affinity.studio/help/workspace-shortcuts-editing/ = redirects to Canva browser-notice SPA shell (no content)",
        "affinity.studio/help/workspace-shortcuts-workspace/ = same Canva notice shell",
        "support.serif.com/hc/en-us/articles/10259259400463 (V1/V2 cheat-sheet PDFs) = HTTP 403 Forbidden",
        "resources.serif.com/spotlight/learning/shortcuts/Affinity-Designer-2-Shortcuts-macOS.pdf = binary PDF, not text-extractable via WebFetch",
        "affinity.help/publisher2/.../index.html (TOC) = SPA index, links to shortcuts.html which IS fetchable"
      ],
      "notes": "affinity.studio help pages are now Canva-hosted and return a browser-update notice instead of shortcut content (post-Canva-acquisition). serif.com cheat-sheet article is 403 to WebFetch and the resources.serif.com PDFs download as binary. Canonical machine-fetchable source for V2 shortcut structure is the per-persona affinity.help .../pages/Workspace/shortcuts.html pages.",
      "id": "SFR-CMDCAP-affinity-shortcuts-07"
    }
  ]
}
```

### [SFR-CMDCAP.figma-shortcuts] figma / shortcuts (14 groups)

```json
{
  "groups": [
    {
      "group": "Essential",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "3 (third-party page count; not stated by official help article)",
      "exemplars": [
        "Show/Hide UI: Ctrl+\\ / Cmd+\\",
        "Pick Color: i",
        "Search: Ctrl+/ / Cmd+/"
      ],
      "notes": "Official enumeration source is the in-app Keyboard shortcuts panel (open with Ctrl+Shift+? / Control+Shift+?). Help article 360040328653 does NOT list per-category counts; counts here are unverified third-party.",
      "id": "SFR-CMDCAP-figma-shortcuts-01"
    },
    {
      "group": "Tools",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "10 (third-party page count; UNVERIFIED against official source)",
      "exemplars": [
        "Move Tool: v",
        "Frame Tool: f",
        "Pen Tool: p",
        "Text Tool: t",
        "Rectangle Tool: r",
        "Ellipse Tool: o",
        "Line Tool: l",
        "Arrow Tool: Shift+L",
        "Slice Tool: s",
        "Add/Show Comments: c"
      ],
      "notes": "Single-key tool activators; full list resolvable from in-app panel.",
      "id": "SFR-CMDCAP-figma-shortcuts-02"
    },
    {
      "group": "View",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "5 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Rulers: Shift+R",
        "Outlines: Shift+O",
        "Pixel Preview: Ctrl+Shift+P / Shift+Cmd+P",
        "Layout Grids: Shift+G",
        "Pixel Grid: Shift+'"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-03"
    },
    {
      "group": "Zoom",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "8 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Pan: Space+drag",
        "Zoom In: Ctrl+Plus / Cmd+Plus",
        "Zoom Out: Ctrl+- / Cmd+-",
        "Zoom to 100%: Ctrl+0 / Cmd+0",
        "Zoom to Fit: Shift+1",
        "Zoom to Selection: Shift+2",
        "Zoom to Next Frame: n",
        "Zoom to Previous Frame: Shift+N"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-04"
    },
    {
      "group": "Text",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "9 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Bold: Ctrl+B / Cmd+B",
        "Italic: Ctrl+I / Cmd+I",
        "Underline: Ctrl+U / Cmd+U",
        "Create Link: Ctrl+K / Cmd+K",
        "Strikethrough: Ctrl+Shift+X / Shift+Cmd+X",
        "Text Align Left: Ctrl+Alt+L / Alt+Cmd+L",
        "Text Align Center: Ctrl+Alt+T / Alt+Cmd+T",
        "Text Align Right: Ctrl+Alt+R / Alt+Cmd+R",
        "Adjust Font Size: Ctrl+Shift+< / Shift+Cmd+<"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-05"
    },
    {
      "group": "Shape",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "10 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Pen: p",
        "Pencil: Shift+P",
        "Paint Bucket: b",
        "Remove Fill: Alt+/",
        "Remove Stroke: Shift+/",
        "Swap Fill Stroke: Shift+X",
        "Outline Stroke: Ctrl+Shift+O / Shift+Cmd+O",
        "Flatten Selection: Ctrl+E / Cmd+E",
        "Join Selection: Ctrl+J / Cmd+J",
        "Delete Heal Selection: Shift+Backspace"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-06"
    },
    {
      "group": "Selection",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "9 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Select All: Ctrl+A / Cmd+A",
        "Select Inverse: Ctrl+Shift+A / Shift+Cmd+A",
        "Select None: Esc",
        "Deep Select: Ctrl+Click / Cmd+Click",
        "Select Children: Enter",
        "Select Parent: Shift+Enter",
        "Select Next Sibling: Tab",
        "Group Selection: Ctrl+G / Cmd+G",
        "Ungroup Selection: Ctrl+Backspace / Cmd+Backspace"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-07"
    },
    {
      "group": "Cursor",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "3 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Measure to Selection: Alt (hold)",
        "Duplicate Selection: Alt (drag)",
        "Deep Select Within Rectangle: Ctrl+Drag / Cmd+Drag"
      ],
      "notes": "Modifier-hold behaviors rather than discrete key bindings.",
      "id": "SFR-CMDCAP-figma-shortcuts-08"
    },
    {
      "group": "Edit",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "11 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Copy: Ctrl+C / Cmd+C",
        "Cut: Ctrl+X / Cmd+X",
        "Paste: Ctrl+V / Cmd+V",
        "Duplicate: Ctrl+D / Cmd+D",
        "Rename Selection: Ctrl+R / Cmd+R",
        "Export: Ctrl+Shift+E / Shift+Cmd+E",
        "Find: Ctrl+F / Cmd+F",
        "Copy as PNG: Ctrl+Shift+C / Shift+Cmd+C",
        "Copy Properties: Ctrl+Alt+C / Alt+Cmd+C",
        "Paste Properties: Ctrl+Alt+V / Alt+Cmd+V"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-09"
    },
    {
      "group": "Transform",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "7 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Flip Horizontal: Shift+H",
        "Flip Vertical: Shift+V",
        "Use as Mask: Ctrl+Alt+M / Ctrl+Cmd+M",
        "Edit Shape or Image: Enter",
        "Place Image: Ctrl+Shift+K / Shift+Cmd+K",
        "Set opacity: number keys 0-9"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-10"
    },
    {
      "group": "Arrange",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "10 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Bring Forward: Ctrl+] / Cmd+]",
        "Send Backward: Ctrl+[ / Cmd+[",
        "Bring to Front: ]",
        "Send to Back: [",
        "Align Left/Right: Alt+A / Alt+D",
        "Align Top/Bottom: Alt+W / Alt+S",
        "Align Horizontal/Vertical Centers: Alt+H / Alt+V",
        "Add Auto Layout: Shift+A",
        "Remove Auto Layout: Alt+Shift+A"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-11"
    },
    {
      "group": "Components",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "4 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Show Assets: Alt+2",
        "Team Library: Ctrl+Alt+O / Alt+Cmd+O",
        "Create Component: Ctrl+Alt+K / Alt+Cmd+K",
        "Detach Instance: Ctrl+Alt+B / Alt+Cmd+B"
      ],
      "notes": "",
      "id": "SFR-CMDCAP-figma-shortcuts-12"
    },
    {
      "group": "Other (out-of-scope-of-12-named-categories; captured for completeness)",
      "source_url": "https://keycombiner.com/collections/figma/",
      "coverage": "REPRESENTATIVE_SAMPLE",
      "count": "12 (third-party page count; UNVERIFIED)",
      "exemplars": [
        "Show Keyboard Shortcuts: Ctrl+Shift+?",
        "Save to version history: Ctrl+Alt+S",
        "Undo: Ctrl+Z / Cmd+Z",
        "Collapse all Layers: Alt+L",
        "Run last Plugin: Ctrl+Alt+P / Alt+Cmd+P",
        "Snap to Pixel Grid: Ctrl+Shift+' / Shift+Cmd+'"
      ],
      "notes": "Not one of the 12 categories the task named; the classic in-app panel also groups miscellany here.",
      "id": "SFR-CMDCAP-figma-shortcuts-13"
    },
    {
      "group": "OFFICIAL SOURCE NOTE (enumeration authority)",
      "source_url": "https://help.figma.com/hc/en-us/articles/360040328653-Use-Figma-products-with-a-keyboard",
      "coverage": "INDEX_ONLY",
      "count": "unstated",
      "exemplars": [
        "Open shortcut panel: Ctrl+Shift+? (Win) / Control+Shift+? (Mac)",
        "Help and resources menu > Keyboard shortcuts"
      ],
      "notes": "Official help article 360040328653 (title now 'Use Figma products with a keyboard') does NOT enumerate categories or per-category counts; it directs users to the in-app Keyboard shortcuts panel for the full categorized list. WebSearch surfaced a Figma-side tab grouping of 7 tabs (Essential Tools, Selection & Editing, Layering, View & Navigate, Components, Prototyping, Other) which differs from the 12 legacy panel categories named in this task. The 12-category structure (Essential/Tools/View/Zoom/Text/Shape/Selection/Cursor/Edit/Transform/Arrange/Components) matches the legacy in-app panel and is mirrored by third-party keycombiner; treat all counts as UNVERIFIED until read from the live in-app panel.",
      "id": "SFR-CMDCAP-figma-shortcuts-14"
    }
  ]
}
```
