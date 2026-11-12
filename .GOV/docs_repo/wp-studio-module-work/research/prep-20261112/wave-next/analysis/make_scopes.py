"""Scratch: write draft scope files in the format wave_from_scope.py reads (research/scope-20261107-<area>.json).

Drafts only: they are written to the wave-next scratch folder, never into the kernel. REQUIRED keys come from
tools/lead/wave_from_scope.py; `exposure` is optional (lanegen default low).
"""
import json
import os
import sys

OUT = sys.argv[1]
BASE = "b57c82d1"
ATTR = "required: every commit step takes the caller's context (CommandContext or ActorContext); never a default actor"
DIAG = "producer_through_observe: one terminal frame per command, codes and counters only"
SPEC = "`.GOV/spec/master-spec-v02.229/spec-modules/14-studio-creative-suite.md`"

HOW = f"""How to work:
1. Read the code first. Use the spec's names and cite clauses ({SPEC}). Use only names already in the spec, MTs or code.
2. Red test and focused test per item. Typed commands with typed parameters and typed refusals; each mutating command is one named undo step, carries the caller's attribution and survives save and reopen.
3. Commands register in the existing catalog: your domain file under `harness-core/src/action/commands/` (the per-domain `ROWS` tables that `commands.rs` concatenates into `RUN_COMMANDS`), or a `viewer.<method>` row.
4. V4-06: a short list of sibling sites, fixed or named. Hostile input: bounded work and allocation, typed refusals, no panic.
5. About 6 commits. Width over polish. State what you cut.
MTs: read those your slice proves and name them in your report; the lead records them. Do not edit MT files."""

WHY = ("Why this lane: the spec-coverage audit of 2026-11-11 (`S/research/spec-coverage-20261111-rollup.json` and the group file named below) "
       "judged every requirement against origin 87f331f9. Rows named here as MISSING or PARTIAL are your acceptance: a row moves when a model can "
       "run it through a typed command (JSONL loop of `harness/src/cli/drive.rs`) and the result reaches save and export.")

LABEL = ("Facts are READ by the wave planner at origin b57c82d1 with git grep, read-only, not run, unless marked 'planner READ at 0a733db8' "
         "(research/width-plan-20261108-*.json, older pin: re-check). Treat them as claims and check them.")


def scope(id_, summary, crates, paths, target_body, closes, mts, tree, suites, exposure="medium", diag=DIAG, out_of_scope=""):
    low = id_.lower()
    slug = low.replace("smb-", "", 1)
    target = f"{summary}\n\n{WHY}\n\n{LABEL}\n\n{target_body}\n\n{HOW}\n\nOut of scope: {out_of_scope or 'panels and drawing (except where this lane is the GUI lane), RF-01, render-gpu, any Cargo.lock change without asking the lead.'}"
    return slug, {
        "id": id_, "file": "hsk-studio-" + slug, "brief": "lane-" + low,
        "crate": ", ".join(crates), "crates_head": ", ".join(c.split(" ")[0].replace("hsk-studio-", "") for c in crates),
        "summary": SHORT[id_].replace('"', "'")[:200], "target": target, "diag": diag, "attr": ATTR, "exposure": exposure,
        "paths": "; ".join(paths), "suites": suites, "closes": closes, "mts": mts, "tree": tree, "disk": "d",
    }


SHORT = {
    "SMB-GUI-MENU-1": "B6 in the Vulkan window: 19 titles open menus from one descriptor source, greyed items show reasons, remedies run, items dispatch typed commands, tooltips L0 and L1",
    "SMB-TYPE-CHARACTER-ATTRIBUTES-1": "Per-range character attributes and OpenType features into Type, script and language itemization, replace_text newline and empty-story forms",
    "SMB-BOARD-1": "Whiteboard first slice: board profile, container, sticky, shape_with_text, section and connector with factory, read and mutate commands and PNG export",
    "SMB-WEB-CODE-1": "Web code intelligence: document types, tag libraries and attribute vocabulary, CSS property surface, hints, validation and formatting as typed web commands",
    "SMB-LAYOUT-DELETE-CASCADE-1": "page.delete with content and text-frame delete re-link threads in one undo step; frame-holding group, page and spread deletes; autoflow and smart reflow",
    "SMB-DS-VARIABLES-AUTHOR-1": "Design variables as typed commands, plus history-aware component text compaction and .fig design-state prune at save",
    "SMB-PS-FILTER-ADJUST-COMMANDS-1": "effect.* commands over the 25 smart filter kinds, adjustment-layer create, edit and apply, and crop keys in the develop DNG XMP",
    "SMB-WEB-SITE-2": "Templates, library items and snippets on the site, change-link-site-wide, and session Action::Read for read-only verbs",
}

lanes = []

lanes.append(scope(
    "SMB-GUI-MENU-1",
    "Build-order step B6 in the Vulkan window: the 19 menu titles open real menus derived from one descriptor source, greyed items show their availability reason, a remedy runs, items dispatch typed commands, tooltips L0 and L1",
    ["hsk-studio-native-host (feature document: menu popups, availability, tooltips)", "kumihimo-ui (additive menu_popup, menu_item, tooltip widgets)",
     "hsk-studio-harness-view (one new egui-free menu tree module)", "hsk-studio-controls (read; additive only)"],
    ["hsk-studio-native-host/src/gui/chrome.rs", "hsk-studio-native-host/src/gui/mod.rs", "hsk-studio-native-host/src/gui/preset.rs",
     "hsk-studio-native-host/tests/gui_menu.rs (new)", "kumihimo-ui/src/widgets.rs (additive)", "hsk-studio-harness-view/src/view/menu_tree.rs (new, egui-free; name is a placeholder)"],
    """Group file: research/spec-coverage-20261111-g7-shell-manual-misc.json. Rows: STU-SHL-010..044 (menu: 7 MISSING 013, 014, 039, 041-044; 23 PARTIAL), STU-SHL-046..055 (availability predicate: MISSING 048, 049, 054, 055), STU-SHL-230..236 (tooltip system: MISSING 230, 231, 233, 235, 236), STU-SHL-240..247 (UiDescriptor: MISSING 242, 245-247). Design: gui-design/build-order.json step B6 'Descriptors and menu', done_when 'All 19 titles open, greyed items show a reason, and a remedy runs'; B6 components menu_popup, menu_item, tooltip_l0, tooltip_l1, context_menu (build-order.json; shapes in gui-design/components.json). Ledger candidates: 2026-11-07T17:49:57Z (menus), 2026-11-07T13:35:06Z (availability fields in the semantic tree), 2026-11-07T12:31:03Z (widget ids not seeded in the UiCatalog), 2026-11-07T17:49:57Z (Viewer::run_command does not accept layer.set_visible).

Facts:
- native-host draws the 19 titles from `gui/preset.rs:13` (`MENU: [&str; 19]`) in `gui/chrome.rs:61-76`; every title is static: `chrome.rs:19` NIY = "not_implemented_yet", and a press on a disabled control answers it (`gui/mod.rs:326`).
- The menu model exists egui-free in `hsk-studio-controls/src/menu.rs`: `MenuBar::new` :367, `walk` :372, `find` :389, `resolve_remedies` :397, `audit` :432, `agent_json` :483, `manual_topics` :502; `Availability` :155 with `inapplicable(code, text, remedy_command_id)` :169 and `reason_line` :193. `menu_bar_ui` at :638 is egui: do not call it.
- The only populated MenuBar is the egui test shell's 7 titles (`harness-view/src/view/shell.rs:1071`): read it, do not edit it.
- `harness-core/src/action/ui_seed.rs:19` `ui_seed()` returns the UiCatalog seeded by SMB-COMMAND-SURFACE-1; RUN_COMMANDS is the typed command table.
- kumihimo-ui has `MenuTitle` (`widgets.rs:287`) and `menu_title_row` (:357); no popup, menu item or tooltip widget exists (git grep).
Depends on: SMB-GUI-FRAME-1 and SMB-VK-UI-PASS-1 (READY, on origin, not yet judged). One GUI line of work before B9 (build-order `parallel_from`): no other GUI lane runs beside this one. Host binary runs take the GPU lock; run Vulkan tests with VK_INSTANCE_LAYERS=VK_LAYER_KHRONOS_validation and go READY only validation-clean (direction.lead_rulings_20261108.vk_ui_pass_1).""",
    ["research/spec-coverage-20261111-g7-shell-manual-misc.json STU-SHL menu, availability, tooltip, UiDescriptor rows", "gui-design/build-order.json B6",
     "ledger direction.next_lane_candidates 2026-11-07T17:49:57Z menus"],
    [], "smb-pr-entry-references-1",
    "native-host [document] tests and kumihimo-ui suite; harness-view [viewer] suite only if you edit its src",
    exposure="high",
    diag="producer_through_observe: menu open, item run and refusal outcomes as codes and counters in the receipt's gui.frame member; commands keep their own Run observe frames",
    out_of_scope="ScrubValue and core controls (B7), Tool Rail (B8), dock host (B9), panels (B10+), canvas overlays; no egui, eframe, wgpu, naga or WGSL line; no edit to the egui shell.",
))

lanes.append(scope(
    "SMB-TYPE-CHARACTER-ATTRIBUTES-1",
    "Character attribute model and paragraph structure: per-range size, tracking, kerning, baseline shift and OpenType feature tags passed into Type; script and language itemization instead of Latn/en; replace_text newline and empty-story forms",
    ["hsk-studio-folio (typestyle.rs stored fields)", "hsk-studio-layout (textflow.rs)", "hsk-studio-type",
     "hsk-studio-harness-view (layout_ui.rs, view/commands/layout.rs, view/layout_export.rs)", "hsk-studio-harness-core (action/commands/layout.rs)"],
    ["hsk-studio-folio/src/typestyle.rs", "hsk-studio-layout/src/textflow.rs", "hsk-studio-harness-view/src/view/layout_export.rs", "hsk-studio-type/src (itemization)",
     "hsk-studio-harness-view/src/layout_ui.rs", "hsk-studio-harness-view/src/view/commands/layout.rs", "hsk-studio-harness-core/src/action/commands/layout.rs"],
    """Group file: research/spec-coverage-20261111-g3-vector-type.json. Rows: 14.7.9 Character Attributes STU-TYP-155..161 and 192 (MISSING 156, 158-161, 192; PARTIAL 155, 157), 14.7.17 language record STU-TYP-209 (MISSING) for script and language itemization, 14.7.5 shaping STU-TYP-127..129, 133. Origin: research/width-plan-20261108-print-web.json rank 3, start gate 'after SMB-TYPE-INDENT-SPACING-1 merges' (met: 153a410d is on origin). Ledger candidates: 2026-11-07T12:31:03Z (layout.replace_text with a newline keeps one paragraph), 2026-11-07T17:33:46Z (empty story stored as one zero-length paragraph), 2026-11-07T16:35:52Z (everything shapes as Latin, English).

Facts:
- STU-TYP-155 evidence (audit): size, line height and letter spacing at `folio/src/typestyle.rs:116-122`; baseline_shift, horizontal_scale and tsume: 0 hits.
- planner READ at 0a733db8: Type already accepts features and script per style; `layout_export.rs:302-306` passes them empty (the file is `harness-view/src/view/layout_export.rs` at b57c82d1; re-read the lines).
- This tree's previous lane (SMB-TYPE-INDENT-SPACING-1, 153a410d) edited the same files: read its lane file notes for the paths it ruled.
Rule 4b: a new Folio record field also runs the hsk-studio-package suite.""",
    ["research/width-plan-20261108-print-web.json rank 3", "research/spec-coverage-20261111-g3-vector-type.json STU-TYP 14.7.9 rows"],
    [], "smb-pr-retime-hold-1",
    "folio, layout, type and package suites; harness-core and harness-view [viewer] suites; harness [viewer] layout targets",
))

lanes.append(scope(
    "SMB-BOARD-1",
    "Whiteboard mode first slice: document.new board profile, board container, sticky, shape_with_text, section and connector (STRAIGHT, ELBOWED; node and magnet endpoints) with factory, read and mutate commands and PNG export",
    ["hsk-studio-folio (board node records)", "hsk-studio-composite (drawing over existing vector and text paths)", "hsk-studio-harness-core (action/commands/board.rs)",
     "hsk-studio-harness-view (new egui-free board module)"],
    ["hsk-studio-folio/src (board node records)", "hsk-studio-composite/src (board drawing)", "hsk-studio-harness-core/src/action/commands/board.rs",
     "hsk-studio-harness-view/src/view/board.rs (new, egui-free; placeholder name)", "hsk-studio-harness/tests/board.rs (new)"],
    """Group file: research/spec-coverage-20261111-g5-web-whiteboard.json. Rows: STU-WB-001, 002, 102..117, 121, 134 (board mode and node kinds, 13 of 14 MISSING; command-surface obligation STU-WB-134 MISSING). Origin: research/width-plan-20261108-print-web.json rank 4, start gate 'after SMB-AI-LIVE-EFFECTS-1 merges' (met: 1e652818 is on origin). Scenario: workflows/scenario-board-slides-web.json steps 1, 3, 6, 8-13, 26.

Facts:
- `harness-core/src/action/commands/board.rs` exists with an empty `ROWS` table (5 lines): this lane owns it.
- No board code: grep StudioBoard|board_node|BoardNode|sticky_note|shape_with_text: 0 (audit).
- STU-WB-102 says board node kinds are StudioLayer kinds. `folio/src/lib.rs:268` is `pub enum Payload`; `Payload::Group` appears in 41 files. Before coding, list every exhaustive Payload match your design would touch and send it to the lead: if it adds Payload variants, this lane lands last in the wave.
- Open spec question (workflows/backlog-20261107.json spec_questions): a board has no signature, default layout, Layout Preset or tool family (OD-4 at STU-SHL-136). It blocks the board GUI only; this slice is engine and commands.
STU-WB-101 calls whiteboarding a lower-priority parity domain; keep the slice to the node kinds above. Rule 4b: new Folio records also run the hsk-studio-package suite.""",
    ["research/width-plan-20261108-print-web.json rank 4", "research/spec-coverage-20261111-g5-web-whiteboard.json STU-WB board rows"],
    [], "smb-sentinels-native-1",
    "folio, composite and package suites; harness-core and harness-view [viewer] suites; harness board tests",
    exposure="high",
))

lanes.append(scope(
    "SMB-WEB-CODE-1",
    "Web code intelligence, first slice: document-type records, tag libraries and the attribute vocabulary, the CSS property surface, code hints, markup validation and formatting as typed web commands",
    ["hsk-studio-web-core (new modules)", "hsk-studio-harness-core (new command file and handlers)"],
    ["hsk-studio-web-core/src (new modules: doc types, vocabulary, css properties, hints, validation, formatting)",
     "hsk-studio-harness-core/src/action/commands/web_code.rs (new; one DOMAINS line in commands.rs)", "hsk-studio-harness-core/src/web_code.rs (new handlers)",
     "hsk-studio-web-core/tests (new)"],
    """Group file: research/spec-coverage-20261111-g5-web-whiteboard.json. Rows: 14.28.1 STU-WEB-011..029, 047, 052, 090 (17 MISSING, 4 PARTIAL), 14.28.3 code intelligence STU-WEB-046, 048..059 (12 MISSING), 14.28.4 CSS property surface STU-WEB-060..076 (15 MISSING, PARTIAL 060, 076). Web is the largest MISSING area of the audit (119 rows).

Facts:
- web-core holds `css.rs`, `responsive.rs` (`Breakpoint` min/max width px at :20, scan_media_blocks), `dom`, `preview.rs`, `report.rs`; no document-type record (grep internal_kind|dtd_context: 0), no colouring scheme (grep scheme_id|keyword_lists: 0), no code navigation (grep balance_braces|select_parent_tag: 0) (audit evidence).
- Existing web rows: `harness-core/src/action/commands/web.rs` (77 lines; `web.media` row at :73) and handlers in `harness-core/src/web_commands.rs` (`"web.media"` arm at :390). SMB-WEB-SITE-2 edits commands/web.rs and web_site.rs in parallel: put your rows in a new domain file and your handlers in a new file.
- Vocabulary data: write it from the spec's clauses; any external table needs a permissive licence recorded in `builders/dep-requests.jsonl` before pushing; no copyleft source. Dreamweaver behaviour not in the spec is UNVERIFIED (D5 oracle).
Open spec question (backlog spec_questions): a layout and paint engine for the web subset has no named engine; live rendering is out of this slice.""",
    ["research/spec-coverage-20261111-g5-web-whiteboard.json STU-WEB 14.28.1, 14.28.3, 14.28.4 rows"],
    [], "smb-r53-followups-1",
    "web-core suite; harness-core suite; harness [viewer] web targets",
))

lanes.append(scope(
    "SMB-LAYOUT-DELETE-CASCADE-1",
    "Layout deletes that InDesign performs: page.delete with content and a text-frame layer delete re-link the thread in one undo step; groups, pages and spreads holding frame layers; autoflow placement modes and smart text reflow",
    ["hsk-studio-harness-core (harness/containers.rs)", "hsk-studio-harness-view (view/layers.rs, layout_place.rs, layout_ui/threads.rs)",
     "hsk-studio-layout (threaded_reflow.rs)"],
    ["hsk-studio-harness-core/src/harness/containers.rs", "hsk-studio-harness-view/src/view/layers.rs", "hsk-studio-harness-view/src/view/layout_place.rs",
     "hsk-studio-harness-view/src/layout_ui/threads.rs", "hsk-studio-layout/src/threaded_reflow.rs",
     "hsk-studio-harness-core/src/action/commands/layout_flow.rs (new; one DOMAINS line in commands.rs)"],
    """Group file: research/spec-coverage-20261111-g4-layout-io-docs.json. Rows: STU-LAY-020 autoflow placement modes (MISSING, spec :5851), STU-LAY-123 autoflow and smart text reflow (MISSING, :5855); page and spread rows STU-LAY-001, 003, 107 (PARTIAL). Ledger candidates (2026-11-12T17:43:00Z, 2026-11-11T10:39:51Z, 2026-11-11T12:51:10Z): page.delete with content; text-frame layer delete with re-linking; deleting a group, page or spread that holds a frame layer; asset.place with frame_to_content keeps the given bounds.

Facts:
- page.delete refuses `page_not_empty` for content (`harness-core/src/harness/containers.rs:54-58`, :513-522 docs, :590 graphic frame case): replace it with the cascade in one undo step.
- delete_layer refuses `layer_bound_by_layout_records` (`harness-view/src/view/layers.rs:219`), added by SMB-R60-FOLLOWUPS-1 (this tree's previous lane, e29d933a): replace it with thread re-linking; undo restores; reopen editable.
- Threads: `folio/src/layout.rs:392` text_threads; rows layout.thread / layout.unthread at `harness-core/src/action/commands/layout.rs:14-15`; the Viewer maps them at `harness-view/src/view/commands/layout.rs:67-68`; thread code in `harness-view/src/layout_ui/threads.rs`.
- SMB-TYPE-CHARACTER-ATTRIBUTES-1 runs in parallel and owns `layout_ui.rs`, `view/commands/layout.rs` and `action/commands/layout.rs`: do not edit them. New rows go in a new domain file. If the re-link needs a new LayoutEdit variant (layout_ui.rs), tell the lead and land after that lane.""",
    ["ledger direction.next_lane_candidates 2026-11-12T17:43:00Z page.delete with content and text-frame layer delete",
     "ledger direction.next_lane_candidates 2026-11-11T10:39:51Z layer.delete of a threaded text-frame layer",
     "ledger direction.next_lane_candidates 2026-11-11T12:51:10Z frame_to_content bounds and group/page/spread delete",
     "research/spec-coverage-20261111-g4-layout-io-docs.json STU-LAY-020, STU-LAY-123"],
    [], "smb-pr-edit-footprint-1",
    "harness-core and harness-view [viewer] suites; layout suite; harness [viewer] layout and layers targets",
))

lanes.append(scope(
    "SMB-DS-VARIABLES-AUTHOR-1",
    "Design variables as typed commands (collections, modes, variables, aliases, scopes, code syntax, explicit modes; resolve and validate as reads) plus history-aware component text compaction and .fig design-state prune at save",
    ["hsk-studio-harness-view (vector entry design member, view/design/*, view/design/persist.rs)", "hsk-studio-harness-core (design command rows)",
     "hsk-studio-components (read)"],
    ["hsk-studio-harness-view/src/view/vector/style.rs (the member check near :291-300)", "hsk-studio-harness-view/src/view/design/component.rs and instance.rs (sync compaction)", "hsk-studio-harness-view/src/view/design/variables.rs (new; placeholder name)",
     "hsk-studio-harness-view/src/view/design/persist.rs (prune at save; design_opened is at :315)", "hsk-studio-harness-core/src/action/commands/design.rs", "hsk-studio-harness/tests (new)"],
    """Group file: research/spec-coverage-20261111-g8-designsystems-automation-assets.json. Rows: STU-DS-123..133 (MISSING 131, 133; PARTIAL 123, 124, 126-128, 130, 132), STU-DS-024..034 styles versus variables (PARTIAL). Origin: research/width-plan-20261108-raster-vector-design.json rank 3, depends on SMB-DS-COMPONENTS-AUTHOR-1 (met: b57c82d1 is origin). Ledger candidates 2026-11-12T17:58:12Z: history-aware text-record compaction for component syncs; prune superseded .fig instance design states at save time, keeping reopen byte-identical.

Facts:
- planner READ at 0a733db8: StudioVariableCollection `folio/src/components.rs:246`, StudioVariable :265, explicit_variable_modes `folio/src/records.rs:83`; nothing writes them (git grep variable_collection|create_variable|set_code_syntax over the harness crates: 0); the vector entry refuses them (`harness-view/src/view/vector/style.rs:297-300`, changed since by SMB-DS-COMPONENTS-AUTHOR-1: re-read); `components::resolve_variable` (`components/src/lib.rs:36`) has no host caller. Command names from STU-DS-133 (spec :20068).
- builders/lanes/hsk-studio-ds-components-author-1.json (this tree's previous lane): not_done lists 'in a session, about 127 syncs of a component with one text layer and two instances reach text_layer_over_cap; a save and reopen compacts (R1)' and '.fig instance states with superseded contents are not pruned on open'; lead_rulings_20261112 R1 (no text compaction an undo can reach) and R2 (design file states kept as saved because viewer_fig_edit_persist of lane SMB-FIG-EDIT-2 pins a byte-identical entry on reopen; cap MAX_DESIGN_STATES 1024). Compaction must never drop a record a retained undo step can reach; prune at save must keep save-then-reopen byte-identical.
Bindings of variables to paints and node fields are out (no Folio home); tokens export follows them.""",
    ["research/width-plan-20261108-raster-vector-design.json rank 3", "research/spec-coverage-20261111-g8-designsystems-automation-assets.json STU-DS variables rows",
     "ledger direction.next_lane_candidates 2026-11-12T17:58:12Z text-record compaction", "ledger direction.next_lane_candidates 2026-11-12T17:58:12Z .fig prune at save"],
    [], "smb-ai-distort-effects-1",
    "harness-core and harness-view [viewer] suites; harness [viewer] design, components and fig edit persist targets",
))

lanes.append(scope(
    "SMB-PS-FILTER-ADJUST-COMMANDS-1",
    "Filter and adjustment commands: effect.* over the 25 smart filter kinds with parameter validation at the command, adjustment-layer create, edit and apply in place, plus the crop keys in the develop DNG XMP",
    ["hsk-studio-harness-view (place.rs entry-edit helper, effect and adjustment commands, develop.rs XMP crop)", "hsk-studio-harness-core (photo or effect rows)",
     "hsk-studio-pigment (read)", "hsk-studio-folio (read)", "hsk-studio-interop-lrcat (read, if the edge is approved)"],
    ["hsk-studio-harness-view/src/view/place.rs (an entry-edit helper beside editable_entry)", "hsk-studio-harness-view/src/view/raster_effects.rs (new, egui-free; placeholder name)",
     "hsk-studio-harness-view/src/view/develop.rs (XMP crop only)", "hsk-studio-harness-core/src/action/commands/photo.rs",
     "hsk-studio-harness-view/Cargo.toml (only if the lead approves the interop-lrcat edge)", "hsk-studio-harness/tests (new)"],
    """Group files: research/spec-coverage-20261111-g2-raster-effects-controls.json and -g1-colour-raw-compositing.json. Rows: STU-FX-007 (PARTIAL), STU-RAS-035, 151, 153 (PARTIAL), STU-RAS-103..112 parameter contract (MISSING 107, 111), STU-RAW-015 (PARTIAL, crop). Origin: research/width-plan-20261108-raster-vector-design.json rank 4 (filter-commands slice, AC-005 of SMB-PIGMENT-FILTER-KERNELS-1). Ledger candidates: 2026-11-07T13:29:59Z 'Filter commands (the operator surface, AC-005) for the nineteen kernels'; 2026-11-12T10:30:18Z crop in the DNG XMP (receipt code develop_crop_not_in_dng_xmp).

Facts:
- planner READ at 0a733db8: the session entry raster_records (`harness-view/src/view/place.rs:92`, schema :96) holds Folio StudioRasterRecords; embed_inner commits an entry-only step (place.rs:1044); StudioSmartFilterKind has 25 kinds (`folio/src/smart_filter.rs:88-115`, cap 16 at :22); validate through pigment `SmartFilterOp::from_record` (`pigment/src/smart_filter.rs:313`) and `FilterKernel::validate` (`pigment/src/filter_kernels.rs:177`) so EFFECT_PARAM_OUT_OF_RANGE (STU-FX-104) refuses at the command; StudioAdjustment `folio/src/raster.rs:150`; restructure_and_entry `harness-core/src/harness.rs:1488` (at b57c82d1: :1494).
- READ at b57c82d1: `harness-view/src/view/develop.rs:1214-1215` set_develop_dng_xmp writes the caller's packet and says the lrcat XMP codec owns the crs: keys (also :1516); `interop-lrcat/src/xmp.rs` is that codec; harness-view depends on hsk-studio-develop (optional, Cargo.toml:11, :42) and not on interop-lrcat. The crop keys need a harness-view -> interop-lrcat edge (steward edge, Cargo.lock line, lead approval) or a second crs: writer in develop. Ask the lead before either.
Per-filter masks, selection-scoped apply and destructive filter forms are the next slice.""",
    ["research/width-plan-20261108-raster-vector-design.json rank 4", "ledger direction.next_lane_candidates 2026-11-07T13:29:59Z filter commands",
     "ledger direction.next_lane_candidates 2026-11-12T10:30:18Z crop in the DNG XMP"],
    [], "smb-folio-angle-unbounded-1",
    "harness-core and harness-view [viewer] suites; harness [viewer] raster and develop targets; develop suite only if you edit it",
))

lanes.append(scope(
    "SMB-WEB-SITE-2",
    "Site depth after SMB-WEB-SITE-1: templates with editable, locked and repeating regions and update propagation, library items, snippets, change-link-site-wide, and a session Action::Read for read-only verbs",
    ["hsk-studio-harness-core (web_site.rs, action/commands/web.rs)", "hsk-studio-session (ports.rs Action)", "hsk-studio-kernel-bindings (wire.rs mapping)"],
    ["hsk-studio-harness-core/src/web_site.rs", "hsk-studio-harness-core/src/action/commands/web.rs", "hsk-studio-session/src/ports.rs",
     "hsk-studio-kernel-bindings/src/wire.rs", "hsk-studio-harness/tests (new web site targets)"],
    """Group file: research/spec-coverage-20261111-g5-web-whiteboard.json. Rows: 14.28.6 STU-WEB-091..093, 096..099 (7 MISSING), site record and synchronisation STU-WEB-113, 119 (PARTIAL). Ledger candidates: 2026-11-12T15:08:00Z session Action::Read for read-only verbs (web.site_report and similar) with the kernel-bindings mapping; 2026-11-12T15:08:00Z and 2026-11-11T11:09:18Z change-link-site-wide for case-mismatched and percent-encoded spellings; 2026-11-11T10:39:51Z diag_sink abort flake (act only if it repeats in R60 attempt 3 or in your bundled run).

Facts:
- `session/src/ports.rs:12-17` `enum Action { Edit, Save, Export }` with `as_str`; `kernel-bindings/src/wire.rs:128-130` maps each variant; `harness-core/src/web_site.rs:124` `REPORT_ACTION: Action = Action::Export` (lead Q1 reading at SMB-WEB-SITE-1). Action::Export or Save is named in 15 files (31 hits): list the exhaustive matches a new variant touches before coding.
- No template, library item or snippet record: grep repeating_table|library_item|locked_region: 0; snippet_trigger_duplicate: 0 (audit).
- SMB-WEB-CODE-1 runs in parallel in web-core and new harness-core files; this lane owns web_site.rs and commands/web.rs.
- diag_sink test: `harness-core/tests/diag_sink.rs:568` an_abort_leaves_the_lines_written_and_no_end_of_run; module `harness-core/src/diag_sink.rs`.""",
    ["research/spec-coverage-20261111-g5-web-whiteboard.json STU-WEB 14.28.6 rows", "ledger direction.next_lane_candidates 2026-11-12T15:08:00Z Action::Read",
     "ledger direction.next_lane_candidates 2026-11-12T15:08:00Z change-link-site-wide", "ledger direction.next_lane_candidates 2026-11-11T11:09:18Z change-link-site-wide"],
    [], "smb-pr-sequence-session-1",
    "session and kernel-bindings suites; harness-core suite; harness [viewer] web targets",
))

os.makedirs(OUT, exist_ok=True)
for slug, s in lanes:
    with open(os.path.join(OUT, f"scope-20261107-{slug}.json"), "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(s, indent=1, ensure_ascii=False) + "\n")
    print(slug, s["tree"], len(s["target"]))
args = {
    "schema": "scratch.wave_next_args@1",
    "note": "Draft only. wave_from_scope.py reads S/research/scope-20261107-<area>.json; copy the scope files there first. Nothing here was run.",
    "base": BASE,
    "hold": "R60 attempt 3 takes the D: lock first. Until it releases, read the code, write red tests and commit locally; your first hold queues behind the round. Run every lock-needing step of a phase inside one outer hold (wave-parity-2-common ONE HOLD PER PHASE). Lanes land one at a time on the lead's release (HANDOFF workflow item 16).",
    "areas": [slug for slug, _ in lanes],
    "command": "python tools/lead/wave_from_scope.py --base " + BASE + " --hold \"<hold>\" " + " ".join(slug for slug, _ in lanes),
}
with open(os.path.join(OUT, "wave-args.json"), "w", encoding="utf-8", newline="\n") as f:
    f.write(json.dumps(args, indent=1, ensure_ascii=False) + "\n")
