"""dw_panels_dialogs.py -- Task 2: panels, dialogs, inspectors, toolbars.

Sources actually read (all offline):
  Configuration/Dialogs/Eve/*.eve   -- 259 native modal dialogs in Adobe's Eve
                                       layout language; parsed to a full control
                                       tree with labels, identifiers, defaults
  Configuration/Dialogs/Eve/*.adm   -- 4 legacy Adobe Dialog Manager sheets
  Configuration/Floaters/*          -- the HTML-implemented floating panels
  Configuration/Inspectors/*.htm|html -- the Property inspector surfaces
  Configuration/Toolbars/toolbars.xml -- every toolbar and every toolbar item
  Configuration/ToolbarsOptions/customize.xml -- which items are user-customizable
  Configuration/workspace/*.xml     -- shipped workspace layouts and panel docking
  Configuration/Menus/menus.xml     -- panel ids reachable via dw.toggleFloater()
  en_US/Resources/*.zbin            -- English labels
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
import dw_eve                                               # noqa: E402
from dw_zstrings import load_all_strings, resolve           # noqa: E402

TOGGLE_RE = re.compile(r"toggleFloater\(\s*['\"]([^'\"]+)['\"]")
FLOATER_CALL_RE = re.compile(
    r"(?:showFloater|toggleFloater|setFloaterVisibility|getFloaterVisibility)"
    r"\(\s*['\"]([^'\"]+)['\"]")


def build(out_path):
    exact, lower, smeta = load_all_strings(C.INSTALL_ROOT)
    failures = []

    def R(key):
        return resolve(key, exact, lower)

    # ---------------- native dialogs (.eve) --------------------------------
    eve_dir = os.path.join(C.CONFIG, "Dialogs", "Eve")
    dialogs = []
    control_kind_tally = {}
    for p in sorted(C.walk(eve_dir, exts={".eve"})):
        try:
            layouts = dw_eve.parse_eve(C.read_text(p))
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "eve", "path": C.rel(p), "error": repr(exc)})
            continue
        if not layouts:
            failures.append({"stage": "eve", "path": C.rel(p),
                             "error": "no layout block found"})
            continue
        for lay in layouts:
            flat = dw_eve.flatten_controls(lay["nodes"])
            for c in flat:
                control_kind_tally[c["kind"]] = control_kind_tally.get(c["kind"], 0) + 1
            roots = [n for n in lay["nodes"]]
            top = roots[0]["args"] if roots else {}
            tkey, ttl = dw_eve.split_localized(top.get("name"))
            operable = [c for c in flat
                        if c["kind"] not in ("row", "column", "group", "placeholder",
                                             "separator", "static_text", "image",
                                             "dialog", "subview", "panel")]
            dialogs.append({
                "surface_kind": "native modal dialog (Eve layout)",
                "layout_name": lay["layout_name"],
                "file": C.rel(p),
                "window_title": ttl if isinstance(ttl, str) else None,
                "window_title_string_key": tkey,
                "window_identifier": top.get("identifier"),
                "window_args": top,
                "root_view_kind": roots[0]["kind"] if roots else None,
                "node_count": len(flat),
                "operable_control_count": len(operable),
                "controls": flat,
                "provenance": "parsed",
            })

    adm = []
    for p in sorted(C.walk(eve_dir, exts={".adm"})):
        txt = C.read_text(p)
        m = re.search(r"sheet\s+(\w+)", txt)
        iface = re.findall(r"(\w+)\s*:\s*@(\w+)\s*;", txt)
        adm.append({
            "surface_kind": "legacy Adobe Dialog Manager sheet",
            "file": C.rel(p),
            "sheet_name": m.group(1) if m else None,
            "interface_items": [{"name": a, "type": b} for a, b in iface],
            "raw": txt,
            "provenance": "parsed",
        })

    # ---------------- HTML-implemented panels (Floaters) -------------------
    floaters = []
    fdir = os.path.join(C.CONFIG, "Floaters")
    for p in sorted(C.walk(fdir, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "Floaters", "path": C.rel(p), "error": repr(exc)})
            continue
        s["surface_kind"] = "HTML-implemented floating panel"
        s["provenance"] = "parsed"
        floaters.append(s)
    floater_support = sorted(C.rel(p) for p in C.walk(fdir)
                             if os.path.splitext(p)[1].lower() in (".js", ".css"))

    # ---------------- Property inspectors ----------------------------------
    inspectors = []
    idir = os.path.join(C.CONFIG, "Inspectors")
    for p in sorted(C.walk(idir, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "Inspectors", "path": C.rel(p), "error": repr(exc)})
            continue
        txt = C.read_text(p)
        # A DW property inspector declares which selection it binds to in an
        # HTML comment: <!-- tag:img,priority:5,selection:within,hline,vline -->
        binding = None
        m = re.search(r"<!--\s*(tag:[^>]*?)-->", txt, re.I | re.S)
        if m:
            binding = {}
            for part in m.group(1).split(","):
                part = part.strip()
                if not part:
                    continue
                if ":" in part:
                    k, v = part.split(":", 1)
                    binding[k.strip().lower()] = v.strip()
                else:
                    binding[part.lower()] = True
        fns = set(s["js_functions"])
        inspectors.append({
            "surface_kind": "property inspector",
            "file": C.rel(p),
            "title": s["title"],
            "binding_declaration": binding,
            "binds_to_tag": (binding or {}).get("tag"),
            "priority": (binding or {}).get("priority"),
            "selection_scope": (binding or {}).get("selection"),
            "control_count": len(s["controls"]),
            "controls": s["controls"],
            "api_hooks_implemented": sorted(fns & {"canInspectSelection", "inspectSelection",
                                                   "displayHelp", "initializeUI"}),
            "js_functions": sorted(fns),
            "js_includes": s["js_includes"],
            "localized_strings_used": s["localized_strings_used"],
            "provenance": "parsed",
        })

    # ---------------- toolbars ---------------------------------------------
    tb_path = os.path.join(C.CONFIG, "Toolbars", "toolbars.xml")
    toolbars = []
    tb_item_kinds = {}
    root, note = C.parse_xml_tolerant(tb_path)
    if root is None:
        failures.append({"stage": "toolbars.xml", "error": note})
    else:
        def tb_node(el):
            tag = el.tag.split("}")[-1]
            a = C.attrs_of(el)
            rec = {"item_kind": tag, "id": a.get("id")}
            for src, dst in (("mmstring:label", "label_string_key"),
                             ("mmstring:tooltip", "tooltip_string_key")):
                if src in a:
                    v, how = R(a[src])
                    rec[dst] = a[src]
                    rec[dst.replace("_string_key", "")] = v
                    rec[dst.replace("_string_key", "_resolution")] = how
            for src, dst in (("command", "command_js"), ("enabled", "enabler_js"),
                             ("checked", "checked_js"), ("showif", "showif_js"),
                             ("value", "value_js"), ("update", "update_event"),
                             ("domRequired", "dom_required"),
                             ("buttonGroup", "button_group"),
                             ("platform", "platform"), ("file", "implementing_file"),
                             ("image", "image"), ("width", "width"),
                             ("container", "container"),
                             ("backgroundStyle", "background_style"),
                             ("label", "label_literal"),
                             ("tooltip", "tooltip_literal"),
                             ("disabledImage", "disabled_image")):
                if src in a:
                    rec[dst] = a[src]
            mapped = {"id", "command", "enabled", "checked", "showif", "value",
                      "update", "domRequired", "buttonGroup", "platform", "file",
                      "image", "width", "container", "backgroundStyle", "label",
                      "tooltip", "disabledImage", "mmstring:label", "mmstring:tooltip"}
            extra = {k: v for k, v in a.items() if k not in mapped}
            if extra:
                rec["other_attributes"] = extra
            kids = [tb_node(c) for c in list(el)]
            if kids:
                rec["items"] = kids
            return rec

        for el in list(root):
            tag = el.tag.split("}")[-1]
            if tag != "toolbar":
                continue
            t = tb_node(el)
            t["surface_kind"] = "toolbar"
            t["provenance"] = "parsed"
            toolbars.append(t)

        def tally(n):
            tb_item_kinds[n["item_kind"]] = tb_item_kinds.get(n["item_kind"], 0) + 1
            for c in n.get("items", []):
                tally(c)
        for t in toolbars:
            tally(t)

    # customizable toolbar items
    customize = []
    cp = os.path.join(C.CONFIG, "ToolbarsOptions", "customize.xml")
    r2, n2 = C.parse_xml_tolerant(cp)
    if r2 is None:
        failures.append({"stage": "customize.xml", "error": n2})
    else:
        for el in r2.iter():
            if el.tag.split("}")[-1] == "toolbar":
                customize.append({
                    "toolbar_id": C.attrs_of(el).get("id"),
                    "customizable_item_ids": [C.attrs_of(c).get("id") for c in list(el)],
                })

    # ---------------- workspace layouts ------------------------------------
    workspaces = []
    wdir = os.path.join(C.CONFIG, "workspace")
    for p in sorted(C.walk(wdir, exts={".xml"})):
        r3, n3 = C.parse_xml_tolerant(p)
        if r3 is None:
            failures.append({"stage": "workspace", "path": C.rel(p), "error": n3})
            continue

        def w_node(el):
            rec = {"node": el.tag.split("}")[-1], "attributes": C.attrs_of(el)}
            kids = [w_node(c) for c in list(el)]
            if kids:
                rec["children"] = kids
            return rec
        workspaces.append({
            "workspace_name": os.path.splitext(os.path.basename(p))[0],
            "file": C.rel(p),
            "tree": w_node(r3),
            "docked_panel_ids": [C.attrs_of(e).get("id") for e in r3.iter()
                                 if e.tag.split("}")[-1] == "panel"],
            "provenance": "parsed",
        })

    # ---------------- panel registry ---------------------------------------
    menus_txt = C.read_text(os.path.join(C.CONFIG, "Menus", "menus.xml"))
    panel_ids = sorted(set(FLOATER_CALL_RE.findall(menus_txt)))
    ws_ids = sorted({i for w in workspaces for i in w["docked_panel_ids"] if i})
    all_panels = sorted(set(panel_ids) | set(ws_ids))
    # menu label for each toggleFloater panel, taken from the menu item that calls it
    panel_labels = {}
    for m in re.finditer(
            r'<menuitem[^>]*mmstring:name="([^"]+)"[^>]*toggleFloater\(\s*[\'"]([^\'"]+)',
            menus_txt):
        v, _ = R(m.group(1))
        if v:
            panel_labels.setdefault(m.group(2), v)
    for m in re.finditer(
            r'<menuitem[^>]*toggleFloater\(\s*[\'"]([^\'"]+)[^>]*mmstring:name="([^"]+)"',
            menus_txt):
        v, _ = R(m.group(2))
        if v:
            panel_labels.setdefault(m.group(1), v)

    panel_registry = []
    for pid in all_panels:
        panel_registry.append({
            "panel_id": pid,
            "menu_label": panel_labels.get(pid),
            "menu_label_provenance": "resolved" if panel_labels.get(pid) else "not_in_menu",
            "reachable_via_dw_toggleFloater": pid in panel_ids,
            "docked_in_shipped_workspace": [w["workspace_name"] for w in workspaces
                                            if pid in w["docked_panel_ids"]],
            "implementation": ("HTML floater in Configuration/Floaters"
                               if any(pid.lower() in f["file"].lower() for f in floaters)
                               else "native (no HTML surface ships for this id)"),
            "implementation_provenance": "heuristic: matched panel id against the "
                                         "Floaters/ filenames; a native panel ships no "
                                         "declarative surface, so its controls are not "
                                         "recoverable from the Configuration tree",
        })

    method = {
        "task": "2 - panels, dialogs, inspectors, toolbars",
        "how": [
            "Configuration/Dialogs/Eve/*.eve parsed with a purpose-written "
            "recursive-descent parser for Adobe's Eve layout language "
            "(dw_eve.py). Every node, its kind, its identifier, its geometry "
            "and all of its arguments are kept. Labels come from the inline "
            "'$$$/Key=Default English' form, so they are read literally out of "
            "the shipped file, not guessed.",
            "control_role and value_kind are a fixed lookup from the Eve widget "
            "kind to a rebuild-facing description; that mapping is this tool's, "
            "hence labelled heuristic_mapping below.",
            "Configuration/Inspectors/*.htm parsed for form controls and for the "
            "HTML comment that declares which tag and selection the inspector "
            "binds to.",
            "Configuration/Toolbars/toolbars.xml parsed with the lenient scanner "
            "because toolbar attribute values contain raw JavaScript.",
            "Panel registry assembled from dw.toggleFloater()/showFloater() ids "
            "found in menus.xml plus the panel ids docked in the shipped "
            "workspace layouts.",
        ],
        "known_gap": "Dreamweaver's major panels (Files, CSS Designer, Insert, DOM, "
                     "Assets, Snippets, Behaviors, Properties) are native C++ "
                     "surfaces. They ship no declarative control file, so this "
                     "export carries their ids, their menu labels and their docking, "
                     "but not a control-by-control parameter list. Their editable "
                     "parameters are recovered instead through the CSS property "
                     "surface (see dreamweaver_templates_css.json) and through the "
                     "dialogs they open (above).",
        "heuristic_mapping": dw_eve.CONTROL_SEMANTICS,
        "string_tables": smeta,
    }

    doc = C.envelope("handshake.studio.dreamweaver.panels_dialogs.v1", method, {
        "counts": {
            "native_dialog_layouts": len(dialogs),
            "native_dialog_files": len({d["file"] for d in dialogs}),
            "native_dialog_nodes_total": sum(d["node_count"] for d in dialogs),
            "native_dialog_operable_controls_total": sum(d["operable_control_count"]
                                                         for d in dialogs),
            "eve_control_kinds_seen": control_kind_tally,
            "legacy_adm_sheets": len(adm),
            "html_floating_panels": len(floaters),
            "html_floating_panel_support_files": len(floater_support),
            "property_inspectors": len(inspectors),
            "property_inspectors_with_a_binding_declaration":
                sum(1 for i in inspectors if i["binding_declaration"]),
            "property_inspector_controls_total": sum(i["control_count"] for i in inspectors),
            "toolbars": len(toolbars),
            "toolbar_items_by_kind": tb_item_kinds,
            "customizable_toolbars": len(customize),
            "shipped_workspace_layouts": len(workspaces),
            "distinct_panel_ids": len(panel_registry),
        },
        "native_dialogs": dialogs,
        "legacy_adm_sheets": adm,
        "html_floating_panels": floaters,
        "html_floating_panel_support_files": floater_support,
        "property_inspectors": inspectors,
        "toolbars": toolbars,
        "toolbar_customization": customize,
        "workspace_layouts": workspaces,
        "panel_registry": panel_registry,
        "excluded_ai": C.excluded_ai(
            "panels, dialogs, inspectors and toolbars",
            candidates=[d["layout_name"] for d in dialogs]
                       + [d.get("window_title") for d in dialogs]
                       + [c.get("label") for d in dialogs for c in d["controls"]]
                       + [i["file"] for i in inspectors]
                       + [p["panel_id"] for p in panel_registry]
                       + [p.get("menu_label") for p in panel_registry],
            extra_note="Checked every Eve layout name, every dialog title, every "
                       "control label in every native dialog, every property "
                       "inspector file and every panel id and panel menu label."),
        "failures": failures,
    })
    size = C.write_json(out_path, doc)
    return doc, size


if __name__ == "__main__":
    import json
    doc, size = build(sys.argv[1])
    print(json.dumps(doc["counts"], indent=1))
    print("failures:", len(doc["failures"]))
    for f in doc["failures"][:10]:
        print("  ", f)
    print("bytes:", size)
