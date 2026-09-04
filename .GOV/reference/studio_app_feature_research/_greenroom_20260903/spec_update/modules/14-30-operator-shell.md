---
schema: handshake.indexed_spec.module@1
spec_version: "v02.206"
bundle_id: "master-spec-v02.206"
module_id: "14-30"
section_id: "14.30"
title: "14.30 Operator Shell — Menu, Availability, Docks, and Gesture Arbitration"
status: "STAGED_DRAFT_NOT_IN_ACTIVE_MANIFEST"
supersedes_section: "NONE — new sub-section. Extends 14.16, 14.20, 14.21 and 14.22 of .GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md; restates [STU-UNI-002] as a clarification."
source_baseline_version: "v02.205"
source_baseline_path: ".GOV/spec/master-spec-v02.205/spec-modules/14-studio-creative-suite.md"
provenance_sidecar: "14-30-operator-shell.provenance.json"
body_sha256: "ASSIGNED_AT_BUNDLE_ASSEMBLY"
metadata_rule: "frontmatter is machine metadata; body follows after this block"
---

## 14.30 Operator Shell — Menu, Availability, Docks, and Gesture Arbitration

This sub-section is the normative contract for the Studio operator shell: the menu bar Studio contributes into, the single availability predicate that decides what applies right now, the slot resolver that decides which of several applicable surfaces occupies a single-occupant slot, the dock and panel model, the pointer-gesture arbitration rules, and the accessibility and observability contract that makes all of it inspectable and steerable out of process. 14.31 specifies the tool palette, the options surface and the scrubbable numeric control that live inside this shell. 14.32 specifies the tooltip, help and UserManual generation contract that gives every surface here its prose and its manual anchor.

**[STU-SHL-001] Standing of this sub-section.** This sub-section is product LAW under [STU-SECTION-001] and is subject to the storage override in [STU-SDB-001] through [STU-SDB-009] and to the canonical contracts in 14.23. It ADDS to the cross-cutting sub-sections rather than replacing them: [STU-MDL-001] through [STU-MDL-006], [STU-QUIET-001] through [STU-QUIET-005], [STU-UNI-001] through [STU-UNI-005], [STU-MAN-001] through [STU-MAN-004] and [STU-CON-007] all remain in force unchanged. Where a clause here is more specific than a clause it extends, the more specific clause governs the surface it names and the general clause continues to govern everything else. This sub-section contains no clause that weakens a v02.205 obligation.

**[STU-SHL-002] One vocabulary (normative, closed).** Every Studio artefact — spec clause, work packet, microtask, command id, panel id, test name, manual entry — MUST use the canonical name in the left column and MUST NOT use any name in the right column. Using a superseded name is a conformance defect, not a style preference.

| Canonical Studio name | Meaning | Superseded names that MUST NOT be used |
|---|---|---|
| **Layout Preset** | a named arrangement of docks, Tool Rail order and search bias that gates NOTHING | task mode, workspace, named preset, persona |
| **document profile** | the declared shape of a document: a set of container kinds plus a set of feature flags | document type enum, document_capability_profile, context vector, document_types |
| **profile signature** | the sorted set of a document's declared container kinds; the layout persistence key | document_type (as a persistence key) |
| **availability predicate** | the single three-valued evaluation deciding whether an element applies right now | requires/disabled two-valued gate, can_bind veto (as a standalone gate) |
| **availability_state** | the predicate's result: `AVAILABLE`, `INAPPLICABLE_HERE`, `NOT_IN_THIS_DOCUMENT` | enabled/disabled, hidden |
| **slot resolver** | the ranking that decides which AVAILABLE candidate occupies a single-occupant slot | contextual_binding_rule, properties_panel dispatch |
| **Context Bar** | the horizontal strip under the tab strip and above the canvas, with a Tool Zone and a Selection Zone | options bar, control bar, tool option bar (as the name of the whole region) |
| **UiDescriptor** | the one record backing a menu leaf, a palette row, a tooltip, an AccessKit node and a manual anchor | AppCommand (as the final shape), command registry record |
| **ParamSpec** | the generated numeric contract for one parameter | captured parameter metadata, value_field_contract |
| **ScrubValue** | the scrubbable numeric widget | value field, number box, studio.field.* (as a type name) |
| **Task Scope** | a scoped, deliberately entered and deliberately exited tool-plus-options set | interactive effect workspace, taskspace, mode |

**[STU-SHL-003] Region names (normative, closed).** The shell has exactly seven addressable regions. Their `region_id` values are stable identifiers and their display names are the operator-facing labels.

| region_id | Display name | Kind |
|---|---|---|
| `top` | Context Bar | two-zone single-occupant strip |
| `left-rail` | Tool Rail | single-occupant rail |
| `left` | Browse Dock | group stack |
| `right` | Inspect Dock | group stack |
| `right-rail` | Meter Rail | single-occupant rail |
| `bottom` | Time & Results Dock | group stack |
| `centre` | Viewport | unbounded `egui_tiles` tree of editor groups |

**[STU-SHL-004] Studio does not own a menu bar.** Studio MUST NOT create a second menu bar, a second command registry, a second navigation router, a second manual store or a second layout persistence schema. It contributes into the surfaces the shell already ships. Specifically: the shell menu bar carries eight menus — FILE, EDIT, VIEW, GO, RUN, HELP, EDITORS, OPERATOR — at fixed accessibility node ids in the reserved band with Alt mnemonics; Studio contributes leaves into FILE, EDIT, VIEW and HELP and inserts eleven module menus between VIEW and GO while a pane showing the Studio module has focus. EDITORS, OPERATOR, GO and RUN are untouched by Studio.

**[STU-SHL-005] Module switch is a viewport swap, never a teardown.** Selecting Studio MUST swap the operator's work area to the Studio viewport while every other module keeps running with live state. Studio documents MUST be modelled as a SINGLE shell pane kind (`StudioViewport`) whose content identifier carries the active document, so the shell's module tab-list machinery observes exactly one stable tab and Studio's own document tab strip manages the document set independently. A module switch MUST NOT rewrite, close or reorder the Studio document set. Acceptance: a regression test that switches away from Studio and back with three Studio documents open and asserts all three survive with their per-document state.

**[STU-SHL-006] No new routing machinery.** Studio MUST register its addresses and navigation targets in the navigation layers that already exist — the address type, the navigation bus, the model-driven navigation surface, the module rail, the backend client and the JSON-RPC model tool dispatch. Opening a Studio attachment from another module MUST be one additional navigation-target variant, not a parallel bus. Merging the shell's routing layers is explicitly out of Studio scope.

**[STU-SHL-007] Storage law.** Every durable artefact this sub-section introduces — dock layouts, Layout Presets, panel registry rows, descriptor sets, manual seed rows, shortcut sets — MUST persist through Handshake-managed SurrealDB with the EventLedger as sole durable authority, embedded (no separate database process), per [STU-SDB-002]. SQLite, libSQL, Turso and PostgreSQL MUST NOT appear anywhere in the Studio dependency, test, cache or acceptance surface, per [STU-OVR-003]. Operator-scoped preference layers persist through the shell preference client; document-scoped layers persist as a Studio-owned sibling schema discriminated by `schema_id`, never by extending the shell's own layout schema.

**[STU-SHL-008] Naming law binding.** [STU-SECTION-003] governs every identifier in this sub-section. No vendor product name is a Studio menu title, submenu, leaf, panel, region, tool, family, Layout Preset, Task Scope, command id or manual anchor. Vendor names appear in this sub-section ONLY as capture provenance in a `provenance` field or in an explanatory note, never as a Studio name.

**[STU-SHL-009] Observability and quiet operation.** Every surface specified here MUST be discoverable, inspectable and steerable out of process through the AccessKit tree by stable `author_id` per [STU-MDL-002] and [STU-MDL-004], and MUST satisfy [STU-QUIET-001] through [STU-QUIET-005]: no foreground window, no focus steal, no OS-level input injection, no cursor warping, no pane popping while an operator or another agent works elsewhere. `handshake_core` MUST NOT gain `wgpu`, WGSL or GPU-compute dependencies for any surface in this sub-section ([STU-ARC-002]); all GPU work stays behind the six engine traits in `studio-engine`.

---

### 1. The menu

#### 1.1 Position, order and the index promise

**[STU-SHL-010] Menu bar composition.** The Studio menu bar is the shell menu bar with fifteen Studio-touched titles: four extended (FILE, EDIT, VIEW, HELP) and eleven inserted (DOCUMENT, SELECT, OBJECT, TYPE, COLOR, EFFECTS, MOTION, INSERT, CODE, AUTOMATE, WORKSPACE). Insertion is conditional on a pane showing the Studio module holding focus; the shell's own titles are never removed while Studio menus are present.

**[STU-SHL-011] Top-level order (normative, closed).** Exactly nineteen titles in exactly this left-to-right order:

`FILE`, `EDIT`, `VIEW`, `DOCUMENT`, `SELECT`, `OBJECT`, `TYPE`, `COLOR`, `EFFECTS`, `MOTION`, `INSERT`, `CODE`, `AUTOMATE`, `WORKSPACE`, `GO`, `RUN`, `HELP`, `EDITORS`, `OPERATOR`.

**[STU-SHL-012] Order is fixed.** The order in [STU-SHL-011] MUST NOT be reordered by a Layout Preset, by the document profile, by usage frequency, by a preference or by an adaptive heuristic. Positional stability is what makes the bar scriptable, learnable and documentable. Width is a rendering concern only: at narrow widths Studio MUST abbreviate titles and then overflow trailing titles into an order-preserving chevron, and MUST still emit every title into the AccessKit tree. Solving width by hiding titles is forbidden — it trades a rendering problem for an index-stability problem and breaks stable-target addressing.

**[STU-SHL-013] The index promise.** The menu tree is the EXHAUSTIVE index of Studio capability. Every capability is reachable from this tree, including capabilities that also appear in a panel, in a Context Bar zone, in the Tool Rail or in a context menu, and including capabilities with no other home. A capability that exists in the command registry and has no menu path, and is not explicitly marked `palette_only` with a typed reason, is a REGISTRY BUG and MUST fail the build. Panels, zones, rails and context menus are PROJECTIONS of menu leaves; they are never homes for capability the menu lacks.

**[STU-SHL-014] Context menus are references, never homes.** No Studio command may exist only in a context menu. Every context-menu entry MUST be a reference to a menu-bar leaf id and MUST dispatch the identical command id. (Provenance for why the rule is needed: in the captured suites, one page-layout application carries 448 leaf paths across its right-mouse roots against 245 in its main menu, and one web authoring application ships 88 context menu bars against one main menu bar, leaving 1,210 of its 2,176 invocable entries context-only.)

**[STU-SHL-015] One registry, two projections, zero drift.** The menu tree MUST NOT be a parallel structure that references commands. The menu tree IS a field on the command record. The shell's `AppCommand` is widened into the UiDescriptor of [STU-SHL-240] with `menu_path`, `menu_order`, `domain`, `requires`, `capability`, `menu_only`, `shortcut_id`, `manual_anchor`, `provenance`, `availability` and the ParamSpec block; all existing fields keep their meaning and all new fields default compatibly so pre-existing registry tests remain valid. A `menu_tree()` function folds the command set by `menu_path` in a second `OnceLock`. There is no hand-written Studio menu enum: the shell's menu action type becomes a single `Dispatch(command_id)` variant and the per-leaf variants collapse. A swarm agent dispatching a command id and an operator clicking its menu leaf execute the identical arm.

**[STU-SHL-016] No parallel Studio command registry.** Studio MUST NOT create a richer `STUDIO_COMMANDS` table beside the shell registry. Two registries means the menu, the palette and the Tool Rail can disagree, which breaks the index promise directly.

**[STU-SHL-017] Menu invariants (normative).** All seven MUST hold and each MUST be a build-time or test-time check.

| Id | Invariant |
|---|---|
| MENU-INV-1 | Every command is reachable from a `menu_path` OR is explicitly marked `palette_only` with a typed reason. There is no third state. |
| MENU-INV-2 | Every menu leaf resolves to exactly one command id, and every command id appears at most once as a PRIMARY leaf. Additional appearances carry `alias_of`. |
| MENU-INV-3 | Every registry domain is reachable from the menu tree. |
| MENU-INV-4 | No command may be enabled in one projection and disabled in another for the same document state. One availability evaluation feeds every renderer. |
| MENU-INV-5 | Every menu leaf and every palette row carries a stable AccessKit `author_id`, addressed as `menu.{command_id}`, so it survives a menu reorganisation. |
| MENU-INV-6 | The palette never invents a command the menu lacks, and the menu never contains a leaf the palette cannot find. |
| MENU-INV-7 | Every tool in the tool registry ([STU-SHL-140]) appears at `WORKSPACE > Tools > <family> > <tool>` and its menu leaf dispatches the identical command id the Tool Rail dispatches. |

#### 1.2 The never-hide rule

**[STU-SHL-018] The menu NEVER hides.** A menu node — a top-level title, a submenu or a leaf — MUST be rendered at every level regardless of its `availability_state`. `AVAILABLE` renders enabled. `INAPPLICABLE_HERE` renders greyed with its reason and its remedy. `NOT_IN_THIS_DOCUMENT` renders greyed with its reason. No node is ever removed, collapsed away, or conditionally omitted. The reason this rule holds for the menu and not for the Tool Rail is structural, not a compromise:

1. A hidden node has no stable AccessKit address. Argus and every out-of-process inspector address surfaces by `author_id`, and an element that is not rendered emits no node, so a test can only infer its state from the absence of an effect ([STU-MDL-002]).
2. An index whose entries disappear is not an index. The menu's whole contract under [STU-SHL-013] is exhaustiveness; conditional membership destroys it.
3. The menu is the address space that the UserManual, the command palette, the shortcut editor and the model command surface all resolve against. An address space with conditional membership cannot be documented or reverse-looked-up.

The Tool Rail is a roughly twenty-slot ergonomic surface over 362 tools and was never an index, so it may omit ([STU-SHL-049]). A dock is an arrangement and the AccessKit tree behind it is the address space, so the dock may omit what the tree must still carry.

**[STU-SHL-019] Machine-readable `disabled_reason`.** Every node whose `availability_state` is not `AVAILABLE` MUST carry a machine-readable reason record and that record MUST be readable through all THREE paths, with identical content:

| Path | Shape |
|---|---|
| tooltip (L1, [STU-SHL-232]) | the rendered reason sentence plus the one-click remedy control |
| AccessKit node | `disabled = true`, plus `availability_state`, `reason_code`, `reason_text` and `remedy_command_id` as node properties |
| command API | the command's typed availability response carries `availability_state`, `reason_code`, `reason_text`, `remedy_command_id` and the profile clause that produced the state |

A reason rendered in one path and absent from another is a conformance defect. A reason that exists only as prose in a tooltip is a conformance defect; `reason_code` is a closed enumeration ([STU-SHL-047]) and the prose is generated from it.

**[STU-SHL-020] Enable by promotion, never by refusal.** Every `INAPPLICABLE_HERE` state MUST carry a `remedy_command_id` naming ONE enabled command that would make the predicate true. The menu renders the remedy as an enabled sibling leaf; the Tool Rail and the tooltip render it as an inline button; the command API returns it in the availability response. Worked examples that MUST hold: in a page-layout document `MOTION > Keyframes` is `INAPPLICABLE_HERE` with `NO_SUCH_CONTAINER` and `DOCUMENT > Timelines > New Timeline` is the enabled remedy; the node tool on a pixel layer is `INAPPLICABLE_HERE` with `WRONG_LAYER_KIND` and "convert this pixel layer to curves" is the remedy.

**[STU-SHL-021] Depth cap.** Maximum explicit depth below a top-level title is 3. Maximum depth including registry expansion is 4. Studio MUST NOT exceed 4.

#### 1.3 The full menu tree

**[STU-SHL-022] Tree notation.** Each entry below is `Label | node kind | command id`. `submenu` nodes list their children. `expansion` nodes name their leaf source and their leaf count and are governed by [STU-SHL-037]. A trailing `…` marks a leaf that opens a dialog. This tree is the DESIGNED surface: 1,533 explicit leaves, 203 submenus, 375 menu-only nodes, plus registry expansion.

**[STU-SHL-023] FILE (extends the shell FILE menu).**

- `New` | submenu | `studio.document.new` → Blank Document…, From Document Preset…, From Template…, From Clipboard, From Asset Library…, Duplicate of Current Document, Document Presets…
- `Open…` | item | `studio.document.open`
- `Open Recent` | submenu | `studio.document.open_recent` → *(recent documents, generated)*
- `Open From Asset Library…` | item | `studio.document.open_from_asset_library`
- `Browse Assets…` | item | `studio.document.browse_assets`
- `Close` / `Close All` / `Close Others` | items | `studio.document.close` / `.close_all` / `.close_others`
- `Save` / `Save As…` / `Save a Copy…` / `Save as Template…` / `Save All` / `Revert` | items | `studio.document.save` / `.save_as` / `.save_copy` / `.save_template` / `.save_all` / `.revert`
- `Place` | submenu | `studio.interop.place` → Place Linked…, Place Embedded…, Place from Asset Library…, Place Multiple…, Relink…, Relink to Folder…, Update All Links, Embed Link, Unembed…, Edit Original, Go to Link, Link Info…
- `Import` | expansion | `studio.interop.import` — leaf source: import-format registry
- `Export` | expansion | `studio.interop.export` — leaf source: export-format registry
- `Export Recipes` | submenu | `studio.interop.recipe` → Manage Export Recipes…, Save Current as Recipe…, Run Recipe…, Export Queue
- `Publish` | submenu | `studio.web.publish` → Publish Site…, Put Selected Files, Get Selected Files, Check In, Check Out, Undo Check Out, Synchronize Sitewide…, Cloaking
- `Site Management` | submenu | `studio.web.site` → New Site…, Manage Sites…, Site Setup…, Server Setup…, Site Reports…, Check Links Sitewide, Change Link Sitewide…, Recreate Site Cache, Advanced
- `Package` | submenu | `studio.layout.package` → Package Document…, Package Options…, Copy Linked Assets, Copy Fonts, Include Report, Package for Print Provider…
- `Preflight` | submenu | `studio.layout.preflight` → Run Preflight, Preflight Profiles…, Define Profile…, Embed Profile in Document, Preflight Report…, Preflight Panel
- `Print` | submenu | `studio.layout.print` → Print…, Print Presets…, Print Booklet…, Print Tiles…, Printer Marks & Bleed…, Page Setup…, Print Selection Only
- `Document Info` | submenu | `studio.document.info` → Document Properties…, File Metadata…, Document Statistics…, Fonts Used…, Links Used…, Colours Used…
- `Version History` | submenu | `studio.history.version` → Browse Versions…, Create Named Version…, Restore Version…, Compare Versions…, Ledger Receipts for This Document…

**[STU-SHL-024] EDIT (extends the shell EDIT menu).**

- `Undo` / `Redo` | items | `studio.history.undo` / `.redo`
- `History` | submenu | `studio.history` → Step Backward, Step Forward, History Panel, New Snapshot…, Delete Snapshot, Restore Snapshot, Purge History, History Options…
- `Cut` / `Copy` / `Copy Merged` | items | `studio.clipboard.cut` / `.copy` / `.copy_merged`
- `Copy As` | submenu | `studio.clipboard.copy_as` → SVG, CSS, PNG, Code, Command JSON, Object Id
- `Paste` | item | `studio.clipboard.paste`
- `Paste Special` | submenu | `studio.clipboard.paste_special` → Paste in Place, Paste in Front, Paste Behind, Paste Into, Paste Outside, Paste Without Formatting, Paste on All Artboards, Paste Attributes, Paste Effects, Paste Develop Settings
- `Copy with Property Links` / `Copy with Relative Property Links` / `Copy Expression Only` / `Paste Reversed Keyframes` | items | `studio.clipboard.copy_property_links` / `.copy_property_links_relative` / `.copy_expression` / `.paste_keyframes_reversed` — these are clipboard verbs over the expression graph and belong in EDIT, not in MOTION
- `Clear` / `Duplicate` / `Duplicate to Document…` / `Repeat Last Command` | items
- `Find & Replace` | submenu | `studio.find` → Find / Change…, Find Next, Find Previous, Find in All Open Documents, Find in Site / Folder…, Regex Find…, Find Font…, Find Colour…, Find Object…, Find Glyph…, Query Presets…, Recent Searches, Replace All in Selection
- `Spelling & Language` | submenu | `studio.typography.spelling` → Check Spelling…, Dynamic Spelling, Autocorrect, Ignore Spelling, Add to User Dictionary, User Dictionary…, Hyphenation Exceptions…, Language…
- `Track Changes` | submenu | `studio.layout.changes` → Enable Track Changes, Show Changes, Accept Change, Reject Change, Accept All Changes, Reject All Changes, Accept All by Author…, Reject All by Author…, Next Change, Previous Change, Change Info…
- `Notes & Comments` | submenu | `studio.layout.notes` → New Note, New Comment, Next Note, Previous Note, Convert Note to Text, Remove All Notes, Resolve Comment, Notes Mode
- `Assignments` | submenu | `studio.interop.assignment` → Add Selection to Assignment, Add All Stories to Assignment, Add All Graphics to Assignment, Add Layer to Assignment, New Assignment…, Check In / Check Out, Update Assignment, Package Assignment…
- `Preferences` | expansion | `studio.app.preferences` — leaf source: preference registry; structurally menu-only ([STU-SHL-040])
- `Keyboard & Menus` | submenu | `studio.app.customise` → Keyboard Shortcuts…, Menu Customisation…, Toolbar Customisation…, Import Shortcut Set…, Export Shortcut Set…, Reset Shortcuts to Default, Shortcut Conflict Report…, Keyboard Layout…
- `Capability Grants` | submenu | `studio.app.capability` → Review Capability Grants…, Grant Capability…, Revoke Capability…, Capability Audit Log…

**[STU-SHL-025] VIEW (extends the shell VIEW menu).**

- `Zoom In` / `Zoom Out` | items
- `Zoom To` | submenu | `studio.view.zoom_to` → Fit Page in Window, Fit Spread in Window, Fit Artboard in Window, Fit All Artboards, Fit Selection in Window, Actual Size, 200%, 400%, Zoom To…, Zoom to Pixel Grid
- `Screen Mode` | submenu | `studio.view.screen_mode` → Normal, Preview, Bleed, Slug, Presentation, Full Screen, Trim View
- `Display Quality` | submenu | `studio.view.display` → Fast, Typical, High Quality, Pixel Preview, Outline / Wireframe, Overprint Preview, GPU Preview, Allow Object-Level Display Settings, Clear Object-Level Display Settings. For documents carrying a `timeline` container this submenu additionally exposes the six-member layer-quality enumeration.
- `View Layout` | submenu | `studio.view.view_layout` → 1 View, 2 Views, 4 Views, Share View Options. This is a VIEWER-INTERNAL split of ONE document at ONE time sharing one playhead. It is NOT the centre editor tree of [STU-SHL-063] and MUST NOT be implemented as one.
- `Proof` | submenu | `studio.color.proof` → Proof Setup…, Proof Colours, Gamut Warning, Colour-Vision Proof…
- `Rulers & Measurement` | submenu | `studio.view.rulers` → Show Rulers, Ruler Units…, Change Ruler Origin, Reset Ruler Origin, Video Ruler, Measurement Tool Overlay
- `Guides` | submenu | `studio.view.guides` → Show Guides, Lock Guides, Clear Guides, New Guide…, New Guides from Selection, Guides in Back, Smart Guides, Guide Templates, Manage Guides…
- `Grids` | submenu | `studio.view.grids` → Show Document Grid, Show Baseline Grid, Show Layout Grid, Show Pixel Grid, Snap to Document Grid, Snap to Baseline Grid, Grid Setup…
- `Snap To` | submenu | `studio.view.snap` → Snap to Guides, Snap to Grid, Snap to Objects, Snap to Point, Snap to Pixel, Snap to Glyph, Snap to Timeline Frames, Snapping Options…
- `Extras` | submenu | `studio.view.extras` → Show Extras, Show Edges, Show Frame Edges, Show Text Threads, Show Hidden Characters, Show Bounding Box, Show Transparency Grid, Show Slices, Show Annotations, Show Live Corner Widgets, Show Gaps, Extras Options…
- `Rotate View` | submenu | `studio.view.rotate` → Rotate 90° CW, Rotate 90° CCW, Rotate 180°, Rotate View…, Reset Rotation
- `Code View Options` | submenu | `studio.code.view_options` → Word Wrap, Line Numbers, Syntax Colouring, Auto Indent, Highlight Invalid Code, Hidden Characters, Code Folding
- `Live Preview` | submenu | `studio.web.preview` → Live View, Real-time Preview, Preview in Browser…, Preview in Secondary Browser, Device Preview…, Responsive Widths…, Manage Browsers…
- `Related Files` | submenu | `studio.code.related` → Show Related Files, Next Related File, Previous Related File, Related Files Options…
- `Video Overlays` | submenu | `studio.video.overlays` → Safe Margins, Title Safe, Action Safe, Transparency Grid, Broadcast Range Warning, Scopes Overlay
- `Story Editor` | submenu | `studio.typography.story_editor` → Open Story Editor, Story Editor Display…, Show Depth Ruler, Show Style Name Column, Show Paragraph Break Marks
- `Structure View` | item | `studio.document.structure_view`

**[STU-SHL-026] DOCUMENT (Studio module menu).**

- `Document Setup…`, `Canvas Size…`, `Image Size…`, `Resample…`, `Crop to Selection`, `Trim…`, `Reveal All` | items
- `Rotate Document` | submenu → 90° Clockwise, 90° Counter-Clockwise, 180°, Arbitrary…, Flip Horizontal, Flip Vertical
- `Colour Mode` | submenu → Bitmap, Greyscale, Duotone…, Indexed Colour…, RGB, CMYK, Lab, Multichannel
- `Bit Depth` | submenu → 8 bits / channel, 16 bits / channel, 32 bits / channel, HDR Toning…
- `Artboards` | submenu → New Artboard, New Artboard from Selection, Duplicate Artboard, Delete Artboard, Delete Empty Artboards, Artboard Options…, Rearrange All Artboards…, Fit Artboard to Artwork, Rename Artboard…, Go to Artboard…, Convert Selection to Artboard, Artboard Presets…
- `Pages & Spreads` | submenu → Insert Pages…, Move Pages…, Duplicate Spread, Delete Pages…, Allow Document Pages to Shuffle, Allow Selected Spread to Shuffle, Go to Page…, First Page, Previous Page, Next Page, Last Page, Next Spread, Previous Spread, Page Transitions…, Page Size…
- `Parent Pages` | submenu → New Parent…, Apply Parent to Pages…, Save as Parent, Load Parents…, Override All Parent Page Items, Detach Selection from Parent, Remove All Local Overrides, Parent Page Options…, Go to First Parent
- `Sections & Numbering` | submenu → Numbering & Section Options…, Start Section, End Section, Section Marker…, Page Numbering Style…
- `Liquid Layout` | submenu → Adjust Layout…, Liquid Page Rule…, Create Alternate Layout…, Alternate Layout Options…
- `Long Document` | submenu → Table of Contents, Index, Cross-References, Notes & References, Book, Text Variables, Conditional Text
- `Compositions` | submenu | `studio.motion.composition` → New Composition, Composition Settings…, Crop Composition to Selected Layer Bounds, Crop Composition to Region of Interest, Composition Flowchart, Composition Mini-Flowchart, Pre-render, Preview, Save Frame As…, Save Current Preview, Set Poster Time, Responsive Time Settings…, Trim Composition to Work Area, Nest Composition. *(The two render-queue entries of the vendor's equivalent menu map to `FILE > Export` and to the `render-queue` panel, not here.)*
- `Timelines` | submenu | `studio.motion.timeline_doc` → New Timeline, Timeline Settings…, Duration…, Frame Rate…, Resolution…, Nest Timeline, Timeline Presets…, Delete Timeline, Timeline from Selection
- `Boards` | submenu → New Board, Board Settings…, Infinite Canvas, Facilitation Timer…, Voting Session…, Follow Presenter, Board Templates…
- `Site Tree` | submenu → New Page, New Folder, Page Properties…, Document Type…, Apply Template to Page…, Detach from Template, Make Template, Editable Regions…, Site Map
- `Develop` | submenu → Open in Develop, Reset Develop, Copy Develop Settings, Paste Develop Settings, Sync Develop Settings…, Auto Tone, Auto White Balance, Snapshots, Process Version, Camera Profile, Enhance, Develop Presets…, Workflow Options…, Apply to Raster Document
- `Grids & Guides Setup` | submenu → Margins & Columns…, Baseline Grid…, Document Grid…, Layout Grid…, Bleed & Slug…, Frame Grid…
- `Fonts` | submenu → Manage Fonts…, Document Fonts…, Missing Fonts…, Font Substitution Rules…, Package Fonts, Add Web Font…
- `Accessibility & Structure` | submenu → Alt Text…, Articles / Reading Order…, Tag Structure…, Export Tagging…, Accessibility Check…, Contrast Report…, XML Structure…
- `Document Metadata…` | item
- `Document Profile…` | item | `studio.document.profile` — opens the declared container kinds and feature flags of [STU-SHL-043]

**[STU-SHL-027] SELECT (Studio module menu).**

- `All`, `All on Active Artboard`, `All on Spread`, `Deselect`, `Reselect`, `Inverse` | items
- `Select By` | submenu → Layer Kind…, Fill Colour, Stroke Colour, Stroke Weight, Blending Mode, Opacity, Graphic Style, Object Style, Font, Paragraph Style, Component / Instance, Tag or Label…, Same Appearance, Similar Objects
- `Modify` | submenu → Grow, Shrink, Border…, Smooth…, Expand…, Contract…, Feather…, Refine Edge…
- `Select By Content` | submenu → Colour Range…, Luminance Range…, Focus Area…, Subject, Sky, Transparent Areas
- `Selection From` | submenu → Layer Transparency, Layer Mask, Vector Path, Channel…, Text, Guides, Artboard Bounds
- `Selection Sets` | submenu → Save Selection…, Load Selection…, Edit Selection Sets…, Delete Selection Set
- `Navigate Selection` | submenu → Next Object Above, Next Object Below, First Object, Last Object, Select Parent, Select Child, Select Siblings, Select Parent Tag
- `Edit in Isolation`, `Exit Isolation`, `Quick Mask Mode`, `Edit Selection as Mask`, `Lock Selection`, `Select All Unlocked` | items

**[STU-SHL-028] OBJECT (Studio module menu).**

- `Transform` | submenu → Free Transform, Move…, Scale…, Rotate…, Shear…, Reflect…, Flip Horizontal, Flip Vertical, Rotate 90° CW, Rotate 90° CCW, Rotate 180°, Distort, Perspective, Skew, Warp…, Puppet Warp, Content-Aware Scale, Transform Again, Transform Each…, Reset Transform, Transform Reference Point…
- `Arrange` | submenu → Bring to Front, Bring Forward, Send Backward, Send to Back, Send to Current Layer, Move to Artboard…, Move to Page…
- `Align & Distribute` | expansion | `studio.object.align`
- `Group` / `Ungroup` | items
- `Lock` | submenu → Lock Selection, Unlock All, Lock Others, Lock Position, Lock Guides
- `Hide` | submenu → Hide Selection, Show All, Hide Others
- `Rename…` | item
- `Object Label / Script Id…` | item — menu-only ([STU-SHL-040]): an automation-addressing concern with no visual representation
- `Layer` | submenu | `studio.object.layer` → New, Duplicate Layer…, Delete Layer, Merge Down, Merge Visible, Merge Selected, Flatten Document, Rasterise, Convert To, Blending Mode *(39-member enumeration)*, Track Matte *(10-member enumeration)*, Opacity…, Fill Opacity…, Knockout, Isolate Blending, Frame Blending *(4-member enumeration)*, Layer Colour Tag…, Layer Comps…, Layer States…, 3D Layer, Guide Layer, Environment Layer, Layer Styles, Layer Settings…, Material, Camera, Light, Pre-compose…, Auto-trace…, Create Shapes from Vector Layer, Open Layer, Open Layer Source, Add Marker
- `Mask` | submenu | `studio.object.mask` → New Mask, Add Layer Mask, Add Vector Mask, Reveal All, Hide All, From Selection, From Transparency, Make Clipping Mask, Release Clipping Mask, Apply Mask, Disable Mask, Delete Mask, Invert Mask, Refine Mask…, Link/Unlink Mask, Edit Mask, Mask Density & Feather…, Mask Shape…, Mask Feather…, Mask Opacity…, Mask Expansion…, Mode *(8-member enumeration)*, Inverted, Locked, Closed, RotoBezier, Motion Blur, Feather Falloff, Free Transform Points, Set First Vertex, Lock Other Masks, Hide Locked Masks, Unlock All Masks, Remove Mask, Remove All Masks, Reset Mask
- `Path` | submenu → Join, Average…, Average and Join, Outline Stroke, Offset Path…, Simplify…, Add Anchor Points, Remove Anchor Points, Convert Point, Split Path at Selected Points, Reverse Path Direction, Clean Up…, Close Path, Open Path, Path Direction…, Convert to Vector Network
- `Boolean` | submenu → Union, Subtract Front, Subtract Back, Intersect, Exclude, Divide, Trim, Merge, Crop, Outline, Hard Mix, Soft Mix…, Trap…, Make Compound Path, Release Compound Path, Repeat Last Boolean
- `Shape` | submenu → Convert Shape, Corner Options…, Live Corners, Expand…, Expand Appearance, Edit Shape Parameters…, Reset Shape
- `Frame & Fitting` | submenu → Frame Type, Fitting, Content, Anchored Object, Clipping Path, Caption, Text Frame Options…, Text Wrap…, Flex Container Options…
- `Table` | expansion | `studio.layout.table`
- `Component` | submenu → Create Component, Create Component Set, Add Variant, Component Properties…, Detach Instance, Swap Instance…, Reset All Overrides, Go to Main Component, Publish to Library…, Update from Library, Library Manager…, Break Symbol Link
- `Auto Layout` | submenu → Add Auto Layout, Remove Auto Layout, Direction: Horizontal, Direction: Vertical, Direction: Wrap, Spacing…, Padding…, Alignment…, Absolute Position, Resizing…
- `Constraints` | submenu → Left, Right, Top, Bottom, Centre, Scale, Stretch, Constraint Options…
- `Layout Grid` | submenu → Add Layout Grid, Remove Layout Grid, Columns…, Rows…, Grid…, Save Grid Style…
- `Blend` | submenu → Make Blend, Release Blend, Blend Options…, Expand Blend, Replace Spine, Reverse Spine, Reverse Front to Back
- `Procedural` | submenu → Live Paint, Image Trace, Gradient Mesh, Envelope Distort, Repeat, Intertwine, Pattern, Global Edit, Offset Path Repeat…
- `Clip` | submenu → Speed / Duration…, Enable, Link / Unlink, Synchronize…, Nest…, Make Subclip…, Merge Clips…, Multi-Camera, Replace, Render, Restore, Source Settings…, Interpret Footage…, Modify Audio Channels…, Frame Hold Options…, Scale to Frame Size, Set to Frame Size
- `Styles & Appearance` | submenu → Object Styles…, Graphic Styles…, Create Style from Selection, Redefine Style, Break Link to Style, Load Styles…, Appearance: Add New Fill, Appearance: Add New Stroke, Appearance: Clear Appearance, Appearance: Reduce to Basic, Copy Appearance, Paste Appearance
- `Object Export Options…`, `Object Metadata…`, `Capture Appearance from Selection` | items

**[STU-SHL-029] TYPE (Studio module menu).**

- `Font` | submenu → *(installed font families, generated)*, Recent Fonts, Favourite Fonts, Font Filter…, Add to Favourites, Font Preview Size…
- `Size` | submenu → *(preset size ladder)*, Other Size…, Increase Size, Decrease Size, Restore Default Size
- `Character` | submenu → Kerning, Tracking…, Leading…, Baseline Shift…, Horizontal Scale…, Vertical Scale…, Skew…, Case, Position, Underline & Strike, Bold, Italic, No Break, Ligatures, Language…, Character Colour…
- `Paragraph` | submenu → Align, Indents…, Space Before / After…, Drop Caps…, Hyphenation…, Justification…, Keep Options…, Span / Split Columns…, Balance Ragged Lines, Paragraph Rules…, Paragraph Shading…, Paragraph Borders…, Composer, Bullets & Numbering, Tabs…
- `OpenType` | expansion | `studio.typography.opentype`
- `Variable Fonts` | submenu → Axes…, Named Instances, Reset Axes, Create Named Instance…
- `Styles` | submenu → Character Styles…, Paragraph Styles…, Table Styles…, Cell Styles…, Create Style from Selection, Redefine Style, Break Link to Style, Clear Overrides, Load Styles from Document…, Next Style…, Style Conflict Report…
- `Insert Special Character` | submenu → Symbols, Markers, Hyphens & Dashes, Quotation Marks, Other, Unicode Value…
- `Insert White Space` | submenu → Em Space, En Space, Non-breaking Space, Thin Space, Hair Space, Flush Space, Figure Space, Punctuation Space, Sixth / Quarter / Third Space
- `Insert Break` | submenu → Line Break, Column Break, Frame Break, Page Break, Odd / Even Page Break, Paragraph Return, Discretionary Hyphen, Indent to Here, Right Indent Tab, Zero-Width Joiner / Non-Joiner
- `Glyphs` | submenu → Glyphs Panel, Alternates for Selection, Add to Glyph Favourites, Remove from Glyph Favourites, Glyph Sets…, Recently Used Glyphs
- `Type on a Path` | submenu → Make Type on a Path, Type on a Path Options…, Delete Type from Path, Flip Direction, Effect, Align to Path…, Spacing…
- `Threading` | submenu → Thread Text Frames, Unthread, Show Text Threads, Autoflow, Smart Text Reflow, Load Overset Text, Overset Report…
- `Convert` | submenu → Create Outlines, Convert to Point Text, Convert to Area Text, Convert to Frame Grid, Convert to Table, Convert Bullets to Text, Convert Numbering to Text
- `Story` | submenu → Optical Margin Alignment, Story Direction, Story Options…, Fill with Placeholder Text
- `CJK & Complex Scripts` | submenu → Mojikumi Settings…, Kinsoku Settings…, Kinsoku Break Type, Kinsoku Hang Type, Ruby…, Kenten…, Tate-Chu-Yoko…, Warichu…, Grid Alignment, Character Alignment, Character Direction, Keyboard Direction, Digits, CJK Leading Model, Writing Direction
- `Text Animators` | submenu | `studio.typography.animator` → Animate Property *(19-member enumeration: All Transform Properties, Anchor Point, Position, Scale, Rotation, Skew, Opacity, Blur, Character Offset, Character Value, Fill Colour, Stroke Colour, Stroke Width, Line Anchor, Line Spacing, Tracking, Enable Per-character 3D, Variable Font Axes, All Font Axes)*, Add Selector *(Range, Wiggly, Expression)*, Fill Animator Sub-properties *(Brightness, Hue, Opacity, RGB, Saturation, Colour)*, Stroke Animator Sub-properties *(same six)*, Remove All Text Animators, Text Path Options…, More Options…
- `Show Hidden Characters`, `Text Macros…` | items

**[STU-SHL-030] COLOR (Studio module menu).**

- `Colour Settings…` | item — menu-only
- `Working Spaces` | submenu → RGB Working Space…, CMYK Working Space…, Greyscale Working Space…, Spot Working Space…, Transparency Blend Space…
- `Colour Management Policies` | submenu → Policies…, Profile Mismatch Warnings, Missing Profile Warnings, Rendering Intent…, Black Point Compensation, Engine…
- `Document Profile` | submenu → Assign Profile…, Convert to Profile…, Embed Profile, Remove Profile, Install Profile…, Profile Info…
- `Proof` | submenu → Proof Setup…, Custom Proof Condition…, Proof Colours, Gamut Warning, Colour-Vision Proof…, Save Proof Preset…
- `Swatches & Colour Assets` | expansion | `studio.color.swatch`
- `Spot Colour & Ink` | submenu → New Spot Colour…, Convert Spot to Process, Convert Process to Spot, Ink Manager…, Ink Aliases…, Named Colour Libraries…, Overprint Fill, Overprint Stroke, Overprint Black…
- `Separations & Output` | submenu → Separations Preview, Overprint Preview, Flattener Preview, Flattener Presets…, Ink Limit Warning…, Trap Presets…, Total Area Coverage…
- `Recolour` | submenu → Recolour Artwork…, Colour Harmony…, Global Edit Colour, Adjust Colour Balance…, Blend Front to Back, Blend Horizontally, Blend Vertically, Convert to Greyscale, Convert to CMYK, Convert to RGB, Invert Colours, Saturate…
- `LUTs` | submenu → Apply LUT…, Load LUT…, Export LUT…, LUT Library…, Sort LUTs by Name, Remove LUT
- `Colour Picker & Sampling` | submenu → Colour Picker…, Eyedropper Options…, Sample Size…, Sample from Screen, Swap Fill and Stroke, Default Fill and Stroke, Copy Colour Value
- `Gradients & Patterns` | submenu → New Gradient…, Edit Gradient…, Gradient Type…, Reverse Gradient, Gradient Libraries…, New Pattern…, Pattern Libraries…
- `Colour Variables` | submenu → Create Colour Variable…, Bind Selection to Variable…, Variable Collections…, Mode Switch…, Detach Variable

**[STU-SHL-031] EFFECTS (Studio module menu).**

- `Last Effect`, `Last Effect Options…` | items
- `Apply Effect` | expansion | `studio.effect.apply` — leaf source: the effect registry, grouped at level 3 by effect FAMILY. The family level is MANDATORY; a flat effect list is unusable at this scale.
- `Adjustments` | expansion | `studio.effect.adjustment`
- `Layer Effects` | submenu → Drop Shadow…, Inner Shadow…, Outer Glow…, Inner Glow…, Bevel & Emboss…, Satin…, Colour Overlay…, Gradient Overlay…, Pattern Overlay…, Stroke…, Layer Blur…, 3D Effect…, Global Light…, Scale Effects…, Copy Layer Style, Paste Layer Style, Clear Layer Style, Create Layer from Style, Hide All Effects
- `Live Filters` | submenu → Add Live Filter…, Edit Live Filter…, Disable Live Filter, Rasterise Live Filter, Live Filter Mask…, Reorder Live Filters…
- `Effect Stack` | submenu → Reorder Effects…, Duplicate Effect, Delete Effect, Effect Opacity…, Effect Blend Mode…, Mask Effect with Selection, Disable All Effects, Reset All Parameters, Copy Effect Stack, Paste Effect Stack, Manage Effects…
- `Interactive Effect Workspaces` | submenu — each leaf enters a Task Scope ([STU-SHL-152]), not a Layout Preset → Liquify…, Vanishing Point…, Lens Correction…, Wide Angle Correction…, Raw Filter…, Blur Gallery…, Effect Gallery…, Displace…, Content-Aware Fill…, Select & Refine…
- `Effect Presets & Styles` | submenu → Save Effect Preset…, Apply Effect Preset…, Manage Effect Presets…, Import Presets…, Export Presets…, Effect Styles…
- `Effect Rendering` | submenu → GPU Rendering, CPU Fallback, Render Quality…, Purge Effect Cache, Cross-Backend Equivalence Report…
- `Convert for Non-Destructive Effects`, `Expand Appearance`, `Rasterise Effect` | items

**[STU-SHL-032] MOTION (Studio module menu).**

- `Playback` | submenu → Play / Pause, Play In to Out, Play Around Playhead, Loop, Shuttle Left / Right, Step Forward 1 Frame, Step Back 1 Frame, Playback Resolution…, Playback Options…
- `Go To` | submenu → Start of Timeline, End of Timeline, Next Edit Point, Previous Edit Point, Next Keyframe, Previous Keyframe, Next Marker, Previous Marker, In Point, Out Point, Time…
- `Keyframes` | expansion | `studio.motion.keyframe` — includes Set Keyframe, Toggle Hold Keyframe, Interpolation *(6-member enumeration: Linear, Bezier, Continuous Bezier, Auto Bezier, Hold, Current Settings)*, Keyframe Velocity…, Keyframe Assistant, Ease In, Ease Out, Ease Both, Nudge Earlier / Later, Select All Visible Keyframes, Deselect All Keyframes, Separate Dimensions
- `Expression` | submenu | `studio.motion.expression` → Add Expression, Toggle Expression, Convert Expression to Keyframes, Show Expression Errors, Pick Whip…, Expression Editor Panel. *(`Copy Expression Only` and the two property-link copies are clipboard verbs and live in EDIT.)*
- `Markers` | expansion | `studio.motion.marker`
- `In & Out` | submenu → Mark In, Mark Out, Mark Clip, Mark Selection, Clear In, Clear Out, Clear In and Out
- `Edit Actions` | submenu → Insert, Overwrite, Extract, Lift, Ripple Delete, Add Edit, Add Edit to All Tracks, Split at Playhead, Join Through Edits, Trim In / Out, Roll Edit, Slip, Slide, Close Gap, Trim Mode…
- `Tracks` | submenu → Add Track…, Delete Track…, Add Tracks to Match Source, Track Height, Lock Track, Sync Lock, Target Track, Mute / Solo, Track Output Toggle
- `Transitions` | submenu → Apply Default Video Transition, Apply Default Audio Transition, Apply Transition to Selection, Transition Duration…, Set Default Transition…, Transition Presets…
- `Speed & Time` | submenu → Speed / Duration…, Time Remap, Reverse, Freeze Frame…, Frame Blending, Optical Flow, Time Stretch…
- `Audio` | submenu → Audio Gain…, Normalise…, Audio Channels…, Generate Audio Waveform, Auto-Ducking…, Add Submix Track, Mute, Solo, Loudness Report…
- `Captions` | submenu → New Caption Track…, Import Captions…, Export Captions…, Caption Settings…, Merge Captions, Split Caption, Transcribe…, Caption Sidecar Settings…
- `Tracking` | submenu | `studio.motion.tracking` → Track Motion, Track Camera, Track Mask, Track This Property, Stabilise, Tracker Panel, Mask Tracking Options…
- `Render & Preview` | submenu → Render In to Out, Render Effects in Work Area, Render Audio, Render Entire Timeline, Delete Render Files, Preview Quality…, Cache Settings…, Render Queue Panel
- `Motion Presets` | submenu → Apply Animation Style…, Save Animation Style…, Manage Motion Presets…, Install Motion Template…, Browse Presets…, Recent Animation Presets
- `Motion Aids` | submenu → Onion Skin, Onion Skin Options…, Motion Blur, Motion Path Display, Ghosting…
- `Reveal` | submenu | `studio.motion.reveal` → Reveal Properties with Keyframes, Reveal All Modified Properties, Reveal Effects, Reveal Expressions, Extend Reveal to Selection. Every leaf here is the UNTIMED equivalent of a double-tap reveal gesture and MUST exist ([STU-SHL-109]).
- `Prototype` | submenu → Flows, Connections, Triggers, Actions, Animation, Overlays & Scroll, Presentation, Interactive Components…, Interactive Document Settings…
- `Enable Auto-Keyframe` | item | `studio.motion.auto_keyframe` — a document-level toggle read by [STU-SHL-202]

**[STU-SHL-033] INSERT (Studio module menu).**

- `Place…` | item
- `Layer` | submenu → Pixel Layer, Vector Layer, Text Layer, Adjustment Layer…, Fill Layer…, Live Filter Layer…, Mask Layer, Group, Frame, Artboard, Solid, Null, Camera…, Light…, Guide Layer, Adjustment Layer, Composition Layer
- `Shape` | expansion | `studio.insert.shape`
- `Text` | submenu → Point Text, Area Text, Text on Path, Text Frame, Frame Grid, Placeholder Text, Filler Text…
- `Table` | submenu → Insert Table…, Convert Text to Table…, Insert Rows…, Insert Columns…
- `Page Element` | submenu → Current Page Number, Next Page Number, Previous Page Number, Section Marker, Running Header…, Footnote, Endnote, Sidenote, Index Marker, Cross-Reference…, Hyperlink…, Bookmark…, Anchor…, Text Variable…, Conditional Text…
- `Component & Library` | submenu → Insert Component…, Insert Instance…, Insert from Library…, Insert Symbol…, Insert Variable…, Insert Style Token…
- `Media` | submenu → Image…, Video…, Audio…, Animated GIF…, 3D Model…, SVG…, PDF Page…, Barcode / QR Code…, Colour Bars & Tone, Counting Leader, Black Video / Transparent Video
- `Web Element` | expansion | `studio.insert.web`
- `Whiteboard Object` | submenu → Sticky Note, Connector, Section, Stamp, Vote Marker, Timer Widget, Diagram from Text…, Widget…
- `Snippet & Sample` | submenu → Insert Snippet…, New Snippet…, Recent Snippets, Sample Content…
- `Insert Marker`, `Insert Special Character` | items *(alias leaves; `alias_of` per MENU-INV-2)*

**[STU-SHL-034] CODE (Studio module menu).**

- `View Mode` | submenu → Code, Split, Design, Live, Split Vertically / Horizontally, Switch Views
- `Source Formatting` | submenu → Apply Source Formatting, Apply to Selection, Indent, Outdent, Balance Braces, Convert Tabs to Spaces, Code Format Settings…, Clean Up Markup…
- `Edit Code` | submenu → Comment Selection, Uncomment Selection, Wrap Tag…, Remove Tag, Quick Tag Editor…, Edit Tag…, Collapse Selection, Collapse Outside Selection, Expand All, Go to Line…, Toggle Breakpoint
- `Refactor` | submenu → Rename Symbol…, Extract to Style Sheet…, Extract to Include…, Move Inline Styles…, Convert Table to Layout…
- `Style Sheets` | submenu → New Rule…, Edit Rule…, Attach Style Sheet…, Design-Time Style Sheets…, Media Queries…, Transitions…, CSS Designer, Export Styles as CSS…
- `Validate` | submenu → Validate Markup, Validate Style Sheets, Browser Compatibility Check, Accessibility Check, Check Links, Check Spelling, Validation Report…
- `Assistance` | submenu → Code Hints, Show Code Hints Now, Code Hint Preferences…, Abbreviation Expansion, Snippets…, Live Code, Inspect Mode
- `Server` | submenu → Server Behaviours…, Bindings…, Databases…, Test Connection, Define Data Source…, Server Settings…
- `Open in Shell Editor` | item | `studio.code.open_in_shell` — the single seam between Studio code surfaces and the shell editor. The expression editor reuses the same code-editor module rather than the framing (SHL-P-18); a third code implementation is forbidden by [STU-SECTION-003]

**[STU-SHL-035] AUTOMATE (Studio module menu).**

- `Actions` | submenu → New Action…, New Action Set…, Record, Stop Recording, Play Action, Insert Stop…, Insert Menu Item…, Insert Path, Insert Conditional…, Action Options…, Playback Options…, Load Actions…, Save Actions…, Reset Actions, Clear All Actions
- `Macros` | submenu → Record Macro, Play Macro, Edit Macro…, Delete Macro, Macro Library…, Duplicate Macro Category
- `Batch` | submenu → Batch…, Create Droplet…, Image Processor…, Batch Rename…, Batch Export…, Batch Preflight…, Batch Convert Format…, Batch Progress…
- `Scripts` | submenu → Run Script…, Browse for Script…, Script Editor…, Script Library…, Script Properties…, Script Events…, Enable Attached Scripts, Reload Scripts, Script Console
- `Command API` | submenu → Command Browser…, Dry Run Command…, Command Console, Descriptor Inspector…, Copy Command Id, Copy Command JSON, Replay Command Batch…, Export Command Index… — menu-only
- `Data-Driven` | submenu → Data Merge…, Select Data Source…, Preview Data, Create Merged Document…, Define Variables…, Data Sets…, Import Data Set…, Export Data Set…, XML Import…, XML Export…, Convert Text to Field…
- `Plugins` | submenu → Manage Plugins…, Install Plugin from File…, Plugin Capabilities…, Plugin Console, Reload Plugins, Plugin Sandbox Report…, *(installed plugin commands, generated)*
- `Model Lane` | submenu → Propose Edit…, Review Proposals…, Approval Inbox, Validation Report…, Replay Proposal…, Rejected Proposals…, Actor Attribution…, Lease Status… — menu-only
- `Jobs` | submenu → Job Queue, Cancel Job, Job Log…, Render Workers…, Process Ledger…
- `Find & Change as Batch` | submenu → Run Query on Document, Run Query on Book, Run Query on Folder…, Run Query on Site…, Save Query…

**[STU-SHL-036] WORKSPACE (Studio module menu).**

- `Layout Presets` | submenu | `studio.workspace.layout_preset` → *(one leaf per shipped Layout Preset, [STU-SHL-092])*, Save Layout Preset…, Reset Layout, Manage Layout Presets…, Next Layout Preset
- `Panels` | expansion | `studio.workspace.panel` — one show/hide toggle per panel; leaf count 90, ASSERTED ([STU-SHL-038])
- `Tools` | expansion | `studio.workspace.tool` — see [STU-SHL-038]; leaf count 362 across 22 family submenus, ASSERTED
- `Panel Layout` | submenu → Save Layout Preset…, Manage Layout Presets…, Reset Layout, Import Layout Preset…, Export Layout Preset…, Collapse All Panels, Expand All Panels, Float Panel, Dock Panel, Hide All Panels
- `Toolbox` | submenu → Show Tool Rail, Customise Tool Rail…, Tool Presets…, Show Context Bar, Reset All Tools, Tool Cycle Groups…
- `Chrome` | submenu → Context Bar, Status Bar, Ruler Bar, Rails…, Application Frame
- `Presence` | submenu → Show Collaborators, Show Agent Activity, Follow Actor…, Conflict View, Presence Settings…
- `Arrange Documents` | submenu → New Editor Group for Document, Cascade, Tile, Consolidate All, Float All in Windows, Move Document to Group…, Next Document, Previous Document, Split Document View
- `Task Scopes` | submenu | `studio.workspace.task_scope` → *(one leaf per shipped Task Scope, [STU-SHL-153])*, Exit Task Scope

**[STU-SHL-037] HELP (extends the shell HELP menu).**

- `Studio Manual` | submenu → Open Studio Manual, Search Manual…, Help for This Command, Help for This Panel, Help for This Tool, Domain Guides…
- `Full Command Index…` | item — menu-only
- `Keyboard Shortcut Reference…` | item
- `Diagnostics` | submenu → Render Harness…, Visual Debugger, Flight Recorder, Validation Report…, System Report…, GPU Report…, Accessibility Tree Snapshot… — menu-only
- `About Studio` | item

#### 1.4 Expansion nodes, tool shape, and menu-only capability

**[STU-SHL-038] Expansion node register and the assertion boundary.** An `expansion` node's leaves are generated from a named registry rather than authored leaf by leaf. Exactly TWO expansion nodes are ASSERTED — their counts were rebuilt from per-application sources with an auditable merge — and every other expansion count is BUDGETED, an upper bound only.

| Node | Leaf source | Count | Status |
|---|---|---|---|
| `WORKSPACE > Tools` | tool registry, 22 families | 362 | **ASSERTED** |
| `WORKSPACE > Panels` | panel registry | 90 | **ASSERTED** |
| `EFFECTS > Apply Effect` | effect registry, grouped by family | upper bound | BUDGETED |
| `EFFECTS > Adjustments` | adjustment registry | upper bound | BUDGETED |
| `INSERT > Web Element` | web element registry | upper bound | BUDGETED |
| `OBJECT > Table` | table operation registry | upper bound | BUDGETED |
| `OBJECT > Align & Distribute` | alignment registry | upper bound | BUDGETED |
| `FILE > Import` / `FILE > Export` | format registries | upper bound | BUDGETED |
| `COLOR > Swatches & Colour Assets` | colour asset registry | upper bound | BUDGETED |
| `INSERT > Shape` | parametric shape registry | upper bound | BUDGETED |
| `MOTION > Markers` / `MOTION > Keyframes` | marker / keyframe command registries | upper bound | BUDGETED |
| `TYPE > OpenType` | OpenType feature registry | upper bound | BUDGETED |
| `EDIT > Preferences` | preference registry | upper bound | BUDGETED |

A BUDGETED count MUST NOT be written into a microtask acceptance criterion as a target. Each row of the register above is ONE unit of work, and for a BUDGETED row that unit's FIRST acceptance criterion is RECOVERING THE LIST: rebuilding that node's leaf set from the per-application sources with an auditable merge, exactly as `WORKSPACE > Tools` and `WORKSPACE > Panels` were rebuilt. Until that recovery lands, the node ships its expansion empty with a typed reason rather than shipping an invented list. See the declared spec debt in [STU-SHL-133].

**[STU-SHL-039] The settled shape of `WORKSPACE > Tools`.** Every tool in the tool registry appears in the menu, and NEVER as a flat list. The path is exactly:

```
WORKSPACE > Tools > <family> > <tool>
```

22 family submenus, 362 leaves, mean 16 leaves per family, maximum 44 in the largest family. The family level is MANDATORY. A flat 362-entry submenu is unusable and is forbidden by the same reasoning that makes the family level mandatory for the effect list ([STU-SHL-031]). Each leaf dispatches the identical command id that the Tool Rail slot, the tool search result and the command palette row dispatch — four projections, one command. The family register is [STU-SHL-141].

**[STU-SHL-040] Menu-only capability.** 375 nodes are menu-only: they have no persistent panel or inspector home because they operate on the application, on the document as a whole, or on a process, rather than on a selected object whose properties a panel could show. `menu_only = true` on the UiDescriptor is a typed declaration, not an omission, and satisfies the `palette_only` alternative branch of MENU-INV-1 only when the node also has no palette row. The structurally menu-only areas with zero panel rows are: preferences, colour settings and management policy, packaging and collect-for-output, keyboard and menu customisation, and whiteboard document operations. Handshake-native menu-only nodes are `AUTOMATE > Command API`, `AUTOMATE > Model Lane`, `HELP > Full Command Index`, `HELP > Diagnostics`, and `OBJECT > Object Label / Script Id`.

**[STU-SHL-041] Palette and index performance.** The command palette MUST render each row's `menu_path` joined as a breadcrumb, so the palette TEACHES the menu location rather than replacing it. Both projections read ONE availability evaluation and display the same reason code. The palette matcher MUST be the shared fuzzy matcher extracted from the existing symbol palette (subsequence matching with contiguity and word-boundary bonuses), not a fourth independent implementation, and a prefix/trigram index MUST be built once in the same `OnceLock` that holds the command set. A per-frame fresh allocation and substring scan is acceptable at the shell's present command count and is NOT acceptable at Studio's; the semantic match predicate stays as the definition and the index is its accelerator, so existing matcher tests remain valid.

#### 1.5 Shortcut policy

**[STU-SHL-042] Shortcut resolution rules (normative, ordered).** Applied in this priority order.

| Rank | Rule |
|---|---|
| 1 | **FROZEN SPINE.** Chords measured as semantically agreeing across the source captures are immovable in shipped sets. |
| 2 | **CONTEXT SCOPE BEFORE ARBITRATION.** Studio shortcut contexts, most-specific first: `expression_editor`, `graph_editor`, `code_surface`, `text_editing`, `timeline`, `develop`, `board`, `modal_workspace`, `panel_focus`, `canvas`, `global`. |
| 3 | **TOOL LETTERS ARE ONE GLOBAL NAMESPACE.** One letter, one tool; `Shift`+letter cycles the declared tool group forward, `Shift`+letter again or a declared backward chord cycles it back. |
| 4 | **DOMAIN WEIGHT BREAKS TIES**, using registry domain row counts. |
| 5 | **FREQUENCY OVERRIDE** only by a recorded decision with a Flight Recorder measurement attached, never silently. |
| 6 | **NO SILENT SHADOWING.** A chord already bound in the same context is a build-time validator FAILURE. |
| 7 | **ONE CHORD PER COMMAND** in a shipped set. |
| 8 | **PHYSICAL-LAYOUT INDEPENDENCE.** Bindings are stored against a logical key id, never a physical scancode. |
| 9 | **PORTABLE SETS.** One default plus named migration sets, each importable, exportable and diffable. |

**[STU-SHL-043] Shipped shortcut sets.** `Studio Default` is the only normative set. Named migration sets ship alongside it: `Raster-Familiar`, `Vector-Familiar`, `Timeline-Familiar`, `Composition-Familiar`. A migration set MUST NOT change any command's semantics, only its chord.

**[STU-SHL-044] The menu is the shortcut editor.** Because every menu leaf is a UiDescriptor carrying its chord and its context, `EDIT > Keyboard & Menus > Shortcut Conflict Report` MUST enumerate every collision in the active set at any time, per context, with the winning binding and the shadowed ones named. No source suite exposes this; Studio MUST, because the data already exists in the descriptor set.

**[STU-SHL-045] The default chord set is NOT frozen by this sub-section.** See the declared spec debt in [STU-SHL-131]. No shipped chord set may be frozen until the arbitration is recomputed over all five captured binding tables.

---

### 2. The availability predicate

**[STU-SHL-046] One predicate, evaluated once.** Studio MUST have exactly ONE availability predicate. It is evaluated ONCE per element per context change — never per frame, never per surface — and every surface renders the SAME result. Two elements with the same `requires` expression in the same document state MUST produce the same `availability_state` in the menu, in the Tool Rail, in a panel, in search and in the command API. Surface-local gating logic is forbidden.

**[STU-SHL-047] The predicate is over the document PROFILE, never a document type enum.** The input is the document profile: a set of declared CONTAINER KINDS drawn from `{artboard, page_spread, timeline, board, site_tree}` plus a set of FEATURE FLAGS. A document type enum MUST NOT be used as the gating input. The reason is structural and is the operator's own: a type enum re-creates the seven-application silo inside one binary and breaks [STU-DOC-004], the shared-primitives law. The measured shape of the problem is that the same capability serves multiple document shapes — crop is present in five of the captured applications, and gating it behind a mode means implementing it once and hiding it four times. The seven named document shapes survive ONLY as (a) creation presets that seed a profile and (b) the layout persistence key of [STU-SHL-054].

**[STU-SHL-048] Profile lifecycle.** A profile is declared at document creation from a document preset. It GROWS when a container or a layer of a new kind is added. It MUST NOT shrink silently. Growth is a normal, expected event and is the mechanism behind promotion-as-remedy ([STU-SHL-020]).

**[STU-SHL-049] Clause kinds (normative, closed).** A `requires` expression is a conjunction over clauses drawn from exactly these twelve kinds and no others:

`container:<kind>`, `layer_kind:<kind>`, `doc_feature:<flag>`, `selection:>=N`, `selection_kind:<kind>`, `selection_count:<n>`, `tool:<id>`, `capability:<name>`, `capability_flag:<gpu|ml_model_present|raw_decoder|video_decoder>`, `mode:<edit|read_only>`, `color_mode:<rgb|cmyk|grayscale|lab|indexed|hdr>`, `task_scope:<id|null>`.

`has_timeline` and `page_count` are NOT clause kinds: `has_timeline` IS `container:timeline`, and page count is a document query, not a gate.

**[STU-SHL-050] The three states (normative, closed).**

| State | Definition | Carries |
|---|---|---|
| `AVAILABLE` | predicate true | — |
| `INAPPLICABLE_HERE` | predicate false AND the host can name ONE action that would make it true | `reason_code`, `reason_text`, `remedy_command_id` |
| `NOT_IN_THIS_DOCUMENT` | the element has no meaning for this profile at all and no single action reaches it | `reason_code`, `reason_text`, and the profile clause that owns it |

Two-valued availability is forbidden. Two values force a choice between clutter and mystery; the middle state is where discoverability lives, and it is precisely the state a persona toggle destroys.

**[STU-SHL-051] Reason codes (normative, closed).** Exactly twelve members:

`NO_SUCH_CONTAINER`, `WRONG_LAYER_KIND`, `DOC_FEATURE_ABSENT`, `NEEDS_SELECTION`, `WRONG_SELECTION_KIND`, `WRONG_COLOR_MODE`, `CAPABILITY_NOT_GRANTED`, `CAPABILITY_FLAG_ABSENT`, `LEASE_HELD_BY_OTHER_ACTOR`, `READ_ONLY_DOCUMENT`, `EXPRESSION_DRIVEN`, `NOT_IMPLEMENTED_YET`.

`EXPRESSION_DRIVEN` is the code a value field returns when its value is computed rather than stored; it is the same predicate applied to a value field ([STU-SHL-203]). `LEASE_HELD_BY_OTHER_ACTOR` binds to the parallel-work lease model of [STU-PAR-003].

**[STU-SHL-052] Per-surface rendering (normative).** The three states render differently per surface. The difference follows from what each surface IS; it is not a compromise between positions.

| Surface | `AVAILABLE` | `INAPPLICABLE_HERE` | `NOT_IN_THIS_DOCUMENT` |
|---|---|---|---|
| Menu (every level, titles included) | enabled | greyed, reason shown, remedy rendered as an enabled sibling leaf | greyed, reason shown. **NEVER hidden** ([STU-SHL-018]) |
| Tool Rail | enabled | dimmed, remedy inline | **absent from the rail**, but still present in the AccessKit tree and reachable from menu, search and family browse |
| Search and command palette | listed | listed, sorted lower, reason displayed | listed, sorted lowest, reason displayed. **Search NEVER filters on availability** ([STU-SHL-053]) |
| Panels | docked or rail-registered | docked, rendering an explanatory EMPTY STATE (`hide_when_unbound` is false) | not in the dock, but the panel still emits `studio.rail.<region>.<panel>` so an agent can always discover and open it |
| Context Bar | enters the slot resolver | does not enter the resolver | does not enter the resolver |

**[STU-SHL-053] Search never filters on availability.** Search and the command palette MUST return every matching element regardless of state, sorted by state, each row displaying its reason. Filtering unavailable results out looks like a quality improvement and is strictly worse than a persona toggle: the operator types a correct tool name, gets nothing, and concludes Studio does not have the tool — whereas a persona at least names the mode to switch to. Enforced by an inspector test that searches for a `NOT_IN_THIS_DOCUMENT` element and requires a result row carrying a reason.

**[STU-SHL-054] Profile signature and the named signatures.** The layout persistence key is the `profile signature`: the sorted set of the document's declared CONTAINER kinds. Feature flags are NOT part of the key. Containers are what a layout is about — a timeline needs a bottom dock, a page spread needs a pages panel, a site tree needs a file tree — and they change rarely and meaningfully, whereas feature flags change the moment a layer of a new kind is added and would swap the operator's arrangement mid-edit.

| Named shape | Signature |
|---|---|
| raster | `{artboard}` |
| vector | `{artboard}` |
| photo | `{artboard}` |
| layout | `{page_spread}` |
| composition | `{artboard, timeline}` |
| sequence | `{timeline}` |
| web | `{artboard, site_tree}` |

Raster, vector and photo share `{artboard}`, so the container set alone is insufficient to separate them. The tie-break is the dominant feature flag, stated explicitly rather than left to implementation: `pixel_canvas + channels` → raster; `vector_network` → vector; `raw_develop + catalog_link` → photo. A document carrying two of the three resolves to whichever preset created it, recorded on the document.

**[STU-SHL-055] Layout Presets may not gate.** No Layout Preset, preference, Task Scope entry or workspace-like mechanism may change any element's `availability_state`. Enforceable test, asserted through the inspector: for any Layout Preset `W` and any element `E`, `availability_state(E)` MUST be identical with `W` active and with `W` inactive. A Task Scope is the single exception and is not an exception to the predicate: it is an explicit clause kind (`task_scope:<id>`) that the predicate reads, so its effect is inside the one predicate and is visible in the reason code ([STU-SHL-152]).

---

### 3. The slot resolver

**[STU-SHL-056] The slot resolver is a SECOND stage, not a competitor.** Availability and slot resolution are two distinct stages with distinct names. Stage 1 is the availability predicate, per element, three-valued. Stage 2 is the slot resolver, per slot, a ranking. The resolver runs ONLY over candidates whose `availability_state` is `AVAILABLE`. It never re-decides availability and it never makes an unavailable element appear.

**[STU-SHL-057] Single-occupant slots (normative, closed).** Exactly these slots are resolved:

| slot id | Region | Occupancy |
|---|---|---|
| `inspector` | Inspect Dock, first tab of the first group | one panel |
| `context-bar` | Context Bar, Selection Zone | one bar layout |
| `properties` | wherever a properties panel is docked | one binding |

**[STU-SHL-058] Binding declaration record.** Every panel or bar layout that can occupy a slot declares one or more bindings. The record generalises the field's own mechanism — one captured web-authoring application ships 60 property inspectors into ONE slot, every one of them declaring a tag, an attribute, a priority and a selection scope, and implementing a runtime `canInspectSelection` veto; a page-layout application ships 34 resource variants of ONE control panel chosen by what is selected. Studio's record is:

| Field | Type | Semantics |
|---|---|---|
| `slot` | slot id | which slot this binding competes for |
| `binds_to` | selector | what it binds to: `layer_kind:<k>`, `selection_kind:<k>`, `primitive:<name>`, `tool:<id>`, `container:<kind>`, or `any` |
| `selection_scope` | enum `{single, multiple, homogeneous_multiple, none}` | the selection shape the binding handles |
| `priority` | integer, higher wins | declared rank among competing bindings |
| `can_bind` | predicate | a runtime ABSOLUTE VETO, evaluated before priority |
| `sticky_key` | derived | `(profile signature, resolved primitive)`, used by [STU-SHL-061] |

**[STU-SHL-059] Resolution algorithm (normative, ordered).**

1. Collect every binding declared for the slot.
2. Drop every binding whose owning element is not `AVAILABLE`.
3. Drop every binding whose `selection_scope` does not match the current selection shape.
4. Drop every binding whose `binds_to` selector does not match the current selection's resolved primitive. With no selection, only `binds_to: tool:<current>` and `binds_to: any` survive.
5. Apply `can_bind`. This is an ABSOLUTE veto and is applied BEFORE priority; a vetoed binding cannot win on rank.
6. Rank the survivors by `priority`, descending.
7. On an exact tie, the binding with the more specific `binds_to` selector wins; `layer_kind` and `selection_kind` are more specific than `primitive`, which is more specific than `container`, which is more specific than `any`.
8. If the winner differs from the currently resolved binding AND the previous binding's `sticky_key` still matches, keep the previous binding ([STU-SHL-061]).
9. Emit `studio.slot.<slot>.resolved`.
10. If nothing survives, the slot renders its declared empty state. A slot MUST NOT render stale content from a previous selection.

**[STU-SHL-060] Auto-resolved sibling cap.** At most FIVE bindings may be auto-presented as sibling tabs in a resolved slot; the remainder are reachable from the slot's overflow. Five is a DECLARED JUDGEMENT, not a measured value: the platform tolerates deeper tab groups (captured groups of 11 and 13 exist) but auto-generated depth at that scale is unreadable. Recorded as an open operator decision in [STU-SHL-136].

**[STU-SHL-061] Anti-thrash.** Resolution runs on SELECTION CHANGE only, never per frame. Ranks 2 through 8 of [STU-SHL-059] are recomputed only when rank 1's candidate set changes. The active tab is sticky, keyed by `(profile signature, resolved primitive)`. Required validation case: select a node, deselect it, reselect the same node, and assert the emitted AccessKit tree is BYTE-IDENTICAL across the three states. Inspector thrash is both an operator defect and a test-flakiness defect.

**[STU-SHL-062] Resolver observability.** `studio.slot.<slot>.resolved` MUST expose the `author_id` the resolver chose, the matched selector, and the winning priority. Without it an agent must infer the resolver's decision from what happens to be visible, which is not an assertion.

---

### 4. The dock and panel model

#### 4.1 Structure

**[STU-SHL-063] Region structure.** Five regions plus two rails plus the centre, per [STU-SHL-003]. Each EDGE dock is a stack of GROUPS; each group is a TAB STACK; each tab is one panel instance. The two rails are single-occupant strips, not tab stacks. Studio owns the edge, rail and slot semantics and uses `egui_tiles` for the tiling INSIDE each region. There MUST be one `Tree` PER REGION, never one global tree, so a stray drag cannot dissolve the edge structure. A single free-form tile tree is rejected: it has no concept of edge affinity, icon-rail collapse or a single-occupant contextual slot, and four of the six captured applications with workspace files serialise edge-anchored docks.

**[STU-SHL-064] Region register (normative).**

| region_id | Default size | Default groups | Purpose | Collapse behaviour |
|---|---|---|---|---|
| `top` Context Bar | 34px high | — | two zones, single-occupant each | hideable to 0px by one toggle; never becomes an icon rail because there is nothing to enumerate |
| `left-rail` Tool Rail | 44px wide | — | the tool palette, ~20 group slots | single-column / double-column toggle; cannot be emptied; may be moved to the right rail |
| `left` Browse Dock | 260px wide | 1 | what exists and what can be brought in — SOURCES, not properties | collapses to a 32px ICON RAIL listing every registered panel of the dock; click opens as a temporary overlay, shift-click pins the edge open |
| `right` Inspect Dock | 300px wide, 260px hard minimum | 3 | what is selected and how it is configured; the `inspector` slot is pinned to the FIRST tab of the FIRST group | per-group collapse to a title strip, plus whole-dock collapse to a 32px icon rail |
| `right-rail` Meter Rail | 36px wide, hidden by default | — | a continuously-updating readout that must never be hidden behind a tab | show/hide only |
| `bottom` Time & Results Dock | 240px high (300px minimum when a two-column time panel is present) | 1 | TIME and RESULTS: things that are wide and about the whole document rather than the selection | collapses to a 28px icon rail |
| `centre` Viewport | remainder | unbounded | the documents themselves | groups collapse when their last tab closes |

The Inspect Dock's default of three groups maps onto three refresh rhythms: group 1 changes with the SELECTION, group 2 with the OPERATOR's colour choices, group 3 with the DOCUMENT's structure. Panels with different refresh rhythms in one stack are what makes a dock feel jumpy.

**[STU-SHL-065] The centre is an unbounded tree of editor groups.** The Viewport is an `egui_tiles` tree holding an ARBITRARY number of editor groups, each with its own tab stack. The number of groups MUST NOT be fixed — not two, not four, not a 2×2 splitter, not any hard-coded shape. Drag a document tab onto another group's strip to move it; onto a group's body edge to split; onto empty space to create a group; close the last tab and the group collapses with its space redistributed. A hard-coded splitter with named split-weight fields is a DEFECT to be replaced, not a shape to be preserved ([STU-SHL-111]). No group cap may be imposed.

**[STU-SHL-066] Viewer kinds (normative, closed).** A centre tab holds one viewer of exactly one of these kinds: `composition`, `sequence-program`, `source`, `footage`, `layer`, `canvas`, `page`, `code`, `catalog-grid`, `board`. A `layer` viewer is where paint, roto and mask work happens on one layer in isolation.

**[STU-SHL-067] Panels never dock to the centre; documents never dock to an edge.** A panel dragged over the centre becomes FLOATING. A document tab dragged onto an edge is REFUSED with a visible no-drop cue. This keeps the viewport clean and keeps the two tree families from having to share a tile type.

**[STU-SHL-068] Panel states (normative, closed).** Exactly seven: `docked_tab`, `docked_active`, `collapsed_to_rail`, `overlay`, `floating`, `popped_out`, `hidden`. `popped_out` MUST reuse the shell's existing pop-out window implementation with its merge-back and geometry clamping, unchanged. `collapsed_to_rail` is a hidden-but-REGISTERED state and is distinct from not-installed; a `collapsed_to_rail` panel still emits its AccessKit node.

**[STU-SHL-069] Dock constraints (normative).**

1. Minimum dock size is clamped by the same discipline the shell splitter already uses: a minimum fraction of 0.2, a maximum of 0.8, and a step of 0.05. No dock may be dragged to zero and no panel may become unreachable.
2. An edge with zero panels renders as a 6px SEAM, not as nothing, so a panel can always be dragged back to it.
3. Group count per EDGE is capped at 4. The cap applies to EDGES ONLY; the centre tree is explicitly uncapped ([STU-SHL-065]).
4. The dock model MUST render correctly with all four edges absent, because a Studio viewport is embedded inside another module's document view with no edge docks.
5. Every dock gesture, panel-open, panel-move and panel-close MUST be a registered command reachable from the menu and the palette as well as by pointer. A Studio surface that owns an action with no command is a defect.

**[STU-SHL-070] Panel movement is one transaction against one registry.** A `PanelMoveRequest` MUST be applied as a SINGLE transaction against the `StudioPanelRegistry`, which owns the truth about where every panel is; the region trees are then REBUILT from the registry. Two trees MUST NOT be mutated directly in sequence. In debug builds the registry MUST assert every frame that every registered panel appears in exactly one tree. Without this, a cross-tree drag can remove a tile from the source and never insert it into the target, losing the panel.

**[STU-SHL-071] Panel registry shape.** `StudioPanelRegistry` mirrors the shell pane registry's conventions — reference-counted string ids, a deterministic-iteration ordered map, stable kebab-case `author_id`s — but MUST NOT reuse the shell's pane record type, because a Studio panel additionally carries a dock, a group, a tab index, a binding declaration and an optional instance parameter that the shell record does not model.

#### 4.2 Density

**[STU-SHL-072] Panel density model (normative).** Derived from measured vendor declarations, not estimated.

| Quantity | Value | Basis |
|---|---|---|
| dense parameter panel floor | 240px wide | the field-tested floor: 49 captured surfaces declare a preferred docked size with median and mode both 240 |
| ScrubValue field body minimum | 72 × 24px | 24px is the practical minimum pointer target for a press-and-drag grab; a 20px row is fine for click-to-type and too thin to grab reliably |
| scrub allowance | +32px per two-field row | vendor field body ≈ 56px; the grab minimum adds ≈16px per field |
| Inspect Dock default | 300px | 240 floor + 32 scrub allowance = 272, rounded for a scrollbar gutter and the 10px scroll lane |
| Inspect Dock hard minimum | 260px | below this the panel switches to the compact form |
| row pitch | 24 + 4 = 28px | — |
| row budget at 300px | 266px usable: one labelled scalar (label 96 + gap 8 + field 72) or one label plus two linked scalars (24 + 72 + 8 + 72) | THREE fields on one row does not fit at 300px and MUST NOT be attempted |

**[STU-SHL-073] Below the minimum, and above the maximum.** Below 260px a panel switches to a STACKED COMPACT form: label above field, one field per row, 20px pitch. It MUST NOT clip and MUST NOT hide controls; a panel that would clip shows a horizontal scroll rather than a truncated field. A panel declaring a preferred width above 320px is registered in its edge but PRESENTS as an overlay or floating panel by default; the vendors do not force their own wide surfaces into their own 240px docks and Studio must not either.

**[STU-SHL-074] Mixing rule.** Do NOT stack a FIELD-DENSE panel (transform, character, paragraph, effect-controls, adjustments, stroke) with a LIST-DENSE panel (layers, swatches, library, project-bin) in a group under 300px tall. Field-dense panels want a fixed modest height; list-dense panels want all the height they can get. The two-column time panel of [STU-SHL-086] is EXCLUDED from this rule and carries its own sizing contract.

#### 4.3 The panel inventory and the shipping spine

**[STU-SHL-075] Panel count (normative).** Studio has **90 distinct panel identities** plus **9 reused shell panes**. The reused shell panes are Problems, Jobs, SourceControl, UserManual, FontManager, FindInFiles, FlightRecorder, VisualDebugger and RuntimeChat; Studio MUST reuse them and MUST NOT re-implement any of them. The derivation of 90 is auditable: 75 from the vendor-surface deduplication, +9 from the operator's rejection of the preset-library merge, +6 from the compositing and video fold-in. The figures 814 and 75 and 84 are superseded and MUST NOT be cited.

**[STU-SHL-076] 90 is a count of IDENTITIES, not an estimate of effort.** One panel identity may subsume thousands of parameter rows: the effect-controls panel alone subsumes nine captured effect-editing surfaces, one application's 617 effects with 9,654 parameter rows, and another's 635 effects with 1,573 typed parameter records. Any consuming work packet MUST size from the capability corpus mapped THROUGH these panels, never from the panel count.

**[STU-SHL-077] Panel classes (normative, closed).** Every panel declares exactly ONE class, drawn from exactly these four:

| Class | Definition | Panels |
|---|---|---|
| `always_relevant` | answers a question the operator has at essentially every moment in every profile; docked and visible by default, never auto-hidden | 21 |
| `document_type_specific` | always relevant, but only inside certain profiles; present in the default layout for those profiles and ABSENT for the others, though never absent from the accessibility tree ([STU-SHL-052]) | 26 |
| `task_episodic` | opened for a task and closed after it; registered in an icon rail and docked in no default layout | 25 |
| `selection_contextual` | occupies a single-occupant slot and is chosen by the slot resolver against the current selection rather than docked by the operator ([STU-SHL-056]) | 18 |

The four counts sum to 90 and are enumerated per panel in [STU-SHL-113].

**[STU-SHL-078] The eight-panel spine ships FIRST.** Exactly eight panels form the cross-profile invariant spine and are present in EVERY default layout:

`context-bar`, `tools`, `properties`, `layers`, `colour`, `swatches`, `history`, `info`.

These eight MUST be implemented and proven through the accessibility inspector BEFORE any further panel is authored. The remaining 82 panels follow behind the spine, behind a STABLE registry API, and are explicitly parallelisable content work fed by captured parameter metadata. The risk in this design is concentrated in the centre tree, the scrub widget and the slot resolver — not in the panels — so the spine exists to prove those three against a deliberately small panel set. Everything else varies by profile signature, which is what makes a default layout meaningful rather than cosmetic.

**[STU-SHL-079] Panels added by the compositing and video fold-in.** Six panels join the inventory and each is a first-class build target, not a stub: `keyframe-timeline` (bottom, `document_type_specific`), `expression-editor` (right or floating, `task_episodic`), `keyframe-assistant` (right, `task_episodic`), `tracker` (right, `document_type_specific`), `render-queue` (bottom, `task_episodic`), `comp-flowchart` (centre or floating, `task_episodic`).

**[STU-SHL-080] Merge law.** A consolidation that only works given an unbuilt capability is not a consolidation, it is deferred debt. A merge is permitted ONLY where the merged panel is usable AS-IS. The operational test: does the merge require two of its members to be VISIBLE AT THE SAME TIME? If yes, the merge is REJECTED. Consequences that MUST hold:

| Merge | Verdict | Reason |
|---|---|---|
| twelve preset surfaces → one preset library | **REJECTED** | requires simultaneous visibility; the panels stay separate |
| the clip timeline and the keyframe timeline → one panel with a mode switch | **REJECTED** | see [STU-SHL-086] |
| five titling surfaces → one panel | **REJECTED** and redecomposed: properties → the inspector slot by binding; styles → `text-styles`; tools → the Tool Rail; actions → `align-distribute`; the on-canvas editor → a Viewport editing mode; the template browser → `graphics-templates` | a category error: three of the five are not panels at all |
| the graph editor → a mode of the keyframe timeline | **PERMITTED** | the graph replaces the keyframe strip for the SAME properties at the same time; the two views are alternatives, never simultaneous |
| 34 control-panel variants → the `context-bar` slot | **PERMITTED** | 34 alternative layouts of ONE slot chosen by selection; only one selection exists at a time |
| 60 property inspectors → one `properties` panel plus 60 bindings | **PERMITTED** | this is the slot resolver, not a merge |
| motion-sketch + smoother + wiggler → `keyframe-assistant` | **PERMITTED** | three operations on the keyframes of one selected property, never simultaneous |
| several mutually exclusive print previews → `print-preview` | **PERMITTED** | the operator is either separating, flattening, ink-managing or soft-proofing |
| several scope kinds → one `scopes` panel | **PERMITTED WITH A REQUIREMENT** | the panel MUST ship its internal N-up grid in the SAME microtask as the panel, or the merge silently becomes deferred debt: a colourist routinely needs waveform and vectorscope at once |
| source viewer and program viewer | **NOT A MERGE** | two viewers are two editor groups in the unbounded centre tree; simultaneous visibility is already available with zero new capability |

**[STU-SHL-113] The panel catalogue (normative, closed).** The 90 panel identities of [STU-SHL-075] are enumerated below. EACH ROW IS ONE UNIT OF WORK. A panel absent from this table does not exist in Studio, and a panel present here may not be dropped, silently merged or deferred without an operator decision recorded under [STU-SHL-136]. `Class` is drawn from the closed set of [STU-SHL-077]. `Default region` names the region of [STU-SHL-064] the panel occupies in the shipped default layouts; an operator may move any panel to any edge ([STU-SHL-069]) and L2 makes the move permanent ([STU-SHL-081]). `Profiles` names the profile signatures whose default layout carries the panel; a panel not listed for a profile is still reachable there through its icon rail and still emits its accessibility node ([STU-SHL-052]). `Vendor surfaces merged` is CAPTURE PROVENANCE ONLY per [STU-SECTION-003] and never a Studio name; the count in parentheses is how many surfaces of that application collapsed into this one panel identity, which is what makes the merge auditable and reversible from this table alone. Panel COUNT is not panel EFFORT ([STU-SHL-076]).

| Panel | panel_id | Class | Default region | Profiles | Vendor surfaces merged (provenance) |
|---|---|---|---|---|---|
| Actions & Macros | `actions-macros` | `task_episodic` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (1); affinity (1); dreamweaver (1) |
| Adjustments | `adjustments` | `document_type_specific` | Inspect Dock | raster, photo, sequence | photoshop (1); affinity (3); lightroom_classic (1) |
| Align & Distribute | `align-distribute` | `selection_contextual` | Inspect Dock | raster, vector, layout, web, composition | illustrator (1); indesign (1); affinity (1) |
| Anchoring & Pinning | `anchoring` | `selection_contextual` | Inspect Dock | layout, web | affinity (2); indesign (1) |
| Appearance | `appearance` | `selection_contextual` | Inspect Dock | vector, layout, web, raster | illustrator (2); affinity (1); indesign (2) |
| Audio Meters | `audio-meters` | `document_type_specific` | Meter Rail | sequence, composition | premiere (1) |
| Audio Mixer | `audio-mixer` | `document_type_specific` | Time & Results Dock | sequence, composition | premiere (3) |
| Book | `book` | `task_episodic` | Browse Dock | layout | indesign (1); affinity (1) |
| Brushes | `brushes` | `document_type_specific` | Browse Dock | raster, photo, vector | photoshop (2); illustrator (2); affinity (1) |
| Captions | `captions` | `task_episodic` | Browse Dock | sequence | premiere (2) |
| Catalog | `catalog` | `document_type_specific` | Browse Dock | photo | lightroom_classic (1) |
| Channels | `channels` | `document_type_specific` | Inspect Dock | raster, photo | photoshop (1); affinity (1) |
| Character | `character` | `selection_contextual` | Inspect Dock | layout, vector, raster, web, composition, sequence | photoshop (1); illustrator (1); indesign (1); affinity (1) |
| Clone Source | `clone-source` | `task_episodic` | Inspect Dock | raster, photo | photoshop (1); affinity (1) |
| Colour | `colour` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (2); indesign (3); premiere (1); affinity (1) |
| Colour Libraries | `colour-libraries` | `task_episodic` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | indesign Swatch Library Panel |
| Composition Flowchart | `comp-flowchart` | `task_episodic` | Viewport | composition | aftereffects Composition Flowchart and Mini-Flowchart |
| Context Bar | `context-bar` | `selection_contextual` | Context Bar | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (1); indesign (1); affinity (1); dreamweaver (2) |
| CSS Designer | `css-designer` | `document_type_specific` | Inspect Dock | web | dreamweaver (1); illustrator (1); figma (1) |
| Data Merge | `data-merge` | `task_episodic` | Browse Dock | layout, web | illustrator (1); indesign (1); affinity (1) |
| Data Sources | `data-sources` | `document_type_specific` | Browse Dock | web | dreamweaver (4) |
| Develop Presets | `develop-presets` | `document_type_specific` | Browse Dock | photo, raster | lightroom_classic develop presets |
| Document Templates | `document-templates` | `task_episodic` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | lightroom_classic templates; affinity Assets book member |
| Effect Controls | `effect-controls` | `selection_contextual` | Inspect Dock | raster, vector, layout, composition, sequence, photo | premiere (2); indesign (9); photoshop (1); affinity (2); aftereffects (1) |
| Effects Browser | `effects-browser` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, photo | premiere (1); aftereffects (1); affinity (1); illustrator (1) |
| Export | `export` | `task_episodic` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | illustrator (3); affinity (1); indesign (5); lightroom_classic (1) |
| Expression Editor | `expression-editor` | `task_episodic` | Inspect Dock | composition, sequence | aftereffects Expressions and Scripting Palette panel plugin, 0/16 docked |
| Filter & Sort | `filter-sort` | `document_type_specific` | Context Bar | photo, sequence | lightroom_classic (1); premiere (1) |
| Find & Replace | `find-replace` | `task_episodic` | Time & Results Dock | raster, vector, layout, composition, sequence, web, photo | indesign (2); affinity (1); premiere (1); dreamweaver (1) |
| Generate | `generate` | `task_episodic` | Inspect Dock | raster, vector, layout, composition, sequence, photo | photoshop (3); illustrator (4); affinity (1) |
| Glyphs | `glyphs` | `task_episodic` | Inspect Dock | layout, vector, raster, web | photoshop (1); illustrator (1); indesign (1); affinity (2) |
| Gradient | `gradient` | `selection_contextual` | Inspect Dock | vector, layout, raster, web | illustrator (1); indesign (1); photoshop (1) |
| Gradient Presets | `gradient-presets` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | photoshop gradients; illustrator gradient library |
| Graphic Styles | `graphic-styles` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web | photoshop styles; illustrator NamedStyle |
| Graphics | `graphics-templates` | `document_type_specific` | Inspect Dock | sequence, composition | premiere (6) |
| Guides & Grid | `guides-grid` | `task_episodic` | Inspect Dock | raster, vector, layout, web, composition | indesign (3); affinity (1) |
| History | `history` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (1); indesign (1); premiere (1); affinity (1) |
| Hyperlinks | `hyperlinks` | `task_episodic` | Inspect Dock | layout, web | indesign (1); affinity (1); dreamweaver (1) |
| Info | `info` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (2); illustrator (2); indesign (2); premiere (2); affinity (1) |
| Insert | `insert` | `document_type_specific` | Browse Dock | web, layout | dreamweaver (1); indesign (1) |
| Interactions | `interactions` | `document_type_specific` | Inspect Dock | web, layout, composition | dreamweaver (1); indesign (3); figma (1) |
| Keyframe Assistant | `keyframe-assistant` | `task_episodic` | Inspect Dock | composition, sequence | aftereffects AEGP_MotionSketch + AEGP_Smoother + AEGP_Wiggler |
| Keyframe Timeline | `keyframe-timeline` | `document_type_specific` | Time & Results Dock | composition, sequence | aftereffects AE Timeline, 16/16 factory workspaces |
| Keywords | `keywords` | `document_type_specific` | Inspect Dock | photo | lightroom_classic (1) |
| Layers | `layers` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (1); indesign (1); premiere (1); dreamweaver (1); affinity (1); figma (1) |
| Library | `library` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (2); illustrator (2); indesign (1); premiere (1); dreamweaver (2); affinity (4) |
| Links & Placements | `links-placements` | `always_relevant` | Browse Dock | layout, vector, raster, web, sequence | illustrator (1); indesign (1); affinity (3); premiere (1) |
| Long Document | `long-document` | `task_episodic` | Browse Dock | layout | indesign (4); affinity (4) |
| Markers | `markers` | `document_type_specific` | Browse Dock | sequence, composition, layout, photo | premiere (3); affinity (1) |
| Masks | `masks` | `selection_contextual` | Inspect Dock | raster, photo, sequence, composition | affinity (1); photoshop (1) |
| Media Browser | `media-browser` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | premiere (1); dreamweaver (1); photoshop (1) |
| Metadata | `metadata` | `selection_contextual` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | illustrator (2); premiere (1); affinity (1); lightroom_classic (1) |
| Mockup | `mockup` | `task_episodic` | Inspect Dock | vector, raster | illustrator (1) |
| Navigator | `navigator` | `always_relevant` | Inspect Dock | raster, vector, layout, photo, web | photoshop (1); illustrator (1); affinity (1) |
| Notes & Comments | `notes-comments` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (2); illustrator (1); premiere (1); affinity (1) |
| Object Styles | `object-styles` | `always_relevant` | Inspect Dock | vector, layout, web, raster | indesign (2); illustrator (1); photoshop (1); affinity (1) |
| OpenType | `opentype` | `selection_contextual` | Inspect Dock | layout, vector, raster, web | illustrator (1); affinity (1); indesign (1) |
| Pages & Artboards | `pages-artboards` | `document_type_specific` | Browse Dock | layout, vector, web, composition | illustrator (1); indesign (1); affinity (1); figma (1) |
| Paragraph | `paragraph` | `selection_contextual` | Inspect Dock | layout, vector, raster, web | photoshop (1); illustrator (1); indesign (4); affinity (1) |
| Pathfinder | `pathfinder` | `selection_contextual` | Inspect Dock | vector, layout, web | illustrator (2); indesign (1); affinity (1) |
| Paths | `paths` | `document_type_specific` | Inspect Dock | raster, photo | photoshop (1) |
| Pattern Presets | `pattern-presets` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | photoshop patterns; illustrator PatternOptionsPanel |
| Preflight | `preflight` | `task_episodic` | Time & Results Dock | layout, web, vector, sequence | indesign (1); affinity (2); dreamweaver (3) |
| Preset Library | `preset-library` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (5); illustrator (3); indesign (2); affinity (1); lightroom_classic (1) |
| Print Preview | `print-preview` | `task_episodic` | Inspect Dock | layout, vector, raster, photo | illustrator (2); indesign (3); affinity (1) |
| Project Bin | `project-bin` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | premiere (3); dreamweaver (1); indesign (1); affinity (1) |
| Properties | `properties` | `selection_contextual` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (1); indesign (1); premiere (1); dreamweaver (1); affinity (1); figma (1) |
| Reading Order & Tags | `reading-order-tags` | `document_type_specific` | Browse Dock | layout, web | indesign (1); affinity (1); dreamweaver (1) |
| Render Queue | `render-queue` | `task_episodic` | Time & Results Dock | composition, sequence, raster, vector, layout, web, photo | aftereffects WorkQueue Palette; premiere export presets |
| Scopes | `scopes` | `document_type_specific` | Inspect Dock | raster, photo, sequence, composition | photoshop (1); premiere (1); affinity (2); lightroom_classic (1) |
| Shape Presets | `shape-presets` | `always_relevant` | Browse Dock | raster, vector, layout, web | photoshop customshapes |
| Site & Deploy | `site-deploy` | `document_type_specific` | Browse Dock | web | dreamweaver (5) |
| Snapping | `snapping` | `task_episodic` | Context Bar | raster, vector, layout, web, composition | affinity (1); illustrator (1) |
| Source Viewer | `source-viewer` | `document_type_specific` | Viewport | sequence, composition, photo | premiere (4); lightroom_classic (1) |
| Spelling | `spelling` | `task_episodic` | Inspect Dock | layout, web, vector | indesign (2); affinity (1) |
| States & Comps | `states-comps` | `task_episodic` | Inspect Dock | raster, vector, layout, photo | photoshop (1); affinity (3); lightroom_classic (1); figma (1) |
| Stroke | `stroke` | `selection_contextual` | Inspect Dock | vector, layout, raster, web | illustrator (1); indesign (1); affinity (1) |
| Swatches | `swatches` | `always_relevant` | Inspect Dock | raster, vector, layout, composition, sequence, web, photo | photoshop (1); illustrator (1); indesign (3); affinity (1) |
| Symbols | `symbols` | `document_type_specific` | Browse Dock | vector, layout, web | illustrator Symbols |
| Tabs & Indents | `tabs-indents` | `selection_contextual` | Context Bar | layout, vector, web | illustrator (2); indesign (1); affinity (1) |
| Text Frame | `text-frame` | `selection_contextual` | Inspect Dock | layout, vector | indesign (2); affinity (2) |
| Text Styles | `text-styles` | `always_relevant` | Inspect Dock | layout, vector, raster, web, composition, sequence | photoshop (2); illustrator (2); indesign (3); affinity (1) |
| Text Wrap | `text-wrap` | `selection_contextual` | Inspect Dock | layout, vector, web | indesign (1); illustrator (1) |
| 3D & Materials | `three-d-materials` | `document_type_specific` | Inspect Dock | vector, raster, composition | photoshop (2); illustrator (2) |
| Timeline | `timeline` | `document_type_specific` | Time & Results Dock | composition, sequence, raster | photoshop (1); premiere (1); illustrator (1); aftereffects (1) |
| Tool Presets | `tool-presets` | `always_relevant` | Browse Dock | raster, vector, layout, composition, sequence, web, photo | photoshop toolpresets |
| Tools | `tools` | `always_relevant` | Tool Rail | raster, vector, layout, composition, sequence, web, photo | photoshop (2); illustrator (3); premiere (1); affinity (3) |
| Tracker | `tracker` | `document_type_specific` | Inspect Dock | composition, sequence | aftereffects AEGP_TrackerPalette; premiere mask tracking |
| Transform | `transform` | `selection_contextual` | Inspect Dock | raster, vector, layout, composition, sequence, web | illustrator (1); indesign (1); affinity (1); premiere (1) |
| Vectorise | `vectorise` | `task_episodic` | Inspect Dock | vector, raster | illustrator (2); affinity (1) |

The eight spine panels of [STU-SHL-078] - `context-bar`, `tools`, `properties`, `layers`, `colour`, `swatches`, `history`, `info` - are rows of this table and ship FIRST; the remaining 82 rows follow behind them behind a stable registry API.

---

#### 4.4 Persistence

**[STU-SHL-081] The persistence cascade.** `effective_layout = L3 ?? (selected L1 preset) ?? L2 ?? L0`, keyed by PROFILE SIGNATURE ([STU-SHL-054]).

| Layer | Name | Writable | Scope | Content |
|---|---|---|---|---|
| L0 | `shipped_default` | no | per signature | the default layouts of [STU-SHL-083] onward; "Reset layout" always returns here |
| L1 | `layout_preset` | yes | per signature | operator-saved named arrangements, selected from a dropdown in the Studio tab strip listing only presets for the active signature |
| L2 | `user_default` | yes | per signature | the operator's live arrangement, written automatically and debounced whenever anything moves. This is the layer that makes rearrangement stick. |
| L3 | `tab_override` | yes | one open document tab | a diff against L2, created only when a per-tab override is explicitly engaged. Ephemeral unless the tab is part of a saved session. |

**[STU-SHL-082] Persistence scope and storage.** Layout is scoped to the OPERATOR and the profile signature, never to the project. Not one of the 58 captured vendor workspace files is project-scoped; per-project scoping would mean every new project starts from the shipped default and the operator re-arranges forever. L1 and L2 are OPERATOR preferences and persist through the shell preference client under an operator key namespace. L3 persists as a Studio-owned sibling schema `hsk.studio_dock_layout@1` through the same endpoint, discriminated by `schema_id` — a SIBLING rather than an extension, so a Studio schema bump cannot invalidate the shell's own layout. Restore MUST validate and fall back to last-known-good, then L2, then L0; a stored blob naming a panel `author_id` that no longer exists MUST drop that panel and log, never fail the whole restore.

**[STU-SHL-083] A signature change is an OFFER, never a silent swap.** Adding a container that changes the profile signature (for example adding a timeline to a page-layout document, changing `{page_spread}` to `{page_spread, timeline}`) MUST offer the layout swap ONCE per document and remember the answer. Silent swapping is the whole-layout form of inspector thrash and is forbidden.

#### 4.5 Default Layout Presets per profile

**[STU-SHL-084] Shared default rules.** These hold for every default layout:

1. Every layout has the Context Bar on top and the Tool Rail on the left.
2. The FIRST tab of the FIRST right group is the `inspector` slot, EXCEPT where a profile's vendor of record demonstrably disagrees — which happens exactly once, in the composition profile ([STU-SHL-089]).
3. The LAST right group is the document-structure group, and `layers` is its first tab.
4. Panels listed together in one bracket are TABS in the same group, in that tab order, first one active.
5. Panels listed under `icon_rail` are REGISTERED in that edge but the edge starts collapsed.
6. Every default placement is traceable to a counted vendor fact and every one is overridable by the operator in one drag, made permanent by L2. Vendor evidence is evidence of what the field converged on; it is not evidence of what this operator prefers. The defaults are a starting position, not an argument.

**[STU-SHL-085] raster (`{artboard}` + `pixel_canvas`).**
- centre: single `canvas` viewer
- left: collapsed to icon rail — `[brushes, preset-library, library, project-bin, effects-browser, actions-macros]`
- right, three groups with shares 0.30 / 0.24 / 0.46 — `[properties, adjustments, effect-controls, masks]` · `[colour, swatches, scopes, info, navigator]` · `[layers, channels, paths, history]`
- right rail: hidden
- bottom: collapsed to icon rail — `[keyframe-timeline, find-replace, preflight, jobs, problems]`

**[STU-SHL-086] vector (`{artboard}` + `vector_network`).**
- right, three groups with shares 0.32 / 0.24 / 0.44 — `[properties, appearance, stroke, gradient, transform]` · `[colour, swatches, align-distribute, pathfinder, navigator, info]` · `[layers, pages-artboards, object-styles, history]`

**[STU-SHL-087] layout (`{page_spread}`).**
- left: expanded 260px — `[pages-artboards, book, long-document, links-placements, library, preset-library, data-merge, reading-order-tags]`
- right, three groups — `[properties, character, paragraph, text-frame, text-wrap, opentype]` · `[text-styles, object-styles, colour, swatches, align-distribute]` · `[layers, transform, history, info]`
- bottom: expanded 180px — `[preflight, find-replace, spelling, problems]`

**[STU-SHL-088] web (`{artboard, site_tree}`) and photo (`{artboard}` + `raw_develop`).** The web default follows the layout profile's shape with `site-tree`, `related-files` and `code-view` in the Browse Dock and `css-designer` in the Inspect Dock. The photo default is EXTRAPOLATED, not captured: the captured photo-catalog application ships a 29-byte workspace stub and no panel layout was recoverable. It is derived from that application's develop-parameter and catalog-model structure plus the photo-side workspaces of another vendor. This is declared debt ([STU-SHL-134]) and the photo layout MUST carry an `extrapolated: true` marker in its L0 record until a capture exists.

**[STU-SHL-089] composition (`{artboard, timeline}`) — REBUILT FROM MEASUREMENT.** This layout was previously extrapolated from a different vendor's video workspace and is now built from 16 measured factory workspaces of the vendor of record for compositing. The five corrections below are normative; implementing the superseded extrapolation is a defect.

- centre: `composition` viewer; splits to composition + footage, or composition + layer, on demand as SIBLING EDITOR GROUPS
- left: expanded 260px — `[project-bin, effect-controls, media-browser, library, preset-library]`
  **`effect-controls` docks LEFT here, co-tabbed with `project-bin`.** This is the measured vendor default in both of its shipped baseline workspaces. The earlier argument for placing it right — "a keyframe editor needs height" — is void: in this document model the keyframe editor is NOT the effect-controls panel, it is the TIMELINE, which is the full-width bottom band. Effect-controls is only the parameter list.
- right, three groups with shares 0.14 / 0.42 / 0.44 — `[preview-render]` · `[properties, align-distribute, audio-meters, effects-browser]` · `[layers, graphics-templates, scopes, history]`
  **`effects-browser` docks RIGHT here** (it docks LEFT in the sequence profile).
- right rail: **HIDDEN.** All 16 measured workspaces of the vendor of record declare no side bars at all, and place the audio panel as a right-dock TAB.
- bottom: expanded 300px, one group — `[keyframe-timeline, timeline]`, **active tab `keyframe-timeline`**
- icon rail registrations: left `[markers, audio-mixer]`; right `[tracker, keyframe-assistant, expression-editor, metadata]`; bottom `[render-queue, problems, jobs]`

**[STU-SHL-090] sequence (`{timeline}`).**
- centre: horizontal split — `source` viewer left, `sequence-program` viewer right
- left: expanded 300px — `[project-bin, media-browser, effects-browser, library, markers, captions, info, history]`
- right, TWO groups with shares 0.6 / 0.4 — `[properties, effect-controls, graphics-templates]` · `[colour, scopes, metadata, find-replace]`. Two, not three: the vendor of record for this profile ships a single tall right pane, and this is the one profile where the three-group answer does not apply.
- right rail: **VISIBLE**, occupant `audio-meters`. This is the only 100-percent placement fact in the corpus (16 of 16 placements).
- bottom: expanded 320px, two groups — `[timeline, keyframe-timeline]` with `timeline` active · `[audio-mixer]`
- icon rail registrations: right `[tracker, expression-editor, keyframe-assistant]`; bottom `[render-queue, problems, jobs]`

**[STU-SHL-091] The two time surfaces are TWO panels sharing ONE time model.** `timeline` (clips on tracks) and `keyframe-timeline` (property rows with keyframe tracks) are separate panels. They default to two TABS in one bottom group with a different active tab per profile. Merging them into one panel with a mode switch is forbidden by [STU-SHL-080]: it forces one-at-a-time with no escape, and the escape would require a multi-instance capability that does not exist. Two panels give the escape for free — the operator promotes one to its own group in a single drag. Both panels read ONE document time model: frame rate, duration, work area, playhead and time display base. A playhead moved in either moves in both and in every viewer and meter. That shared model is what makes them two panels rather than two documents. The `keyframe-timeline` sizing contract: minimum height 300px; property-tree column 320px default and 220px minimum, below which it degrades to a name-only tree with the value column suppressed; time column never below 400px, below which the panel shows a HORIZONTAL SCROLL rather than compressing the time ruler, because a compressed ruler makes keyframe placement unreliable.

**[STU-SHL-092] Configuration and execution are two surfaces.** The `render-queue` panel owns render-item CONFIGURATION — queue items with per-item render settings and per-item output-module settings. The shell Jobs pane owns EXECUTION — queued, running, failed, artifact. `render-queue` hands each item to Jobs on start. A configuration surface carrying hundreds of controls does not belong inside a shell pane that other modules share.

---

### 5. Layout Presets and Task Scopes

**[STU-SHL-093] Layout Preset definition.** A Layout Preset is a named arrangement, stored as the L1 layer of [STU-SHL-081], scoped to a profile signature. It supplies exactly three things and nothing else:

1. dock arrangement,
2. Tool Rail order and pinned tool groups,
3. search ranking bias.

It supplies NO gate of any kind: no gate on menus, no gate on tools, no gate on panels, and no change to document state, selection, colour or history. A preset for `{timeline}` is not offered while a `{page_spread}` document is focused.

**[STU-SHL-094] [STU-UNI-002] restated as Layout Presets — CLARIFICATION, NOT SUPERSESSION.** [STU-UNI-002] of v02.205 remains in force with its meaning unchanged. Its term "TASK MODES" is renamed to "Layout Presets" and its enumeration is re-expressed against profile signatures. The clause as clarified reads: *Operator workflows are organised as named LAYOUT PRESETS over the SAME document and primitives — never separate applications and never separate document states. Selecting a Layout Preset changes dock arrangement, Tool Rail order and search ranking bias only; document state, selection, colour, history, menu membership, menu enablement and tool availability are untouched. Studio adopts the shared-primitive architecture, not an app-switching shell, and stays fully local-first.*

This is a clarification because the original clause already scoped mode switching to "tool prominence and panel layout only" with "document state, selection, colour and history untouched", which IS a layout preset and is NOT the whole-UI persona swap that the operator rejects. The apparent conflict between [STU-UNI-002] and the operator's no-persona-toggle requirement was FALSE and is closed here. No obligation of [STU-UNI-002] is weakened; one prohibition is added — [STU-SHL-055], that a preset may not gate — which the original clause implied and did not state.

**[STU-SHL-095] Shipped Layout Presets.** Presets are named for a JOB or a TASK, never for a persona, a discipline or an application mode. Across the 58 captured vendor workspace files every single name is a job name; not one is a persona. The proposed shipped set is twelve: `Photo`, `Illustrate`, `Layout`, `Retouch`, `Colour`, `Type`, `Composite`, `Animate`, `Edit`, `Track`, `Web`, `Catalog`. The count and the final names are an open operator decision ([STU-SHL-136]); the naming LAW is not.

**[STU-SHL-096] Task Scope is a DIFFERENT mechanism and keeps a different name.** A Task Scope is a named, document-scoped, temporary FILTER read by the availability predicate through the `task_scope` clause kind. It narrows the visible tool set and swaps the Context Bar Tool Zone. It NEVER swaps panels, menus, shortcuts or chrome, and it ALWAYS has a visible exit. Task Scope and Layout Preset MUST NOT be collapsed into one name or one mechanism: they have different lifetimes, different scopes and different effects. The Task Scope contract is [STU-SHL-152].

---

### 5A. The two entry paths and the document tab strip

**[STU-SHL-097] Documents and projects open as TABS inside the Studio viewport.** Studio holds an arbitrary number of open documents and projects SIMULTANEOUSLY, each as a tab in an editor group of the centre tree ([STU-SHL-065]), and each retaining its OWN state: selection, active tool, history, zoom, playhead, per-tab layout override and scroll position. Closing one tab MUST NOT disturb another. The number of open documents is unbounded and the number of editor groups is unbounded; a document tab may be dragged between groups, split into a new group, or dropped onto empty space to create one. A single file beside a chat pane, and ten groups each holding several tabs, are the SAME model with different tab distributions — neither is a special case and neither is a fixed layout.

**[STU-SHL-098] The module entry swaps the whole viewport; the tab strip swaps the document.** Two entry paths coexist and MUST NOT be collapsed into one:

1. **Whole-viewport switch.** Pressing the Studio entry in the module rail swaps the entire Handshake work area to Studio. Every other module keeps RUNNING with live state behind the swap; nothing is torn down, disposed or re-initialised, and switching back restores the previous module's live state rather than a reconstruction of it ([STU-SHL-005]).
2. **Document switch inside Studio.** Once Studio holds the viewport, the document tab strip and the centre tree switch between open documents ([STU-SHL-097]) without touching module state.

A module switch MUST NOT rewrite, close, reorder or reset the Studio document set, and a document switch MUST NOT affect any other module.

**[STU-SHL-099] Cross-module document handoff.** Another module MAY carry a Studio document as an attachment or an embedded view. Two behaviours are required, and both are ONE navigation-target variant on the EXISTING navigation layers, never new routing machinery ([STU-SHL-006]):

1. **Embedded edit in place.** The embedding module renders a Studio viewport inline over the same document authority. The embedded viewport MUST render correctly with all four edge docks absent ([STU-SHL-069] item 4) and MUST expose the same `author_id` addresses as the full viewport, suffixed per [STU-MDL-103] when a second instance exists.
2. **Open in Studio.** Opening the attachment swaps the viewport to Studio ([STU-SHL-098] path 1), shows the SAVED STATE of documents already open, and opens the attachment as a NEW TAB rather than replacing an existing one. If the attachment is already open, its existing tab is focused and no second tab is created.

---

### 6. Pointer gesture arbitration

**[STU-SHL-100] Why arbitration is normative.** Every numeric field is a press-and-drag target ([STU-SHL-185]). A dense panel is therefore a grid of drag targets, inside a drag-target tab, inside a drag-resizable dock, next to a pannable canvas. Four gesture consumers, one pointer. Arbitration is specified as eight rules, all normative, and it is TESTABLE rather than inferable because of rule 8.

**[STU-SHL-101] Rule 1 — CLAIM AT PRESS, no threshold.** The topmost widget under the pointer at PRESS-DOWN claims the gesture for its ENTIRE duration. There is no movement threshold, no ambiguity window and no hand-off. A drag beginning inside a field can never become a panel drag, a tab drag, a canvas pan, a dock resize, a marquee or a scroll. Implemented as a click-and-drag sense on the claiming widget; while a gesture is active no other widget may set the dragged id.

**[STU-SHL-102] Rule 2 — panel and tab drags start from designated chrome ONLY.** A panel drag or a tab drag may START only from the tab, the group title strip or the panel header. Never from the panel body. This removes the conflict structurally rather than dynamically, and it is what every captured vendor does.

**[STU-SHL-103] Rule 3 — splitters are separate strips with distinct cursors.** Splitter visual thickness 4.0px, interactive hit thickness 8.0px, adopted unchanged from the shell's existing splitter discipline. Splitters use a horizontal/vertical resize cursor; a ScrubValue uses a COLUMN resize cursor, visually distinct, plus a 2px gutter tint on hover. If the two cursors read too similarly in practice, the ScrubValue takes a custom cursor — but the two MUST remain visually distinct. A field's hit rect is INSET 8px from any splitter hit strip. The gutter tint is not redundant with the cursor: it is the only cue that works on a pen or touch device where there is no hover pointer at all.

**[STU-SHL-104] Rule 4 — the drag leaves the panel, and the window.** Once a widget owns the gesture, motion is tracked in SCREEN space. The value keeps updating when the pointer leaves the field, leaves the panel, leaves the dock and leaves the window. While a widget owns the pointer: no other widget shows a hover state, no drop target activates, no tooltip fires, no auto-scroll runs, no dock resizes and no canvas pans. `Escape` ABORTS and restores the pre-press value, discarding the transaction rather than committing a no-op. At a screen edge the CURSOR IS HIDDEN and delta continues to accumulate, with the cursor restored at the press point on release. This is deliberately NOT OS cursor warping: warping is fragile across multi-monitor Windows configurations and it MOVES A POINTER THE OUT-OF-PROCESS INSPECTOR IS TRACKING, which would make the inspector's own observations wrong. Cursor warping is forbidden.

**[STU-SHL-105] Rule 5 — wheel.** The wheel over a hovered field steps that field's value and the field CONSUMES the event so the surrounding panel does not scroll. Because in a dense panel most of the surface IS fields, every dense panel MUST reserve a 10px scroll lane on its inner right edge containing no field, and the wheel over any label, group header, separator or blank area MUST scroll the panel.

**[STU-SHL-106] Rule 6 — modifiers.** `Shift` = coarse, ×10. `Ctrl` = fine, ÷10. `Shift+Ctrl` = ×100. `Alt` is NOT a magnitude modifier and MUST NOT become one: `Alt` is already the duplicate, subtract-from-selection and sample-alternate modifier in every captured application, and in the timeline surface where scrubbing matters most it is load-bearing for a dozen keyframe commands. The objection that `Ctrl` is a menu accelerator prefix is defeated by rule 1: a menu accelerator can only fire when no widget owns the pointer, and `Ctrl` held during an owned scrub gesture cannot reach the menu. Modifiers are sampled CONTINUOUSLY during the gesture, not latched at press, and accumulation is in VALUE space so the accumulated value does not jump when the magnitude changes.

**[STU-SHL-107] Rule 7 — ONE undo entry per GESTURE.** Press opens a transaction on the history stack; all motion coalesces into it; release commits ONE entry naming the parameter and its before and after values. Consecutive wheel notches on the same field coalesce into one entry, closed by a 400ms idle timeout, by the pointer leaving the field, by focus loss, by a modal opening or by `Escape`. An accessibility `SetValue` is ONE discrete entry taking the identical clamp and commit path as a drag. Not one entry per pixel and not one per wheel notch.

**[STU-SHL-108] Rule 8 — arbitration is OBSERVABLE.** `studio.gesture.active` MUST be an AccessKit node whose value is the `author_id` of the widget owning the pointer plus the gesture kind, drawn from the closed set `{scrub, tab-drag, panel-drag, splitter, pan, none}`. A test asserting "a drag that began inside a field did not move the panel" is then two string reads, not a screenshot comparison. Without this node, arbitration is inferable but not assertable, and every gesture test becomes a proof by absence of effect.

**[STU-SHL-109] Arbitration acceptance tests (normative).** These MUST exist and MUST pass before any further panel is authored:

1. Press inside a value field, move the pointer across the panel's tab strip and across a dock splitter, release. Assert `studio.dock.state` is BYTE-IDENTICAL before and after, and that `studio.gesture.active` read `scrub` with that field's `author_id` throughout.
2. Press inside a value field, drag beyond the screen edge, release. Assert the value continued to change, the cursor was hidden and restored at the press point, and no OS cursor warp occurred.
3. Press, drag, press `Escape`. Assert the value equals the pre-press value and that ZERO history entries were written.
4. Drag continuously for 500 pointer samples. Assert exactly ONE history entry.
5. Send five wheel notches within the idle window, then wait past it. Assert exactly ONE history entry.
6. Drive the same parameter through the accessibility `SetValue` action. Assert exactly one history entry and the identical clamped result as the drag.
7. Every timed gesture in the product has an UNTIMED equivalent reachable from the menu and the palette, and the inspector targets the untimed path. A timed gesture (a long press, a double-tap reveal) MUST NOT be the only path to any capability: it is nondeterministic for an out-of-process inspector to drive, and a gesture the inspector cannot reliably produce is a gesture the product cannot reliably test.

---

### 7. Accessibility, addressing, and observability

**[STU-MDL-100] The `author_id` grammar (normative, closed).** This extends [STU-MDL-002], which requires a stable `author_id` on every operator-visible surface but does not specify its shape. The shape is:

```
studio.<surface>.<group>.<element>[.<sub>][#<instance>]
```

Examples: `studio.tool.draw-path.pen`, `studio.options.pen.width`, `studio.panel.properties.stroke.width`, `studio.menu.edit.transform.free-transform`, `studio.tab.layers`, `studio.panel.preset-library#2`.

**[STU-MDL-101] What an `author_id` MUST NOT contain.** An `author_id` NEVER contains a REGION, a GROUP ORDINAL or a TAB INDEX. `studio.tab.right.3.layers` is FORBIDDEN; the correct id is `studio.tab.layers`, and its region and group are PROPERTIES on the AccessKit node, not part of its id. The reason is measured, not stylistic: the shell's existing tab bar addresses tabs by index, which shifts under reorder, and this design consciously does not repeat that defect. A dock is user-rearrangeable by law ([STU-SHL-069]); an id containing a position is invalidated by the first re-dock, and every stored layout, every test assertion and every manual anchor pointing at it breaks silently.

**[STU-MDL-102] A numeric field is addressed by its OWNER, not its location.** `studio.panel.<panel>.<param-path>` when it lives in a panel; `studio.options.<tool>.<param>` when it lives in the Context Bar Tool Zone. One field, one id, unchanged when the panel moves docks. A field MUST NOT have two addresses.

**[STU-MDL-103] Charset, derivation, instances, and immutability.**

1. Charset `[a-z0-9-.]` only, kebab-case within a dotted path. No locale text, no vendor branding, never derived from position, sibling index or label text.
2. `stable_id = "hs-" + author_id` with dots replaced by hyphens, derived MECHANICALLY and never authored twice. `studio.tool.draw-path.pen` → `hs-studio-tool-draw-path-pen`.
3. A second simultaneous instance of a panel takes a `#<n>` suffix; the FIRST instance is unsuffixed, so existing addresses never change when a second is opened.
4. An id, once shipped, is NEVER reused and NEVER renamed. A rename ADDS a new id plus an `aliases[]` entry, and the inspector accepts either.

**[STU-MDL-104] Node id space.** The `studio.` prefix MUST be registered in the shell's declared hashed-`author_id` prefix list, and the egui id MUST be derived from the `author_id` string so Studio lands in the hashed id space, disjoint by construction from the shell's small fixed node-id band. Studio MUST NOT consume fixed node ids, because its panel and tool counts are dynamic. Where the Studio module menus need title node ids, they are derived from `author_id` strings in the hashed space exactly as the shell already does for its dynamic leaf nodes, so the fixed band stays reserved for the shell's eight menus and module menus are additive without renumbering.

**[STU-MDL-105] The accessibility node MUST carry numeric and temporal metadata — a DECLARED GAP stated as a requirement.** The shell's UI tree node today is `{ id, author_id, node_id, role, label, value, disabled, actions, bounds, children }`. It carries a STRING value and NO numeric minimum, no maximum, no step and no unit, and no temporal state. The consequence is precise and must not be softened: the accessibility set-value action ALREADY EXISTS and would happily set a value whose clamp the inspector cannot check, so every scrub control would pass a visual-inspection gate by being clickable while its actual numeric contract went completely untested. That is exactly the class of defect the inspector exists to catch. The temporal half is worse: on an animated property the reported value is a function of the playhead rather than a stored constant, so a test asserting a scrub result on an animated property is UNFALSIFIABLE — and animated is the majority case in a composition, since 1,246 of 1,573 typed effect parameters in the reference corpus are keyframable.

The node MUST therefore additionally carry, all optional, all populated from the same ParamSpec and document time the control renders from:

| Field | Type | Source |
|---|---|---|
| `numeric_value` | f64 | the value evaluated at the current time |
| `numeric_min` | f64 | `ParamSpec.hard_min` |
| `numeric_max` | f64 | `ParamSpec.hard_max` |
| `numeric_step` | f64 | `ParamSpec.step_default` |
| `numeric_jump` | f64 | `ParamSpec.step_coarse` |
| `unit` | string | `ParamSpec.unit` or `display_unit` |
| `temporal_state` | enum | `Static` \| `Animated` \| `Expression` \| `AnimatedAndExpression` |
| `time` | rational | the playhead the value was evaluated at |
| `keyframed_at_time` | bool | whether a keyframe exists at that playhead |

This is an ADDITIVE change to the shell's accessibility snapshot type and it is a HARD PREDECESSOR of the scrubbable numeric control, not a follow-up ([STU-SHL-110], step SHL-P-04). It is a product-wide improvement, not a Studio-local one.

**[STU-MDL-106] Observability nodes (normative).** Three nodes MUST exist beyond the per-element tree. They are the highest-value observability affordances in this design and each replaces a proof-by-absence with a string read.

| Node | Value |
|---|---|
| `studio.dock.state` | a JSON projection of the WHOLE dock model — regions, groups, tab order, active tabs, sizes, collapse states — so a GUI regression test is a string comparison rather than a screenshot diff |
| `studio.slot.<slot>.resolved` | the `author_id` the slot resolver chose, the matched selector, and the winning priority |
| `studio.gesture.active` | the `author_id` owning the pointer plus the gesture kind ([STU-SHL-108]) |

**[STU-MDL-107] Availability is exposed on the node.** Every AccessKit node for a Studio element MUST carry `availability_state`, `reason_code`, `reason_text` and `remedy_command_id` alongside `disabled`, and their values MUST be the SAME evaluation the tooltip and the command API return ([STU-SHL-019]). A surface that renders a reason it cannot expose has failed [STU-MDL-002].

**[STU-MDL-108] Snapshot truncation is explicit.** The centre editor tree is unbounded by law, so the accessibility snapshot can grow large. Studio MUST respect the shell's existing maximum-snapshot-node cap and MUST report truncation EXPLICITLY in the snapshot rather than silently emitting a partial tree. No group cap may be imposed to make the snapshot smaller ([STU-SHL-065]).

**[STU-MDL-109] Steering path law.** Every surface in this sub-section is steerable through TWO paths and only two: the typed command API for authority mutation, and the accessibility tree by `author_id` for visual verification and steering. Both resolve to the same document authority through the promotion lifecycle. OS-level input injection is forbidden ([STU-QUIET-002]), and a negative-test harness MUST confirm that no shell surface responds to simulated global keyboard or mouse input.

---

### 8. Build order

**[STU-SHL-110] The prerequisite chain is normative.** The following ordered steps are the dependency structure of the operator shell. Each row is a distinct unit of work with a named predecessor set and a named acceptance surface; a downstream step MUST NOT be started before its predecessors have passed their acceptance. The risk in this design is concentrated in exactly three steps — SHL-P-03, SHL-P-09 and SHL-P-12 — and none of it is in the panels; those three are the ones worth proving against the deliberately small spine of [STU-SHL-078].

| Id | Stage | Step | Depends on | Acceptance |
|---|---|---|---|---|
| SHL-P-01 | 1 | Replace the Studio module definition with a real one whose default pane is the Studio viewport, and RETIRE the snapshot test that pins it to a ported web-source table in the SAME change | — | switching to Studio opens a creative surface, and the retired snapshot test is gone |
| SHL-P-02 | 1 | Replace module-switch tab-list rewriting with a viewport swap over live state | SHL-P-01 | switch away and back with three Studio documents open; all three survive ([STU-SHL-005]) |
| SHL-P-03 | 2 | Replace the hard-coded 2×2 splitter with the unbounded centre tree, KEEPING the clamp discipline and the splitter hit thickness permanently | SHL-P-02 | N groups with tabs, cross-group document drag, serialisation round-trip ([STU-SHL-065]) |
| SHL-P-04 | 3 | Accessibility node gains numeric and temporal metadata | — | [STU-MDL-105] fields present and populated |
| SHL-P-05 | 3 | Tool-prose extraction pass: recover the join between a tool's name and its summary from the assembly IL of the one vendor that ships per-tool prose | — | mechanically bound tool-level prose rises from 0% toward the 58% ceiling ([STU-MAN-109]) |
| SHL-P-06 | 4 | The descriptor generator, the generated id enum, and the UserManual seed rows | SHL-P-05 | a seeded Studio tool is retrievable from the canonical manual store ([STU-MAN-100]) |
| SHL-P-07 | 5 | One descriptor-driven tooltip helper; then migrate the legacy hover-text call sites | SHL-P-06 | the helper emits BOTH the tooltip and the accessible description from one record ([STU-SHL-245]) |
| SHL-P-08 | 5 | Widen the shell command record into the UiDescriptor and derive the menu from it | SHL-P-06 | menu, palette and rail all dispatch from one registry ([STU-SHL-015]) |
| SHL-P-09 | 6 | ScrubValue with the single clamped write path | SHL-P-04, SHL-P-08 | [STU-SHL-210]; hard and soft bounds separate from the FIRST commit; TemporalState ships WITH the widget |
| SHL-P-10 | 7 | The seven-region dock host, one tree per region | SHL-P-03, SHL-P-09 | Inspect Dock functional, other regions stubbed; [STU-SHL-070] transaction discipline in place |
| SHL-P-11 | 7 | `StudioPanelRegistry` plus the eight-panel spine | SHL-P-10 | the eight panels of [STU-SHL-078] present in every default layout |
| SHL-P-12 | 8 | The availability predicate and the slot resolver, PROVEN through the inspector end to end | SHL-P-11 | [STU-SHL-061] byte-identical reselect test and [STU-SHL-109] gesture tests pass |
| SHL-P-13 | 9 | Document time model | SHL-P-11 | one shared frame rate, duration, work area, playhead and time display base |
| SHL-P-14 | 10 | Layer property tree and keyframe model in the domain layer | SHL-P-13 | **BLOCKED until [STU-SHL-130] is settled** |
| SHL-P-15 | 11 | `keyframe-timeline` panel | SHL-P-14, SHL-P-12 | two columns, movable divider, shared row height and scroll, untimed reveal equivalents |
| SHL-P-16 | 11 | Recompute shortcut arbitration over all five binding tables | — | blocks ONLY the shortcut freeze ([STU-SHL-131]) |
| SHL-P-17 | 12 | Graph editor as a MODE of `keyframe-timeline` | SHL-P-15 | temporal interpolation only; the spatial half waits on [STU-SHL-132] |
| SHL-P-18 | 12 | Expression evaluation and the inline expression editor | SHL-P-14, SHL-P-09 | reuses the shell code editor; structured evaluation errors surfaced on the property row and in Problems |
| SHL-P-19 | 13 | `expression-editor` panel | SHL-P-18 | task-episodic dockable editor over the same evaluation |
| SHL-P-20 | 13 | Clip timeline panel and the trim grammar | SHL-P-13, SHL-P-12 | tracks, clips, transitions, audio lanes and the nine timeline-edit tools |
| SHL-P-21 | 14 | `render-queue` panel and output modules | SHL-P-20, SHL-P-15 | configuration surface hands execution to Jobs ([STU-SHL-092]) |
| SHL-P-22 | continuous, behind SHL-P-12 | The remaining 82 panels | SHL-P-12 | content work behind a stable registry API; explicitly parallelisable |

**[STU-SHL-111] Defects that MUST be replaced, not preserved.** Three shipped surfaces are ports of a previous web implementation and are recorded here as defects so a later implementer does not preserve them out of deference:

1. A hard-coded 2×2 splitter carrying the previous implementation's own field names, ported verbatim because an earlier acceptance criterion asserted those names by string. Its own module documentation already names the tiling engine as the intended replacement, and that engine is already a dependency. Replacement, not rewrite. (SHL-P-03)
2. Module switching that REWRITES the active pane's tab list to a module's canonical tab set. With Studio documents modelled as panes this would destroy the open document set on any module switch. (SHL-P-02)
3. The Studio module definition itself, which is another module's entry reordered — its default pane and every one of its tabs are model-runtime surfaces, not one creative surface — pinned in place by a snapshot test proving no drift from the web source. The test MUST be retired in the same change or it will FAIL the correct implementation. (SHL-P-01)

**[STU-SHL-112] Scope boundary for shell-wide steps.** SHL-P-04, SHL-P-07 and SHL-P-08 fix existing product-wide defects and benefit every module. Whether each is executed inside the Studio work packet or as a separate shell packet is an open operator decision ([STU-SHL-136]); the DEPENDENCY is not open — Studio cannot ship the scrub control before SHL-P-04, or the menu before SHL-P-08.

---

### 9. Declared spec debt

**[STU-SHL-120] What this section is.** The items below are DECLARED SPEC DEBT: they are NOT decisions, and no clause elsewhere in this sub-section may be read as having settled them. Each names what is unknown, what it blocks, what would close it, and what happens if it is resolved late. An implementer encountering one of these MUST stop and escalate rather than choosing.

**[STU-SHL-130] SD-1 — THE LAYER-VERSUS-NODE COMPOSITING FORK. NOT DECIDED. Settle before SHL-P-14.**

*The question.* Is Studio's compositing document model LAYER-based (a composition holds an ordered stack of layers, each carrying a property tree) or NODE-based (a directed graph of operations)?

*Why it is a fork and not a rendering detail.* The two are different DOCUMENT MODELS with different undo semantics, different parallelism, and different model-steerability. The vendor of record for compositing that the green room captured is layer-based; high-end visual effects practice is predominantly node-based. Serving visual-effects work properly may mean a node graph.

*What the evidence supports.* The LAYER model fully: 1,326 catalogued property match names, 759 containment edges, 13 property-group topics, the keyframe record format and the expression container are all measured. A node model has NO reference basis in the corpus and would need its own.

*What already exists in the product.* Handshake already ships node-graph precedent in its own shell — a graph surface and a canvas board — so the fork is not a build-versus-buy question, it is a document-model question.

*The weakest form of the question that IS already answered.* A graph PROJECTION over a layer document is already required and already field-precedented: the captured application ships a read-only dependency view over nested compositions, and this design carries it as the `comp-flowchart` panel ([STU-SHL-079]). That is a strictly weaker claim than needing a graph document model, and it is settled.

*What it blocks and why the timing matters.* It changes SHL-P-14, the domain-layer property and keyframe model, and it would change the `keyframe-timeline` row model. If it is resolved AFTER SHL-P-14 the cost is a REBUILD of the document model, the undo model and the timeline row model together — not a rework. This is the one open item in this sub-section whose cost of late resolution is a rebuild.

*Disposition.* SHL-P-14 is BLOCKED until an operator decision is recorded. Owner: operator.

**[STU-SHL-131] SD-2 — THE SHORTCUT SPINE BREACH. Shipped chord sets are FROZEN-BLOCKED.**

*The measurement.* The union of distinct chords across four captured binding tables is 448, of which 211 are claimed by two or more applications, 18 agree semantically and 193 conflict — a 91% conflict rate. Eighteen agreeing chords were declared a FROZEN SPINE, immovable in shipped sets.

*The breach.* That computation EXCLUDED a fifth application which ships **673 keyboard bindings across 32 contexts**. Folding it in is not a caveat, it is a recomputation. One conflict is already known and is a spine breach: **`Ctrl+P` is one of the 18 frozen chords (Print), and the fifth application binds it to its puppet tool in a canvas-tool context.** This is the first measured contest of a frozen chord. A frozen spine that is breached SILENTLY is worse than one that was never frozen.

*Other already-visible contests, recorded so the recomputation starts from a list rather than a blank page.* `G` (gradient vs pen group, now 2-of-4 with a live objection), `W` (rotate vs wand group), `Y` (anchor-point/pan-behind vs history brush), `Q` (shape/mask cycle vs quick mask), `C` (camera vs crop, where crop is one of the eight tools present in five source applications), `E` (toggle-effects vs eraser), `U` (reveal-modified vs shape group), `J`/`K` (previous/next keyframe vs healing group), `Ctrl+B` (paint tool vs bold, a fifth claimant and a TOOL rather than a text attribute), `Ctrl+T` (type tool vs free transform, changing the vote from 1-of-4 to 2-of-5 against the existing ruling), and five function-key keyframe bindings colliding with the panel-toggle reserve. The bare digits 1–6 are newly claimed by camera and 3D-gizmo commands.

*Proposed handling for the spine breach, NOT a decision.* The freeze holds, Print keeps `Ctrl+P` globally, and the puppet tool takes a CANVAS-context binding — which rank 2 already permits. Recorded as a proposal because a frozen chord's contest must be settled by the operator, not by an implementer.

*What it blocks.* Freezing ANY shipped chord set, and nothing else. Panels, docks, the predicate and the controls are all unblocked. The two contexts `graph_editor` and `expression_editor` are added to the precedence order of [STU-SHL-042] by this fold-in and MUST be present in the recomputation. SHL-P-16 is the closing action. Owner: operator.

**[STU-SHL-132] SD-3 through SD-8 — carried capture gaps.** Each is recorded as debt, not as a decision, and each names its closing action.

| Id | Debt | Blocks | Closing action |
|---|---|---|---|
| SD-3 | **Menu expansion counts are inflated by a registry merge defect.** The capability registry merged rows on normalised NAME alone, conflating unrelated capabilities across applications. Where a node was independently rebuilt the overcount was 3.6× (tools: registry 1,270 vs rebuild 362) and 9.7× (panels: registry 814 vs dedup 90). Roughly 2,630 budgeted leaves across the other expansion nodes inherit the defect unchecked. | Any expansion node becoming a microtask with a leaf count as an acceptance criterion | Re-key the merge on `(normalised name, kind, domain)` with a provenance-preserving audit trail, OR rebuild that family from per-application sources, BEFORE the node is authored. Treat 1,533 explicit leaves as designed and every expansion figure as an UPPER BOUND. Only `WORKSPACE > Tools` (362) and `WORKSPACE > Panels` (90) are exempt. |
| SD-4 | **Spatial keyframe interpolation has no captured reference.** The corpus exercises temporal interpolation only; spatial tangents appear in the expression vocabulary but no shipped preset exercises them. | The spatial half of SHL-P-17 and all on-canvas motion-path handle editing | A targeted capture pass, or an explicitly authored design recorded as `handshake_authored`. Building it from memory is exactly the speculation the green room exists to avoid. |
| SD-5 | **Expression argument signatures are not bound to their functions.** 37 argument signatures are pooled beside 333 identifiers but not stored adjacent to the functions they belong to, and the capture explicitly refuses to assert the binding. No type library ships on disk, so return and parameter types are unrecoverable offline. 206 of the 333 identifiers carry no category. | Expression autocomplete and signature help. NOT the editor itself (SHL-P-18, SHL-P-19). | An online documentation pass, or authored signatures marked `handshake_authored`. |
| SD-6 | **Three of nine captured applications have no menu hierarchy.** Two raster/vector applications and one photo-catalog application. One states in its own capture that no mapping exists in the install from its command ids to a menu path and that the hierarchy is not present in any readable install file. | Nothing structurally; the menu tree of [STU-SHL-023]–[STU-SHL-037] is designed, not transcribed | Recover one hierarchy from compiled executable resources and the other from plugin menu registrations, as was already done successfully for one effect menu. Absent a capture, a menu leaf MUST NOT be invented and attributed to a vendor. |
| SD-7 | **143 tool ids and 53 panel ids of one vendor remain unbound to names.** The ids appear in the binaries only as compiled 32-bit literals and the string tables are keyed by English source text; the capture deliberately refuses to guess. That vendor's workspace STRUCTURE is reliable; no claim binds a specific id to a specific name. | Attributing a specific named tool to a specific captured slot | The same extraction pass as SHL-P-05. |
| SD-8 | **Four of ten composition layer-type identifiers are unconfirmed.** Six were found on disk by identifier; four (`solid`, `null`, `adjustment`, `guide`) were not. Their EXISTENCE is not in doubt — all four appear in the captured layer menu — but their identifiers are not confirmed. | Nothing structurally | These four MUST NOT be asserted by identifier in any microtask acceptance criterion until confirmed. |

**[STU-SHL-133] Debt that is NOT open.** The following were open in the source deliberations and are CLOSED by this sub-section, recorded here so they are not reopened: whether disabled menu items grey or hide (closed by [STU-SHL-018], with the operator's own listing of it as an open question noted and answered with its reason); whether panel layout persists per document type, per project or per tab (closed by [STU-SHL-081] and [STU-SHL-082]); the coarse and fine scrub modifiers (closed by [STU-SHL-106]); whether the composition default layout is extrapolated (closed by [STU-SHL-089], now measured); whether the clip timeline and keyframe timeline are one panel (closed by [STU-SHL-091]); whether availability is two-valued or three-valued (closed by [STU-SHL-050]); and whether "task mode", "workspace" and "named preset" are three things (closed by [STU-SHL-002] and [STU-SHL-094] — they are one thing, the Layout Preset).

**[STU-SHL-134] The photo default layout is extrapolated and MUST say so in its own record.** The shipped L0 record for the photo profile signature MUST carry `extrapolated: true` together with a `derived_from` note stating that the layout was built from that application's develop-parameter and catalog-model structure plus another vendor's photo-side workspaces, because the captured workspace file is a 29-byte stub and no panel layout was recoverable. A layout carrying that marker MUST NOT be cited as measured evidence in any acceptance criterion, and replacing it when a capture lands is a layout change rather than a spec amendment. Every other shipped default layout MUST carry `extrapolated: false`, so the distinction between a measured and an inferred default is readable at runtime rather than only in this sub-section.

**[STU-SHL-135] The 362-tool count is an auditable JUDGEMENT, not a measurement.** 549 raw names were normalised and collapsed through an AUTHORED 240-entry cross-vendor synonym table. Every merge MUST ship inline as `vendor_variants` on each tool row, so any merge is visible and reversible from the artefact alone. Two merges are known to be debatable and are named so a reviewer does not have to find them: a clone stamp merged with a clone brush, and a vector freehand tool merged with a raster pencil. This is not a blocker; it is recorded so 362 is understood correctly.

**[STU-SHL-136] Open operator decisions.** The following require an operator decision and MUST NOT be settled by an implementer. Each names what it decides.

| Id | Decision | Decides |
|---|---|---|
| OD-1 | Two left rails (shell activity rail plus Tool Rail) or Studio tools injected into the shell rail | 76px of horizontal chrome; recommendation is to keep two rails and auto-collapse the shell rail when Studio is active, a capability it already has |
| OD-2 | Row height: 24px grab-friendly or 20px vendor-dense | roughly 20% of the visible parameter count per group; the single biggest density lever in the design |
| OD-3 | Whether live adjustment tools (36) and live filter tools (44) stay TOOLS or become COMMANDS that create a layer | whether the tool count is 362 or roughly 296. Recommended split on the gesture test: an operation needing a canvas gesture to say WHERE it applies stays a tool; an operation that is a parameter dialog applied to a whole layer becomes a command |
| OD-4 | How many Layout Presets ship and under what names | the shipped set named in [STU-SHL-095] |
| OD-5 | How many Task Scopes ship | the shipped set named in [STU-SHL-153] |
| OD-6 | Auto-resolved sibling tab cap | the cap declared in [STU-SHL-060] |
| OD-7 | Whether the shell-wide steps SHL-P-04, SHL-P-07 and SHL-P-08 are Studio scope or shell scope | whether Studio is blocked on a shell packet ([STU-SHL-112]) |
| OD-8 | Whether the shared asset library is the graph surface or the asset-and-file-manager module | every library leaf in FILE, INSERT, OBJECT and COLOR. [STU-UNI-003] names one; the green-room sprint contract names the other. Labels are written destination-neutral until this is settled |
| OD-9 | Whether Studio ships generative surfaces routed to Handshake's own model runtime | whether the excluded-AI list is an exclusion list or a re-implementation list |
| OD-10 | Whether the wheel default over a dense panel inverts (wheel scrolls, modifier steps) | a one-line default change if the 10px scroll lane proves too narrow in the visual debugger |
| OD-11 | Whether the operator preference store is operator-scoped or workspace-scoped | the L1 and L2 persistence layers ([STU-SHL-082]) |
| OD-12 | Which artefact or blob tier Studio binds to for bulk binary | raster tiles, video media, brush bitmaps, LUTs and fonts. SurrealDB holds records and references; bulk bytes belong in content-addressed artefact storage and this is NOT a licence for a second database ([STU-SHL-007]) |
| OD-13 | Whether every one of the 362 tools needs its own model-invocation path and manual entry, or whether model-invokability is satisfied at command-registry level | the floor for how much of this design is mandatory rather than desirable under [STU-CON-007] |

---

### 10. Obligations

**[STU-SHL-137] Universal command contract.** Every command introduced by this sub-section — every menu leaf, every dock gesture, every panel open/move/close, every Layout Preset selection, every Task Scope entry and exit — MUST satisfy [STU-CON-007] in full: model-invokable through the one typed command API, parallel-safe through the per-file CRDT and lease path, deterministic, and visually verifiable through the render harness and the accessibility inspector without foreground focus steal.

**[STU-SHL-138] Validation descriptors.** This sub-section contributes at minimum these `StudioValidationDescriptor` checks (14.24): `menu_leaf_without_command`, `command_without_menu_path_or_typed_palette_only_reason`, `command_id_primary_leaf_duplicated`, `availability_disagreement_between_projections`, `author_id_contains_position`, `author_id_reused_after_rename`, `panel_registered_in_two_trees`, `panel_registered_in_zero_trees`, `dock_state_changed_during_scrub_gesture`, `gesture_claimed_by_two_widgets`, `undo_entries_per_gesture_not_one`, `layout_preset_changed_availability_state`, `slot_resolution_not_byte_identical_on_reselect`, `expansion_leaf_count_asserted_from_budgeted_source`, `shortcut_shadowed_in_same_context`, `snapshot_truncated_without_report`.

**[STU-SHL-139] Manual obligation.** Every menu leaf, every region, every panel, every Layout Preset, every Task Scope and every gesture rule in this sub-section MUST have a UserManual entry per [STU-MAN-001] and MUST be reachable by the four search axes of [STU-MAN-004], including reverse lookup from an `author_id` to the entry that documents it. Every closed enumeration in this sub-section — the seven regions, the seven panel states, the twelve clause kinds, the three availability states, the twelve reason codes, the ten viewer kinds, the eleven shortcut contexts, the six gesture kinds — MUST appear in the model-facing manual layer as its LITERAL token list, never as prose. The generation contract that makes this satisfiable at this scale is 14.32.


---

### 11. Microtask Derivation

**[STU-SHL-116] Derivation rule (NORMATIVE).** The operator-shell microtask set is derived from this sub-section MECHANICALLY, not editorially. ONE microtask corresponds to ONE of the following units, and to nothing else:

1. **Each numbered clause of this sub-section**, except the bookkeeping clauses named in [STU-SHL-117]. A clause states a contract, a rule, a structure or an enumeration that can be implemented and PROVEN independently, and it yields one microtask whether or not the sentence carrying it happens to use MUST: a stored contract may be stated in the indicative mood.
2. **Each ROW of a catalogue table** — a table whose FIRST COLUMN names a separate implementable subject rather than a facet of one subject. Each such row is its own microtask, because one microtask reading "362 tools" or "90 panels" is not implementable and would let the work disappear behind a number. The remaining columns of the row are that microtask's acceptance criteria.
3. **Each enumeration table, taken WHOLE** — its members are acceptance criteria of one microtask, not separate microtasks.
4. **Each command, shortcut, binding, preset or template table, taken WHOLE.** Binding a key is not a unit of implementation work and MUST NOT be one microtask per row.
5. **Each parameter table, taken WHOLE**, where the row's seven bound fields are its acceptance criteria.

The two catalogue tables in this sub-section are the expansion-node register of [STU-SHL-038], whose 13 rows are 13 list-recovery units, and the panel catalogue of [STU-SHL-113], whose 90 rows are the 90 panel identities. The persistence-layer table of [STU-SHL-081] and the observability-node table of [STU-MDL-106] are catalogues too: a persistence layer and an accessibility node are each separate implementable subjects.

**[STU-SHL-117] Clauses that yield NO microtask.** Exactly one class, and it is deliberately narrow: the four clauses of this derivation sub-section itself — [STU-SHL-116], [STU-SHL-117], [STU-SHL-118] and [STU-SHL-119] — because they describe how the spec is read rather than what Studio does. **Every other clause in this sub-section yields.** An earlier revision of this clause declared a much larger non-yielding set covering cross-references, universal obligations and framing statements; that was wrong in the dangerous direction, because a clause declared non-yielding drops out of the derived set silently. A clause that only points elsewhere still yields the work of making the pointer true, and an obligation that attaches to every microtask still yields the work of proving it holds.

**[STU-SHL-118] An open item still yields a microtask.** A clause that declares an open operator decision, a blocked dependency or a carried capture gap STILL yields a microtask. Its FIRST acceptance criterion is resolving that dependency — obtaining the operator decision, running the named closing action, or recording the waiver — and its remaining criteria are the work the resolution unblocks. Nothing declared here may silently disappear from the derived set because it is unresolved. A microtask derived from a debt item is BLOCKED, not absent, and carries its blocking dependency in its own body. SHL-P-14 in particular is derived and BLOCKED on [STU-SHL-130]; it is never omitted. A BUDGETED expansion-node row is likewise derived, with list recovery as its first acceptance criterion ([STU-SHL-038]).

**[STU-SHL-119] Yields index.** Applying [STU-SHL-116] through [STU-SHL-118] to this sub-section yields the counts below. Every count is enumerated from the module text, not estimated, and the groups partition the sub-section with no clause counted twice and none omitted.

| Unit group | Clauses | Yields |
|---|---|---|
| Shell composition and module contract | [STU-SHL-001]-[STU-SHL-009] | 10 |
| Menu position, index promise, one registry and the seven invariants | [STU-SHL-010]-[STU-SHL-021] | 12 |
| The full menu tree, one unit per top-level menu | [STU-SHL-022]-[STU-SHL-037] | 16 |
| Expansion-node register, tool submenu shape, menu-only nodes, palette index | [STU-SHL-038]-[STU-SHL-041] | 17 |
| Shortcut policy, ranks, shipped sets and the conflict report | [STU-SHL-042]-[STU-SHL-045] | 4 |
| The availability predicate | [STU-SHL-046]-[STU-SHL-055] | 11 |
| The slot resolver | [STU-SHL-056]-[STU-SHL-062] | 7 |
| Dock structure, regions, viewer kinds, panel states, constraints, registry | [STU-SHL-063]-[STU-SHL-071] | 9 |
| Panel density | [STU-SHL-072]-[STU-SHL-074] | 4 |
| Panel inventory, classes, merge law, the shipping spine | [STU-SHL-075]-[STU-SHL-080] | 6 |
| Layout persistence cascade | [STU-SHL-081]-[STU-SHL-083] | 7 |
| Default Layout Presets per profile, and the two time surfaces | [STU-SHL-084]-[STU-SHL-092] | 9 |
| Layout Presets, Task Scope boundary, document tabs, the two entry paths | [STU-SHL-093]-[STU-SHL-099] | 7 |
| Pointer-gesture arbitration and its acceptance tests | [STU-SHL-100]-[STU-SHL-109] | 10 |
| The 90-panel catalogue, one unit per panel identity | [STU-SHL-113] | 91 |
| Build order and defect replacement | [STU-SHL-110]-[STU-SHL-112] | 3 |
| Declared spec debt and open operator decisions | [STU-SHL-120]-[STU-SHL-136] | 8 |
| Obligations, validation descriptors and the manual binding | [STU-SHL-137]-[STU-SHL-139] | 3 |
| Addressing, accessibility node contract and observability nodes | [STU-MDL-100]-[STU-MDL-109] | 13 |
| **Module total** | 138 clauses | **247** |

The rows that are not one-per-clause are the four catalogue tables and the three enumeration tables, and each is enumerated in the module text rather than asserted: the expansion-node register contributes 13 list-recovery units ([STU-SHL-038]); the panel catalogue contributes 90, one per panel identity ([STU-SHL-113]); the persistence cascade contributes 4, one per layer ([STU-SHL-081]); the observability nodes contribute 3 ([STU-MDL-106]); and the region-name, availability-state and panel-density tables each contribute one enumeration unit beside their clause.

**[STU-SHL-119A] Anchor binding.** A microtask derived from this sub-section cites its clause anchor directly, and a catalogue-row microtask additionally cites its subject. A microtask staged before this sub-section landed carries `spec_anchor_status = "PROVISIONAL"`; binding it to an anchor here clears that status. A microtask that cannot cite an anchor in this sub-section, in 14.31 or in 14.32 is out of scope for the operator shell and MUST be re-derived or retired, not activated.
