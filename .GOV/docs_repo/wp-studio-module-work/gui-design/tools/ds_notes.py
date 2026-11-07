"""Prose blocks of design-system.json: identity, state styles, DPI rules, primitives, departures, research."""

IDENTITY = {
    "core_idea": "Edge-lit graphite. The chrome is true neutral grey with R=G=B everywhere, so it cannot bias a colour decision. State is carried by thin lit edges, not by filled blocks: a 2px green edge on what is active, a 1px edge under what can be edited, a hot point inside each tool icon. The work is the only large colour on screen.",
    "principles": [
        {"id": "ID-1", "rule": "True neutral chrome. Every surface, edge and text token in the chrome has R=G=B; only accent and status tokens carry hue.", "why": "A blue-tinted dark UI biases colour judgement; neutral grey avoids chromatic adaptation and simultaneous contrast. See research_basis F-07."},
        {"id": "ID-2", "rule": "One accent. accent.* marks 'this one is active or selected' and nothing else. It is never decoration and never a large fill, except one primary button per dialog.", "why": "Keeps the accent readable as a signal after eight hours of work."},
        {"id": "ID-3", "rule": "Edges carry state. Active tab: 2px top edge. Selected row and active tool: 2px left edge plus a lifted neutral. Field: 1px bottom edge that thickens to 2px, and can act as a value meter. Scrubbable: 2px gutter tint.", "why": "A single vocabulary across every control is what makes the shell coherent, and rects are the cheapest thing a Vulkan pass can draw."},
        {"id": "ID-4", "rule": "The hot point. Every tool icon is a line form plus one solid 3x3 point placed where the tool acts: brush tip, pen nib, crop corner. The point is a separate atlas layer, tinted accent when the tool is active.", "why": "Own visual signature, not copied from any vendor, and it teaches the tool's hotspot."},
        {"id": "ID-5", "rule": "Shared edges only. Docked panes touch; radius 0 on anything docked. Only things that float have a radius and a shadow: menus, tooltips, dialogs, overlays.", "why": "GUI-DESKTOP-007. Figma reattached its floating panels after pushback (F-03)."},
        {"id": "ID-6", "rule": "Same frame everywhere. The seven regions never change place between domains. A Layout Preset changes what is inside them, never where they are.", "why": "One logic across ten domains; STU-SHL-084, STU-SHL-093."},
        {"id": "ID-7", "rule": "Numbers line up. Every number uses tabular figures, right-aligned inside its field, unit in text.tertiary.", "why": "A dense inspector is read by scanning columns of digits."},
    ],
    "not_this": ["No glass, blur or translucent chrome (GUI-DESKTOP-011).", "No gradients in chrome: gradients appear only inside colour fields and user content.", "No vendor icons, colour identity, wordmarks or layouts copied. Vendor names never reach the UI (STU-SHL-008)."],
}

ACCENT_DECISION = {
    "chosen": "#22C55E on dark, #15803D on light",
    "why": "Studio must retain Handshake identity (STU-SHL-072). #22C55E is the shell's actual dark accent (handshake_native theme/palette.rs, read at handshake_main 4fe1e9a92). It is also unlike the azure used by the source suites, so it gives Studio its own look.",
    "known_costs": [
        "Green sits close to 'success'. status.success uses a paler tone, and always comes with a check glyph.",
        "Green overlays vanish on green-screen footage. Every canvas overlay is a double stroke with a dark halo, so a selection stays visible on any image.",
        "The shell's light accent is #2563EB, not green. Studio's light theme keeps green so one product has one accent; the shell owner should settle this.",
    ],
    "swap_cost": "Accent is one token family. Changing it touches accent.*, focus.ring, overlay.selection, time.playhead_head, presence.operator and time.key_selected, then re-run the contrast check.",
}

SHELL_CONFLICT = "The shell palette today is blue-tinted (dark bg #121418, surface #1D232B). If the menu bar and module rail keep it while a Studio pane has focus, a tinted band runs around a neutral work area and ID-1 fails at the edges. Recommendation: the shell reads the surface.frame, surface.strip and text.* tokens while a Studio pane has focus. Decision belongs to the shell owner; UNVERIFIED how much work that is."

SCALE_RULES = {
    "units": "Every size in this file is a logical pixel at scale 1. Stored layouts hold logical pixels only.",
    "scale": "s = os_monitor_scale * ui_scale. os_monitor_scale comes per monitor from the window system (1, 1.25, 1.5, 1.75, 2, 2.25, 2.5, 3, ...). ui_scale is a Studio preference: 1.0, 1.1, 1.25, 1.5.",
    "rect_snap": "Convert EDGES, not sizes: x0p = round(x0*s), x1p = round(x1*s). Two adjacent panes therefore share one physical edge and can never open a gap or overlap at a fractional scale.",
    "hairline": "A 1 logical px line is max(1, floor(s)) physical px, drawn inside its owner's rect. A 2 logical px edge is max(2, round(2*s)).",
    "text": "Shape at the logical size; rasterise at size*s physical px with no vertical subpixel position (baseline snaps to a physical pixel) and quarter-pixel horizontal bins. Grey coverage, not LCD: the same frame must compare byte-stable across monitors and in screenshots. Hinting off or light. Line height = round(line*s). UNVERIFIED: choice of rasteriser, see build-order B3.",
    "icons": "Author on the logical grid. Rasterise at round(icon*s) physical px per scale, stroke = max(1, round(1.5*s)). Hot point stays on the pixel grid. No bitmap upscaling.",
    "colour_pipeline": "UI tokens are sRGB-encoded. The chrome pass writes them unchanged to the swapchain and blends text coverage in gamma-encoded space, so a token value equals the pixel value in a screenshot and contrast checks hold. User content goes through the colour pipeline (14.8), not through this pass. UNVERIFIED: HDR swapchain behaviour, not specified here.",
    "monitor_change": "On a scale change: rebuild glyph and icon atlases for the new scale, re-run layout from logical values, keep document state. Never write the derived physical sizes back into L2.",
    "minimum_targets": "Interactive target at least 24 x 24 logical at any scale. ui_scale below 1.0 is not offered.",
    "text_floor": "No text under 11 logical px. After Effects users have asked for text scaling for over a decade (F-06); Studio ships ui_scale from day one.",
}

RENDERER_PRIMITIVES = {
    "note": "The shell needs only these. If a mock or component seems to need something else, it is a design defect. Reported to the owner of the Vulkan binding (open decision O2 in HANDOFF).",
    "primitives": [
        {"id": "rect", "use": "all surfaces, edges, seams, tick marks", "shader": "flat colour"},
        {"id": "rrect", "use": "chips, buttons, popups", "shader": "signed distance rounded box, 1 physical px antialias, optional 1 logical px border"},
        {"id": "line", "use": "connectors, curves on canvas overlays, graph editor", "shader": "tessellated quad strip with antialiased edge"},
        {"id": "dash", "use": "marching ants, safe margins, drop zone outline", "shader": "rect or line with dash_px and phase uniform"},
        {"id": "glyph", "use": "all text", "shader": "grey coverage atlas per physical size"},
        {"id": "icon", "use": "all icons, two tint layers (form, hot point)", "shader": "RG coverage atlas: R tinted form, G tinted hot point"},
        {"id": "image", "use": "canvas, thumbnails, viewers, waveforms from cache", "shader": "sampled texture quad under a scissor"},
        {"id": "checker", "use": "transparency grid", "shader": "two colours, 8 logical px cell"},
        {"id": "ramp", "use": "colour fields, gradient previews", "shader": "2-corner or 4-corner interpolation in the owner's colour space"},
        {"id": "shadow", "use": "only menus, tooltips, dialogs, overlay panels", "shader": "9-slice of a pre-blurred rounded rect; no runtime blur"},
        {"id": "scissor", "use": "every pane and scroll view", "shader": "Vulkan scissor rect on physical pixels"},
    ],
}

ELEVATION = {
    "docked": "No shadow. Separation comes from edge.seam.",
    "floating_panel": {"shadow": "9-slice, offset y 4, blur 12, colour #000000 at 45 percent", "edge": "1px edge.seam plus surface.raised fill"},
    "menu_tooltip": {"shadow": "9-slice, offset y 2, blur 8, colour #000000 at 40 percent", "edge": "1px edge.seam"},
    "dialog": {"shadow": "9-slice, offset y 8, blur 24, colour #000000 at 50 percent", "scrim": "none for modeless dialogs; flat #000000 at 35 percent only for a blocking modal"},
}

ICON = {
    "grids": {"tool": 20, "panel": 16, "micro": 12},
    "live_area": {"tool": 16, "panel": 13, "micro": 10},
    "stroke": {"tool": 1.5, "panel": 1.25, "micro": 1.0},
    "cap_join": "round cap, round join",
    "corner_radius": "1 logical px on closed forms, except where a sharp corner is the meaning (crop, marquee)",
    "hot_point": {"size": 3, "shape": "square at 20 grid, 1 px corner rounding", "rule": "Exactly one per tool icon, placed where the tool acts. Panel and micro icons have none.", "tint_rest": "icon form colour", "tint_active": "accent.base"},
    "layers": ["form (R channel)", "hot point (G channel)"],
    "variant_marker": {"size": 4, "shape": "right triangle, bottom-right corner of the 36 x 32 slot", "colour": "text.tertiary; hit area 12 x 12"},
    "colour_rule": "Single colour only, tinted from tokens. No multicolour icons in chrome.",
    "provenance": "All glyphs in the mocks are authored for this kit on the grids above. Icon content is original; generic subjects such as a brush or a pen are concepts, not copies.",
}

DENSITY = {
    "modes": {
        "standard": {"status": "default, STU-SHL-072", "param_pitch": 28, "field_h": 24, "row_h": 24, "layer_row_h": 32, "section_header_h": 24, "group_header_h": 26},
        "dense": {"status": "proposed answer to OD-2, Operator decides", "param_pitch": 24, "field_h": 24, "row_h": 20, "layer_row_h": 24, "section_header_h": 20, "group_header_h": 24, "note": "The grab target stays 24px (STU-SHL-072, STU-SHL-196). Dense removes the 4px gap between rows and tightens list rows only, so OD-2's 20 percent gain is mostly kept without thinning the scrub target."},
        "compact_stacked": {"status": "automatic below 260px wide, STU-SHL-073", "label_h": 16, "field_h": 24, "gap": 4, "param_pitch": 44, "note": "Departure D-05: STU-SHL-073 says 20px pitch, which cannot hold a 24px field plus a label."},
    },
    "switch": "A preference, applied live with no restart. ui_scale is separate and multiplies everything.",
}

STATE_STYLES = {
    "note": "Every state of every control resolves to these. components.json refers to them by id instead of restating colours. The focus ring is drawn outside the control's rect and is never clipped by the pane scissor: it insets when the control touches the pane edge.",
    "interaction": {
        "control": {
            "rest": {"fill": "surface.control", "text": "text.primary", "icon": "text.secondary"},
            "hover": {"fill": "surface.control_hover", "text": "text.primary", "icon": "text.primary"},
            "active": {"fill": "surface.control_active", "text": "text.primary", "icon": "text.primary"},
            "selected": {"fill": "surface.row_selected", "text": "text.primary", "icon": "text.primary", "edge_left_2": "accent.base"},
            "focus": {"ring": "focus.ring", "halo": "focus.halo", "ring_w": 2, "halo_w": 1, "offset": 1},
            "disabled": {"fill": "surface.panel", "text": "text.disabled", "icon": "text.disabled", "reason": "required, see availability"},
        },
        "ghost": {
            "rest": {"fill": "transparent", "text": "text.secondary", "icon": "text.secondary"},
            "hover": {"fill": "surface.row_hover", "text": "text.primary", "icon": "text.primary"},
            "active": {"fill": "surface.control_active", "text": "text.primary", "icon": "text.primary"},
            "selected": {"fill": "surface.row_selected", "text": "text.primary", "icon": "text.primary"},
            "focus": {"ring": "focus.ring", "halo": "focus.halo", "ring_w": 2, "halo_w": 1, "offset": 1},
            "disabled": {"fill": "transparent", "text": "text.disabled", "icon": "text.disabled"},
        },
        "primary": {
            "rest": {"fill": "accent.base", "text": "text.on_accent"},
            "hover": {"fill": "accent.hover", "text": "text.on_accent"},
            "active": {"fill": "accent.pressed", "text": "text.on_accent"},
            "focus": {"ring": "focus.ring", "halo": "focus.halo", "ring_w": 2, "halo_w": 1, "offset": 2},
            "disabled": {"fill": "surface.control", "text": "text.disabled"},
        },
        "field": {
            "rest": {"fill": "surface.field", "text": "text.primary", "unit": "text.tertiary", "edge_bottom_1": "edge.control"},
            "hover": {"fill": "surface.field_hover", "text": "text.primary", "edge_bottom_1": "text.secondary", "gutter_left_2": "accent.base", "cursor": "column-resize when scrubbable"},
            "scrubbing": {"fill": "surface.field_hover", "text": "text.primary", "edge_bottom_2": "accent.base", "gutter_left_2": "accent.base", "cursor": "hidden at screen edge, never warped"},
            "editing": {"fill": "surface.field", "text": "text.primary", "ring": "focus.ring", "ring_w": 2, "halo": "focus.halo", "caret": "text.primary", "selection_bg": "accent.row_tint"},
            "focus": {"ring": "focus.ring", "halo": "focus.halo", "ring_w": 2, "halo_w": 1, "offset": 0},
            "disabled": {"fill": "surface.panel", "text": "text.disabled", "edge_bottom_1": "edge.subtle"},
            "error": {"fill": "surface.field", "text": "text.primary", "edge_bottom_2": "status.error", "glyph": "error, 12px, at the field end", "message": "under the field or in the tooltip, never colour alone"},
            "expression_driven": {"fill": "surface.panel", "text": "text.secondary", "edge_bottom_1": "edge.subtle", "badge": "expression, 12px", "cursor": "default", "claims_press": False},
        },
        "row": {
            "rest": {"fill": "transparent", "text": "text.primary", "secondary": "text.tertiary"},
            "hover": {"fill": "surface.row_hover", "text": "text.primary"},
            "selected": {"fill": "surface.row_selected", "text": "text.primary", "edge_left_2": "accent.base"},
            "selected_unfocused": {"fill": "surface.row_selected", "text": "text.primary", "edge_left_2": "edge.control"},
            "focus": {"ring": "focus.ring", "ring_w": 1, "inset": True},
            "dragging": {"fill": "surface.raised", "shadow": "menu_tooltip", "drop_edge_2": "accent.base"},
            "disabled": {"fill": "transparent", "text": "text.disabled"},
        },
        "tab": {
            "rest": {"fill": "surface.strip", "text": "text.secondary"},
            "hover": {"fill": "surface.row_hover", "text": "text.primary"},
            "active": {"fill": "surface.panel", "text": "text.primary", "weight": 600, "edge_top_2": "accent.base", "joins_body": True},
            "active_unfocused_group": {"fill": "surface.panel", "text": "text.primary", "weight": 600, "edge_top_2": "edge.control", "joins_body": True},
            "focus": {"ring": "focus.ring", "ring_w": 2, "inset": True},
            "dragging": {"fill": "surface.raised", "text": "text.primary", "shadow": "menu_tooltip"},
        },
        "tool_slot": {
            "rest": {"fill": "transparent", "icon": "text.secondary", "hot_point": "text.secondary"},
            "hover": {"fill": "surface.control", "icon": "text.primary", "hot_point": "text.primary"},
            "active": {"fill": "surface.slot_active", "icon": "text.primary", "hot_point": "accent.base", "edge_left_2": "accent.base"},
            "focus": {"ring": "focus.ring", "ring_w": 2, "inset": True},
        },
    },
    "availability": {
        "note": "The three states of STU-SHL-050. Rendering per surface is STU-SHL-052; a state never changes by Layout Preset (STU-SHL-055).",
        "AVAILABLE": "normal states above",
        "INAPPLICABLE_HERE": {"text": "text.disabled", "icon": "text.disabled", "tooltip": "L1 with reason sentence and one-click remedy", "rail": "dimmed, remedy inline", "menu": "greyed, reason on the row, remedy as enabled sibling leaf"},
        "NOT_IN_THIS_DOCUMENT": {"text": "text.disabled", "icon": "text.disabled", "rail": "absent from the rail, present in AccessKit", "menu": "greyed, reason on the row, never hidden", "search": "listed lowest with reason"},
    },
    "status": {
        "error": {"text": "status.error", "band": "status.error_bg", "glyph": "error", "edge_left_2": "status.error"},
        "warning": {"text": "status.warning", "band": "status.warning_bg", "glyph": "warning", "edge_left_2": "status.warning"},
        "success": {"text": "status.success", "band": "status.success_bg", "glyph": "check", "edge_left_2": "status.success"},
        "info": {"text": "status.info", "band": "status.info_bg", "glyph": "info", "edge_left_2": "status.info"},
        "rule": "A status is always a glyph plus text. Colour alone never carries meaning.",
    },
}

DEPARTURES = [
    {"id": "D-01", "clause": "STU-SHL-069 item 1 (min fraction 0.2, max 0.8, step 0.05)", "what": "Edge docks are clamped by pixel minimums, not fractions.", "why": "300 of 1920 is 0.156, under the clause's own 0.2 minimum, and the shares of STU-SHL-085, 086 and 089 (0.24, 0.46, 0.14) break both the step and the minimum. The clause cannot hold together with the shipped defaults. Fractions stay for centre splits and group-share resize, pixel minimums win.", "needs": "spec owner"},
    {"id": "D-02", "clause": "STU-SHL-103 (splitter visual thickness 4px) with GUI-DESKTOP-007 (no gutters)", "what": "At rest a splitter draws as a 1px seam taking no layout space. The 4px visual appears on hover and drag, centred on the seam, drawn over the panes. Hit thickness stays 8px.", "why": "A permanent 4px strip is a gutter. This keeps both clauses."},
    {"id": "D-03", "clause": "STU-SHL-078 (the eight spine panels present in EVERY default layout) with STU-SHL-089 and STU-SHL-090", "what": "The panels those clauses omit are added to the right edge's registrations: colour, swatches, info in composition; layers, swatches in sequence. STU-SHL-084 rule 3 (layers opens the last right group) is therefore not met in sequence, where STU-SHL-090 is the more specific clause.", "why": "The clauses are law and conflict. Registration keeps the measured layout unchanged while making the spine present.", "needs": "spec owner"},
    {"id": "D-04", "clause": "STU-SHL-113 (closed catalogue) with STU-SHL-088, 089", "what": "site-tree, related-files, code-view and preview-render are named in default layouts and absent from the catalogue. layouts.json keeps them flagged catalogue_gap, with a proposed mapping.", "why": "Closed means a panel absent from the table does not exist, so the two clauses conflict.", "needs": "spec owner"},
    {"id": "D-05", "clause": "STU-SHL-073 (stacked compact form, 20px pitch)", "what": "Stacked pitch is 44px: label 16, field 24, gap 4.", "why": "A 24px minimum field (STU-SHL-072) with a label above it cannot fit in 20px."},
    {"id": "D-06", "clause": "STU-SHL-081 (effective_layout = L3 ?? selected L1 ?? L2 ?? L0)", "what": "When an L1 preset is selected and the operator moves a panel, the move is written to L2 and the preset is marked modified with a dot; 'Revert to Preset' returns to L1, 'Reset Layout' to L0.", "why": "Read literally, a selected L1 outranks L2 so a move would not stick, yet L2 is 'the layer that makes rearrangement stick'. This reads both.", "needs": "spec owner"},
    {"id": "D-07", "clause": "Brief says 'workspace'; STU-SHL-002 forbids that word for this thing", "what": "All design files say Layout Preset. Only the requested file names keep the brief's mock-<workspace> pattern, using a domain word. WORKSPACE stays as the menu title because STU-SHL-011 names it.", "why": "Using a superseded name is a conformance defect."},
    {"id": "D-08", "clause": "STU-SHL-095 (twelve proposed presets, OD-4)", "what": "Four more job-named presets are proposed: Develop, Mix, Interface, Workshop. They are marked PROPOSED_OD4.", "why": "The twelve leave audio, design and board with no preset, and develop with only Catalog. No board or audio signature exists in STU-SHL-054 either.", "needs": "Operator, OD-4"},
    {"id": "D-09", "clause": "STU-SHL-066 (closed viewer kinds), STU-SHL-113, STU-SHL-155", "what": "Gaps found while laying out domains: no waveform or spectral viewer kind, no filmstrip panel, no board tools in the tool catalogue (sticky, connector, stamp exist only under INSERT > Whiteboard Object), no prototype-player panel. layouts.json proposes a mapping for each.", "why": "The existing test host has Waveform Editor, Spectral Display and Prototype Player surfaces at c4d06841 with no catalogue row to land on.", "needs": "spec owner"},
    {"id": "D-10", "clause": "STU-SHL-074 (mixing rule) with STU-SHL-087", "what": "The layout preset's last group is [layers, transform, history, info], list-dense with field-dense. When it falls under 300px tall the fit ladder lifts transform into the group above, in presentation only.", "why": "At 1280x720 the group cannot be 300px tall. See layouts.json rule FIT-H4."},
    {"id": "D-12", "clause": "STU-SHL-097 (documents are tabs in editor groups)", "what": "The 32px strip lists documents. A document's own centre is a tree of editor groups with viewer tabs; a document with one viewer shows no group header. Dragging a document tab into a group's body splits and opens that document's default viewer there.", "why": "The clause reads as if the strip were always the group's tabs, but source and program viewers of one sequence need their own tabs. Interpretation; needs a ruling if the shell team reads it another way.", "needs": "spec owner"},
    {"id": "D-11", "clause": "STU-SHL-113 display name 'Pathfinder'", "what": "Kept as the spec names it, flagged. The test host already says 'Path Operations'.", "why": "The name reads as a vendor term of art and STU-SHL-008 bars vendor names. UNVERIFIED whether it is protected. Spec owner should rule.", "needs": "spec owner"},
]

RESEARCH_BASIS = {
    "date": "2026-11-07",
    "method_limits": "Adobe help pages were read as text in a browser. WebFetch was refused with HTTP 403 on helpx.adobe.com, and vendor screenshots could not be displayed because the browser pane was hidden, so no vendor screenshot was visually inspected. Layout conclusions rely on written documentation, forum reports, and the vendor measurements already in spec clauses 14.30 to 14.32. Illustrator's overview page returned no text. Forum reports are anecdote, labelled as such. No vendor application was launched.",
    "sources_checked": [
        {"id": "S-01", "what": "Photoshop workspace overview", "url": "https://helpx.adobe.com/photoshop/desktop/get-started/learn-the-basics/workspace-overview.html"},
        {"id": "S-02", "what": "After Effects workspaces, panels and viewers", "url": "https://helpx.adobe.com/after-effects/desktop/get-started/get-familiar-with-the-interface/workspaces-panels-viewers.html"},
        {"id": "S-03", "what": "Premiere overview of workspaces", "url": "https://helpx.adobe.com/premiere/desktop/get-started/tour-the-workspace/what-are-workspaces.html"},
        {"id": "S-04", "what": "InDesign interface preferences", "url": "https://helpx.adobe.com/indesign/desktop/get-started/settings-and-preferences/indesign-interface-preferences.html"},
        {"id": "S-05", "what": "Lightroom Classic workspace basics and Develop tips, search excerpts only", "url": "https://helpx.adobe.com/ph_en/lightroom-classic/desktop/workspace/workspace-basics.html"},
        {"id": "S-06", "what": "Audition customizing workspaces, search excerpts only", "url": "https://helpx.adobe.com/audition/using/customizing-workspaces.html"},
        {"id": "S-07", "what": "Affinity Designer interface visual reference (V1 page)", "url": "https://affinity.help/designer/en-US.lproj/pages/Workspace/interface.html"},
        {"id": "S-08", "what": "Figma right sidebar help", "url": "https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar"},
        {"id": "S-09", "what": "Figma UI3 feedback thread and help, search excerpts", "url": "https://forum.figma.com/share-your-feedback-26/ui3-feedback-3058/index9.html"},
        {"id": "S-10", "what": "FigJam help on stickies, connectors, sections, search excerpts", "url": "https://help.figma.com/hc/en-us/articles/1500004362321"},
        {"id": "S-11", "what": "Adobe community threads on the Contextual Task Bar and After Effects UI scaling, anecdotal", "url": "https://community.adobe.com/t5/after-effects-ideas/add-ui-scaling/idc-p/5801657"},
        {"id": "S-12", "what": "Adobe Spectrum colour system, neutral greys", "url": "https://spectrum.adobe.com/page/color-system"},
        {"id": "S-13", "what": "Kdenlive theme discussion on blue tint, anecdotal", "url": "https://discuss.kde.org/t/built-in-gray-theme/45385"},
        {"id": "S-14", "what": "WCAG 2.2 non-text contrast", "url": "https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html"},
        {"id": "S-15", "what": "AccessKit Role enum 0.25.2", "url": "https://docs.rs/accesskit/latest/accesskit/enum.Role.html"},
        {"id": "S-16", "what": "Inter typeface, SIL OFL 1.1, tabular numbers", "url": "https://rsms.me/inter/"},
        {"id": "S-17", "what": "Handshake shell theme and splitter constants, read locally at handshake_main 4fe1e9a92", "url": "src/frontend/handshake_native/src/theme/palette.rs, split_layout.rs"},
        {"id": "S-18", "what": "Existing test host panels and tools at product c4d06841", "url": "hsk-studio-harness-view/src/view/shell.rs, edit.rs, hosted.rs"},
    ],
    "findings": [
        {"id": "F-01", "finding": "The Adobe video and audio apps share one docking logic: a named arrangement of panels, drag to group, place alongside, or float; other panels resize to fit.", "source": ["S-02", "S-03"], "use": "One docking model for every domain. Named job-based arrangements match STU-SHL-095."},
        {"id": "F-02", "finding": "Photoshop's Contextual Task Bar floats, and users repeatedly ask for it to dock or stay pinned; it can cover content and the options bar (anecdotal).", "source": ["S-01", "S-11"], "use": "The Context Bar stays fixed in one strip. No floating contextual toolbar anywhere; on-canvas chips only report and never hold controls."},
        {"id": "F-03", "finding": "Figma introduced floating panels in UI3 and reattached them after pushback. Users also said a bottom toolbar pulled attention from the canvas, and that the properties panel had no clear scanning logic (anecdotal, plus Figma's own write-up).", "source": ["S-08", "S-09"], "use": "Keep shared-edge panes and a left Tool Rail, and make Properties section order fixed and generated from group_path."},
        {"id": "F-04", "finding": "Affinity's status bar shows the modifiers available for the current tool; its context toolbar shows tool options; personas switch workspace layouts.", "source": ["S-07"], "use": "Same answer as the L-status hintline. Studio rejects the persona's menu and tool swap per STU-SHL-094."},
        {"id": "F-05", "finding": "InDesign exposes four colour themes, pasteboard matching, a large-tabs option, and auto-collapse of icon panels. Lightroom lets the operator change the grey behind the image.", "source": ["S-04", "S-05"], "use": "Dark and light themes, a selectable canvas surround with a mid-grey option for colour work, and icon rails with click-to-overlay."},
        {"id": "F-06", "finding": "After Effects users have asked for UI text scaling since at least 2014 and call layer names and Effect Controls text too small (anecdotal).", "source": ["S-11"], "use": "ui_scale preference from day one, 12px base, 11px floor."},
        {"id": "F-07", "finding": "Adobe's Spectrum says neutral greys prevent misreading colour through chromatic adaptation or simultaneous contrast; a Kdenlive request calls its blue-tinted dark theme a problem for grading (anecdotal).", "source": ["S-12", "S-13"], "use": "True neutral chrome. Handshake's blue-tinted surfaces are not reused in the Studio viewport."},
        {"id": "F-08", "finding": "After Effects stacked panels expand and collapse with one click on the header; Lightroom's solo mode limits a panel group to one open section.", "source": ["S-02", "S-05"], "use": "Section headers collapse on one click; solo available from the panel menu, not only through a modifier."},
        {"id": "F-09", "finding": "Audition toggles Waveform and Multitrack editors from the toolbar; FigJam keeps its creation tools in one bottom toolbar. Figma's right sidebar is contextual: file-level options with no selection, layer properties with one.", "source": ["S-06", "S-08", "S-10"], "use": "Audio and board presets map onto the same seven regions. Empty selection shows document-level properties, matching STU-SHL-059 step 10's declared empty state."},
        {"id": "F-10", "finding": "WCAG 1.4.11 requires 3:1 for what identifies a control and its state; a full border is not required when other cues identify it; inactive components are exempt.", "source": ["S-14"], "use": "A bottom edge at 3:1 is enough for fields. Dimmed states are exempt from contrast, but still carry a reason by spec."},
    ],
    "reuse": ["Kumihimo stays the control owner (D8, STU-ARC-023): these tokens and components are its Studio theme input, not a new asset.", "Shell splitter constants DIVIDER_THICKNESS 4, DIVIDER_HIT_THICKNESS 8 reused.", "Shell accent #22C55E, error and success text values reused."],
    "rejected": [
        {"option": "Azure accent like the source suites", "why": "Copies their look, and the shell's dark identity is green."},
        {"option": "Filled selection colour on rows", "why": "Large saturated blocks beside the image bias judgement and tire the eye; a lifted neutral plus a lit edge reads as selected at less colour."},
        {"option": "Boxed fields with a full border", "why": "A dense panel becomes a wall of boxes. WCAG does not require it."},
        {"option": "Separate slider widget", "why": "STU-SHL-185 demands one numeric widget. The wide ScrubValue variant shows a rail."},
        {"option": "MSDF text for UI sizes", "why": "UNVERIFIED by test here; distance-field glyphs are usually soft at 11 to 13px, so a per-size coverage atlas is the starting point."},
        {"option": "Layers on the left in the design domain, like Figma", "why": "STU-SHL-084 rule 3 fixes layers as the first tab of the last right group; one logic across domains matters more than one suite's muscle memory. One drag moves it, and L2 keeps it."},
    ],
    "risks": [
        {"risk": "Green accent reads as 'done' or as chroma key.", "mitigation": "Paler success tone with glyph; double-stroke overlays; accent is one token family."},
        {"risk": "Thin edges vanish on a poor monitor.", "mitigation": "All edges pass 3:1; 2px at scale 1; a reviewer checks at 1280x720 on real hardware."},
        {"risk": "Mocks use Segoe UI where Inter is not installed, so text widths differ from the product.", "mitigation": "Width budgets in layouts.json carry margin; re-run acceptance checks on the real render."},
        {"risk": "A real panel's content does not fit the 266px row budget.", "mitigation": "Row budget is spec law; overflow goes to compact form or section groups, never clipping."},
    ],
    "validation_plan": "acceptance-checks.json. The first screen is judged by comparing a real Vulkan frame at 1920x1080 against mock-raster.html.",
}
