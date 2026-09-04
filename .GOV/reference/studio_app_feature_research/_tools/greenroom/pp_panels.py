"""pp_panels.py -- panels, dialogs, workspaces and monitor overlays, offline.

Streams:
  P1  install/xml/*.xml   Adobe prop.map v4 archives. Three distinct kinds ship
      in this one directory and they are separated by their own content, not by
      filename:
        * UINodeArchive -> a serialized dvaui control tree (a dialog or an
          embedded panel section). Walked into a control tree: every node's
          adapter class, id, label text, tooltip, bounds, enabled/visible state
          and control-specific values.
        * DVA_Wrkspce   -> a workspace layout: monitors, top-level frames,
          splitters and the panel ids docked in each pane.
        * everything else (XMP schema definitions, colour themes, cursors)
          which is classified and inventoried rather than walked as UI.
  P2  install/eve/*.eve and Settings/EveScripts/*.eve (non-menu)  Adobe Eve
      dialog layouts parsed into control trees.
  P3  install/Monitor Overlays/*.olp   safe-area and overlay definitions.
  P4  install/table/*.table            the UI colour tables.
  P5  executable string table          panel and dialog string namespaces.
"""
import collections
import os
import re
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C
import dw_eve

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

# dvaui adapter class -> (control role, value kind). Adapter names are read
# verbatim out of the archives; the role mapping is a plain naming read of the
# class name and is labelled as derived.
ADAPTER_ROLE = {
    "dvaui::ui::UI_SubView": ("layout container", "container"),
    "dvaui::ui::UI_Node": ("node", "container"),
    "dvaui::ui::UI_ScrollView": ("scroll view", "container"),
    "dvaui::controls::UI_StaticText": ("label", "read-only text"),
    "dvaui::controls::UI_TextEdit": ("text field", "string"),
    "dvaui::controls::UI_TextEdit_S": ("single-line text field", "string"),
    "dvaui::controls::UI_TextEdit_M": ("multi-line text field", "string"),
    "dvaui::controls::UI_CheckBox": ("checkbox", "boolean"),
    "dvaui::controls::UI_CheckBoxWithText": ("checkbox with label", "boolean"),
    "dvaui::controls::UI_RadioButton": ("radio button", "enum member"),
    "dvaui::controls::UI_PushButton": ("push button", "action"),
    "dvaui::controls::UI_PushButtonWithText": ("push button", "action"),
    "dvaui::controls::UI_IconButton": ("icon button", "action"),
    "dvaui::controls::UI_ToggleButton": ("toggle button", "boolean"),
    "dvaui::controls::UI_PopupButton": ("dropdown", "enum"),
    "dvaui::controls::UI_ComboBox": ("editable dropdown", "enum or free text"),
    "dvaui::controls::UI_DropDownList": ("dropdown", "enum"),
    "dvaui::controls::UI_ListBox": ("list box", "enum"),
    "dvaui::controls::UI_Slider": ("slider", "number"),
    "dvaui::controls::UI_SliderControl": ("slider", "number"),
    "dvaui::controls::UI_ScrubbyNumber": ("scrubbable number field", "number"),
    "dvaui::controls::UI_OutlineBoxWithLabel": ("group box", "container"),
    "dvaui::controls::UI_GroupBox": ("group box", "container"),
    "dvaui::controls::UI_TabGroup": ("tab group", "container"),
    "dvaui::controls::UI_TabView": ("tab view", "container"),
    "dvaui::controls::UI_ProgressBar": ("progress bar", "read-only number"),
    "dvaui::controls::UI_Separator": ("separator", "none"),
    "dvaui::controls::UI_Image": ("image", "read-only"),
    "dvaui::controls::UI_ControlView": ("control base", "container"),
    "dvaui::controls::UI_InteractiveControlView": ("interactive control base", "container"),
    "dvaui::controls::UI_TreeView": ("tree view", "collection"),
    "dvaui::controls::UI_ColorSwatch": ("colour swatch", "colour"),
    "dvaui::controls::UI_HyperlinkText": ("hyperlink", "action"),
}

VALUE_KEYS = ("text", "id", "tooltip", "enabled", "visible", "multiLine",
              "fontSize", "selected", "value", "minimum", "maximum",
              "increment", "checked", "readOnly", "password", "items",
              "placeholder", "alignment", "textTruncation", "iconName",
              "commandID", "wantsFocus", "suppressFocusDrawing")


def _flatten_sections(node):
    """Merge every section-N of one node into a single property dict."""
    props = {}
    secs = node.get("sections")
    if not isinstance(secs, dict):
        return props, []
    kids = []
    for key in sorted(secs):
        if not key.startswith("section-"):
            continue
        sec = secs[key]
        if not isinstance(sec, dict):
            continue
        for k, v in sec.items():
            if k == "children":
                if isinstance(v, dict):
                    for ck in sorted(v, key=lambda s: (len(s), s)):
                        if isinstance(v[ck], dict):
                            kids.append(v[ck])
                continue
            if k in ("sectionName", "Constraints", "LayoutChildren"):
                continue
            if k not in props:
                props[k] = v
    return props, kids


def walk_ui_tree(node, path=(), depth=0, out=None):
    if out is None:
        out = []
    if not isinstance(node, dict) or depth > 60:
        return out
    adapter = node.get("adapter")
    props, kids = _flatten_sections(node)
    role, vkind = ADAPTER_ROLE.get(adapter, (None, None))
    rec = {
        "adapter": adapter,
        "control_role": role,
        "control_role_confidence": "derived from the dvaui class name",
        "value_kind": vkind,
        "depth": depth,
        "container_path": list(path),
    }
    for k in VALUE_KEYS:
        if k in props:
            v = props[k]
            if isinstance(v, dict) and ".aptype" in v:
                v = {kk: vv for kk, vv in v.items() if kk != ".aptype"}
            rec[k] = v
    b = props.get("bounds")
    if isinstance(b, dict):
        rec["bounds"] = {k: b[k] for k in ("left", "top", "width", "height")
                         if k in b}
    if rec.get("text"):
        key, txt = C.split_localized(rec["text"])
        if key:
            rec["text"] = txt
            rec["text_string_key"] = key
    rec = {k: v for k, v in rec.items() if v not in (None, "", [], {})}
    out.append(rec)
    nxt = path + ((rec.get("id") or (adapter or "?").split("::")[-1]),)
    for kid in kids:
        walk_ui_tree(kid, nxt, depth + 1, out)
    return out


def walk_workspace(tree):
    """DVA_Wrkspce archive -> frame tree, splitters and the panels docked in each.

    Layout shape, read off the shipped archives:
      TopLevelFrame-N { HasToolBar HasStatBar HasLeftBar HasRightBar
                        Splitter { Orient Place Sub1 {...} Sub2 {...} } }
      a leaf pane is  Frame { TabIDs [panel ids] CurrTab HiddenTabs
                              RemovedTabs Vis StackedViewFlags }
    Orient is a boolean: the shipped files carry false for one axis and true for
    the other; which is horizontal is not stated in the file, so it is reported
    raw rather than named. Place is the split position as a 0..1 fraction.
    """
    frames = []
    panes = []
    ids = set()

    def as_list(v):
        if isinstance(v, list):
            return [x for x in v if isinstance(x, str) and x]
        if isinstance(v, str) and v:
            return [v]
        return []

    def descend(node, path=()):
        if not isinstance(node, dict):
            return
        fr = node.get("Frame")
        if isinstance(fr, dict):
            tabs = as_list(fr.get("TabIDs"))
            hidden = as_list(fr.get("HiddenTabs"))
            removed = as_list(fr.get("RemovedTabs"))
            ids.update(tabs)
            ids.update(hidden)
            panes.append({
                "pane_path": list(path),
                "visible_tabs": tabs,
                "current_tab": fr.get("CurrTab"),
                "hidden_tabs": hidden,
                "removed_tabs": removed,
                "visible": fr.get("Vis"),
                "stacked_view_flags": fr.get("StackedViewFlags"),
                "tab_count": len(tabs),
            })
        sp = node.get("Splitter")
        if isinstance(sp, dict):
            for side in ("Sub1", "Sub2"):
                sub = sp.get(side)
                if isinstance(sub, dict):
                    descend(sub, path + ("%s(orient=%s,place=%s)/%s" % (
                        "Splitter", sp.get("Orient"), sp.get("Place"), side),))
        for k, v in node.items():
            if k in ("Frame", "Splitter"):
                continue
            if isinstance(v, dict):
                descend(v, path + (k,))

    for k, v in tree.items():
        if k.startswith("TopLevelFrame") and isinstance(v, dict):
            frames.append({
                "frame": k,
                "has_tool_bar": v.get("HasToolBar"),
                "has_status_bar": v.get("HasStatBar"),
                "has_left_bar": v.get("HasLeftBar"),
                "has_right_bar": v.get("HasRightBar"),
            })
            descend(v, (k,))

    # a couple of shipped layouts also carry panel ids outside a Frame
    def sweep(n):
        if isinstance(n, dict):
            for k, v in n.items():
                if k in ("TabIDs", "HiddenTabs", "RemovedTabs", "CurrTab"):
                    ids.update(as_list(v))
                sweep(v)
        elif isinstance(n, list):
            for x in n:
                sweep(x)

    sweep(tree)
    ids = {i for i in ids if i and not i.startswith("<prop.list")}
    return {"top_level_frames": frames, "panes": panes,
            "pane_count": len(panes),
            "panel_identifiers": sorted(ids)}


def classify_xml(path):
    with open(path, "rb") as fh:
        head = fh.read(1400)
    if b"<prop.map" not in head:
        if b"<AppSet" in head:
            return "eucon_control_surface"
        return "not_a_prop_map"
    if b"UINodeArchive" in head:
        return "ui_node_archive"
    if b"DVA_Wrkspce" in head:
        return "workspace_layout"
    if b"CursorData" in head or os.path.basename(path).startswith("CursorDataID"):
        return "cursor_definition"
    if b"ColorTheme" in head or "ColorTheme" in os.path.basename(path):
        return "colour_theme"
    return "prop_map_other"


def classify_xml_full(path):
    kind = classify_xml(path)
    name = os.path.basename(path)
    if kind == "not_a_prop_map":
        with open(path, "rb") as fh:
            head = fh.read(1400)
        if name.endswith("_definitions.xml") or b"MetadataSchema" in head \
                or b"<xmp" in head or b"namespace" in head.lower():
            return "xmp_schema_definition"
        if name.endswith("_view.xml"):
            return "metadata_view_definition"
        if b"<PremiereData" in head:
            return "premiere_data_document"
        return "other_xml"
    if kind != "prop_map_other":
        return kind
    if name.endswith("_definitions.xml"):
        return "xmp_schema_definition"
    if name.endswith("_view.xml"):
        return "metadata_view_definition"
    return "prop_map_other"


def main(out_dir):
    R = C.PREMIERE_ROOT
    table = C.premiere_strings(SCRATCH)
    sources = []
    failures = []

    # ---- P1 install/xml
    xml_dir = os.path.join(R, "xml")
    dialogs = []
    workspaces = []
    others = collections.Counter()
    other_files = []
    for p in sorted(C.walk_files(xml_dir, exts=(".xml",))):
        name = os.path.basename(p)
        if C.looks_ai(name):
            continue
        kind = classify_xml_full(p)
        if kind == "ui_node_archive":
            try:
                tree = C.parse_propmap(p)
                controls = walk_ui_tree(tree.get("root") or tree)
            except Exception as exc:                   # noqa: BLE001
                failures.append({"stage": "P1_ui", "path": C.rel(p),
                                 "error": repr(exc),
                                 "traceback": traceback.format_exc()})
                continue
            interactive = [c for c in controls
                           if c.get("control_role") not in
                           (None, "layout container", "node", "control base",
                            "interactive control base")]
            dialogs.append({
                "surface": os.path.splitext(name)[0],
                "file": C.rel(p),
                "variant": ("V7 theme variant" if ".V7." in name else "base"),
                "control_count": len(controls),
                "interactive_control_count": len(interactive),
                "max_depth": max((c["depth"] for c in controls), default=0),
                "adapters_used": sorted({c["adapter"] for c in controls
                                         if c.get("adapter")}),
                "controls": controls,
            })
        elif kind == "workspace_layout":
            try:
                tree = C.parse_propmap(p)
                ws = walk_workspace(tree)
            except Exception as exc:                   # noqa: BLE001
                failures.append({"stage": "P1_workspace", "path": C.rel(p),
                                 "error": repr(exc)})
                continue
            ws["workspace"] = os.path.splitext(name)[0]
            ws["file"] = C.rel(p)
            ws["monitor_count"] = ((tree.get("MonitorInfo") or {})
                                   .get("NumMonitors"))
            ws["serialization"] = tree.get("DVA_Wrkspce")
            ws["panel_count"] = len(ws["panel_identifiers"])
            workspaces.append(ws)
        else:
            others[kind] += 1
            other_files.append({"file": C.rel(p), "kind": kind})

    sources.append({
        "id": "P1_prop_map",
        "path": C.rel(xml_dir),
        "how": ("every .xml classified by its own content, then UINodeArchive "
                "files walked into control trees and DVA_Wrkspce files walked "
                "into workspace layouts"),
        "ui_node_archives": len(dialogs),
        "workspace_layouts": len(workspaces),
        "other_kinds": dict(others),
    })

    # ---- P2 Eve dialogs
    eve_surfaces = []
    for root_dir in (os.path.join(R, "eve"),
                     os.path.join(R, "EveScripts"),
                     os.path.join(R, "Settings", "EveScripts")):
        if not os.path.isdir(root_dir):
            continue
        for p in sorted(C.walk_files(root_dir, exts=(".eve",))):
            rel = C.rel(p)
            if "/Menus/" in rel or "/NewMenus/" in rel:
                continue          # menus belong to premiere_commands_shortcuts
            if C.looks_ai(os.path.basename(p)):
                continue
            try:
                with open(p, "r", encoding="utf-8", errors="replace") as fh:
                    layouts = dw_eve.parse_eve(fh.read())
            except Exception as exc:                   # noqa: BLE001
                failures.append({"stage": "P2_eve", "path": rel,
                                 "error": repr(exc)})
                continue
            controls = []
            for lay in layouts:
                for ctl in dw_eve.flatten_controls(lay["nodes"]):
                    row = {k: ctl[k] for k in
                           ("kind", "control_role", "value_kind", "identifier",
                            "label", "label_string_key", "container_path",
                            "is_container")
                           if ctl.get(k) not in (None, "", [])}
                    for k in ("min_value", "max_value", "value", "default",
                              "increment", "characters", "digits", "precision",
                              "items", "readonly", "multiselect",
                              "num_visible_items"):
                        if k in ctl:
                            row[k] = ctl[k]
                    controls.append(row)
            interactive = [c for c in controls if not c.get("is_container")
                           and c.get("kind") not in ("static_text", "separator",
                                                     "placeholder", "image")]
            eve_surfaces.append({
                "surface": os.path.splitext(os.path.basename(p))[0],
                "file": rel,
                "is_adam_variant": p.lower().endswith(".adam.eve"),
                "layout_names": [l["layout_name"] for l in layouts],
                "control_count": len(controls),
                "interactive_control_count": len(interactive),
                "control_kinds": dict(collections.Counter(
                    c["kind"] for c in controls)),
                "controls": controls,
            })
    sources.append({"id": "P2_eve",
                    "how": ("Adobe Eve grammar parse; menu .eve files are "
                            "excluded here because they are reported in "
                            "premiere_commands_shortcuts.json"),
                    "eve_surfaces": len(eve_surfaces),
                    "eve_controls": sum(s["control_count"] for s in eve_surfaces)})

    # ---- P3 monitor overlays
    overlays = []
    ov_dir = os.path.join(R, "Monitor Overlays")
    for p in sorted(C.walk_files(ov_dir, exts=(".olp",))):
        try:
            tree = C.parse_propmap(p)
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "P3_overlay", "path": C.rel(p),
                             "error": repr(exc)})
            continue
        areas = []
        mad = tree.get("MonitorAreaDescriptions") or {}
        if isinstance(mad, dict):
            for k in sorted(mad):
                v = mad[k]
                if isinstance(v, dict):
                    areas.append({"area": k, **v})
        overlays.append({
            "overlay": os.path.splitext(os.path.basename(p))[0],
            "file": C.rel(p),
            "action_safe_area": tree.get("ActionSafeArea"),
            "title_safe_area": tree.get("TitleSafeArea"),
            "four_to_three_safe_margin": tree.get("4to3SafeMargin"),
            "enable_overlay_on_transmit": tree.get("EnableOverlayOnTransmit"),
            "area_descriptions": areas,
            "area_count": len(areas),
            "all_settings": {k: v for k, v in tree.items()
                             if k != "MonitorAreaDescriptions"},
        })
    sources.append({"id": "P3_monitor_overlays", "path": C.rel(ov_dir),
                    "how": "prop.map walk of the .olp overlay definitions",
                    "overlays": len(overlays)})

    # ---- P4 UI colour tables
    colour_tables = []
    tbl_dir = os.path.join(R, "table")
    for p in sorted(C.walk_files(tbl_dir, exts=(".table",))):
        try:
            with open(p, "r", encoding="utf-8", errors="replace") as fh:
                txt = fh.read()
        except OSError as exc:
            failures.append({"stage": "P4_table", "path": C.rel(p),
                             "error": repr(exc)})
            continue
        entries = re.findall(r"([A-Za-z_][\w.]*)\s*[:=]\s*([^\r\n;]+)", txt)
        colour_tables.append({
            "table": os.path.splitext(os.path.basename(p))[0],
            "file": C.rel(p),
            "bytes": os.path.getsize(p),
            "entry_count": len(entries),
            "entries": [{"key": k, "value": v.strip()} for k, v in entries],
        })
    sources.append({"id": "P4_colour_tables", "path": C.rel(tbl_dir),
                    "how": "key/value harvest over the shipped UI colour tables",
                    "tables": len(colour_tables),
                    "entries": sum(t["entry_count"] for t in colour_tables)})

    # ---- P5 string namespaces
    ns = {}
    for prefix, purpose in (("$$$/dvaui/", "shared dvaui widget strings"),
                            ("$$$/dvaworkspace/", "workspace and docking"),
                            ("$$$/Premiere/Frontend/", "front-end panels"),
                            ("$$$/Premiere/Handlers/", "panel handlers"),
                            ("$$$/Premiere/Libraries/Dialogs", "dialogs"),
                            ("$$$/Premiere/MZ/", "monitor and timeline"),
                            ("$$$/Overlay/", "monitor overlays"),
                            ("$$$/essentialsound", "Essential Sound panel"),
                            ("$$$/essentialsoundui", "Essential Sound panel UI")):
        rows = {k: v for k, v in table.items()
                if k.startswith(prefix) and not C.looks_ai(k)}
        if rows:
            ns[prefix] = {"purpose": purpose, "count": len(rows),
                          "strings": dict(sorted(rows.items()))}
    sources.append({"id": "P5_exe_strings",
                    "how": "namespace slice of the executable's $$$ literals",
                    "namespaces": len(ns),
                    "strings": sum(v["count"] for v in ns.values())})

    all_adapters = collections.Counter()
    for d in dialogs:
        for c in d["controls"]:
            if c.get("adapter"):
                all_adapters[c["adapter"]] += 1
    eve_kinds = collections.Counter()
    for s in eve_surfaces:
        eve_kinds.update(s["control_kinds"])

    # base vs V7 theme variants collapse to the same logical surface
    logical = {d["surface"].replace(".V7", "") for d in dialogs}

    payload = C.envelope(
        "handshake.studio.premiere.panels_dialogs.v1",
        {
            "summary": ("Panels, dialogs, workspaces and monitor overlays as "
                        "control trees. Two shipped UI description formats are "
                        "parsed: serialized dvaui prop.map archives and Adobe "
                        "Eve layout source."),
            "two_ui_formats": {
                "prop_map_UINodeArchive": ("a serialized live control tree: "
                                           "adapter class per node, plus id, "
                                           "text, tooltip, bounds, enabled and "
                                           "visible state"),
                "eve": ("declarative layout source: control kind, identifier, "
                        "localizable label and, where declared, range and "
                        "precision"),
            },
            "v7_variants": ("A surface shipped as both Name.xml and Name.V7.xml "
                            "is one logical surface with two theme-era layouts; "
                            "logical_surfaces counts it once."),
            "confidence_legend": {
                "parsed": "adapter class, id, text, bounds and state read verbatim",
                "derived from the dvaui class name": ("the human control role, "
                                                      "read off the class name"),
            },
            "known_gaps": [
                ("A dvaui archive stores the control tree, not the parameter "
                 "semantics behind it: a popup's option list is filled at "
                 "runtime and is not in the archive. Where an option list IS "
                 "shipped it appears in the executable's string namespaces, "
                 "which are included in full."),
                ("There are no .qml panel definitions in Premiere. The 458 .qml "
                 "files in the install all belong to the bundled mocha AE "
                 "plug-in's own Qt interface under PlugIns/(AfterEffectsLib)/"
                 "Effects/mochaAE, and describe that third-party tool, not "
                 "Premiere's UI."),
                ("Adobe's Essential Sound / Enhance Speech section archives that "
                 "belong to an excluded AI surface are dropped, not parsed; see "
                 "excluded_ai."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "dialog_and_panel_surfaces": len(dialogs),
                "logical_surfaces_ignoring_v7_variants": len(logical),
                "prop_map_controls_total": sum(d["control_count"] for d in dialogs),
                "prop_map_interactive_controls": sum(
                    d["interactive_control_count"] for d in dialogs),
                "eve_surfaces": len(eve_surfaces),
                "eve_controls_total": sum(s["control_count"] for s in eve_surfaces),
                "eve_interactive_controls": sum(
                    s["interactive_control_count"] for s in eve_surfaces),
                "workspace_layouts": len(workspaces),
                "distinct_panel_identifiers_across_workspaces": len(
                    {pid for w in workspaces for pid in w["panel_identifiers"]}),
                "monitor_overlays": len(overlays),
                "ui_colour_table_entries": sum(t["entry_count"] for t in colour_tables),
                "other_xml_by_kind": dict(others),
                "count_semantics": ("surfaces and controls are entity counts; "
                                    "other_xml_by_kind counts files that are not "
                                    "UI and were classified rather than parsed"),
            },
            "dvaui_adapter_census": dict(all_adapters.most_common()),
            "eve_control_kind_census": dict(eve_kinds.most_common()),
            "workspaces": workspaces,
            "dialogs_and_panels": dialogs,
            "eve_surfaces": eve_surfaces,
            "monitor_overlays": overlays,
            "ui_colour_tables": colour_tables,
            "non_ui_xml_inventory": other_files,
            "string_namespaces": ns,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_panels_dialogs.json", payload)
    print("wrote", path, size, "bytes")
    print("dialogs", len(dialogs), "eve", len(eve_surfaces), "workspaces",
          len(workspaces), "overlays", len(overlays), "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
