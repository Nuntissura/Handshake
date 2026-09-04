"""dw_command_surface.py -- Task 1: the complete Dreamweaver 2021 command surface.

Sources actually read (all plain text, all offline):
  Configuration/Menus/menus.xml            -- every menubar / menu / menuitem /
                                              separator / shortcutlist / shortcut /
                                              menugroup, with command JS, enabler JS,
                                              checked JS, showif JS, arguments,
                                              implementing file, platform, dynamic flag
  Configuration/Menus/Custom Sets/*.xml    -- the three shipped keyboard shortcut sets
  Configuration/Menus/Custom Sets/active set.txt  -- which set ships active
  Configuration/Menus/Adaptive Sets/*.xml  -- per-keyboard-layout shortcut remaps
  Configuration/KeyboardLayouts.xml        -- keyboard layout table
  en_US/Resources/*.zbin                   -- English labels for every mmstring key
  Configuration/**                         -- existence + entry points of each
                                              implementing file referenced by file=
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
from dw_zstrings import load_all_strings, resolve           # noqa: E402

MENUS = os.path.join(C.CONFIG, "Menus")
OUT_NAME = "dreamweaver_command_surface.json"


def build(out_path):
    exact, lower, smeta = load_all_strings(C.INSTALL_ROOT)
    failures = []

    def R(key):
        return resolve(key, exact, lower)

    # ---------------- menus.xml -------------------------------------------
    menus_xml = os.path.join(MENUS, "menus.xml")
    root, note = C.parse_xml_tolerant(menus_xml)
    if root is None:
        failures.append({"stage": "menus.xml", "path": C.rel(menus_xml), "error": note})
        return None, failures

    counters = {"menubar": 0, "menu": 0, "menuitem": 0, "separator": 0,
                "shortcut": 0, "shortcutlist": 0, "menugroup": 0, "groupitem": 0,
                "other": 0}
    implementing_files = {}

    # Dreamweaver resolves menuitem file= against Configuration/ for command and
    # menu providers, but Insert-menu entries name an object file relative to
    # Configuration/Objects/. Both bases are tried and the winner is recorded.
    SEARCH_BASES = [("Configuration", C.CONFIG),
                    ("Configuration/Objects", os.path.join(C.CONFIG, "Objects")),
                    ("Configuration/Commands", os.path.join(C.CONFIG, "Commands")),
                    ("Configuration/Menus", os.path.join(C.CONFIG, "Menus")),
                    ("Configuration/Shared", os.path.join(C.CONFIG, "Shared"))]

    def note_file(f):
        if not f:
            return None
        rel_f = f.replace("/", os.sep).replace("\\", os.sep)
        chosen, base_used = None, None
        for label, base in SEARCH_BASES:
            p = os.path.normpath(os.path.join(base, rel_f))
            if os.path.isfile(p):
                chosen, base_used = p, label
                break
        if chosen is None:
            chosen = os.path.normpath(os.path.join(C.CONFIG, rel_f))
        key = C.rel(chosen)
        if key not in implementing_files:
            implementing_files[key] = {"declared_as": f,
                                       "exists": base_used is not None,
                                       "resolved_against": base_used,
                                       "abs": chosen, "referenced_by": []}
        return key

    def conv(el, path):
        tag = el.tag.split("}")[-1]
        a = C.attrs_of(el)
        counters[tag if tag in counters else "other"] = \
            counters.get(tag if tag in counters else "other", 0) + 1
        node = {"node_type": tag, "id": a.get("id")}
        # label
        skey = a.get("mmstring:name") or a.get("mmstring:label")
        if skey:
            v, how = R(skey)
            node["label"] = v
            node["label_string_key"] = skey
            node["label_resolution"] = how
        elif a.get("name"):
            node["label"] = a["name"]
            node["label_resolution"] = "literal_attribute"
        # behaviour attributes, verbatim
        for src, dst in (("key", "shortcut_key"),
                         ("command", "command_js"),
                         ("file", "implementing_file"),
                         ("arguments", "arguments"),
                         ("enabled", "enabler_js"),
                         ("checked", "checked_js"),
                         ("showif", "showif_js"),
                         ("dynamic", "dynamic"),
                         ("domRequired", "dom_required"),
                         ("platform", "platform"),
                         ("tooltipLiveView", "tooltip_live_view"),
                         ("promptLiveSelection", "prompt_live_selection")):
            if src in a:
                node[dst] = a[src]
        if a.get("file"):
            k = note_file(a["file"])
            node["implementing_file_resolved"] = k
            implementing_files[k]["referenced_by"].append(a.get("id") or path)
        # anything not mapped above is still kept so nothing is silently lost
        mapped = {"id", "key", "command", "file", "arguments", "enabled", "checked",
                  "showif", "dynamic", "domRequired", "platform", "tooltipLiveView",
                  "promptLiveSelection", "mmstring:name", "mmstring:label", "name"}
        extra = {k: v for k, v in a.items() if k not in mapped}
        if extra:
            node["other_attributes"] = extra
        kids = [conv(c, (path + "/" + (a.get("id") or tag))) for c in list(el)]
        if kids:
            node["children"] = kids
        return node

    menubars, shortcutlists, menugroups, other_top = [], [], [], []
    for el in list(root):
        tag = el.tag.split("}")[-1]
        n = conv(el, tag)
        if tag == "menubar":
            menubars.append(n)
        elif tag == "shortcutlist":
            shortcutlists.append(n)
        elif tag == "menugroup":
            menugroups.append(n)
        else:
            other_top.append(n)

    # flat command index ---------------------------------------------------
    flat = []

    def flatten(n, bar, trail):
        if n["node_type"] in ("menuitem", "shortcut"):
            e = dict(n)
            e.pop("children", None)
            e["menubar_id"] = bar
            e["menu_path"] = trail
            flat.append(e)
        for c in n.get("children", []):
            flatten(c, bar, trail + [n.get("label") or n.get("id")]
                    if n["node_type"] in ("menu", "menubar") else trail)

    for bar in menubars + shortcutlists:
        for c in bar.get("children", []):
            flatten(c, bar.get("id"), [])

    # ---------------- shortcut sets ---------------------------------------
    sets = []
    csdir = os.path.join(MENUS, "Custom Sets")
    active = None
    ap = os.path.join(csdir, "active set.txt")
    if os.path.isfile(ap):
        active = C.read_text(ap).strip()
    for fn in sorted(os.listdir(csdir)) if os.path.isdir(csdir) else []:
        if not fn.lower().endswith(".xml"):
            continue
        p = os.path.join(csdir, fn)
        r, nt = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "shortcut_set", "path": C.rel(p), "error": nt})
            continue
        binds = []
        for sc in r.iter():
            if sc.tag.split("}")[-1].upper() != "SHORTCUT":
                continue
            a = C.attrs_of(sc)
            binds.append({"command_id": a.get("ID") or a.get("id"),
                          "keys": a.get("keys", ""),
                          "other_attributes": {k: v for k, v in a.items()
                                               if k not in ("ID", "id", "keys")} or None})
        sets.append({
            "file": C.rel(p),
            "set_name": C.attrs_of(r).get("name"),
            "set_type": C.attrs_of(r).get("type"),
            "is_active_on_a_clean_install": (C.attrs_of(r).get("name") == active),
            "binding_count": len(binds),
            "bindings_with_a_key": sum(1 for b in binds if b["keys"]),
            "bindings": binds,
        })

    # adaptive (keyboard-layout) sets
    adaptive = []
    adir = os.path.join(MENUS, "Adaptive Sets")
    for fn in sorted(os.listdir(adir)) if os.path.isdir(adir) else []:
        p = os.path.join(adir, fn)
        r, nt = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "adaptive_set", "path": C.rel(p), "error": nt})
            continue
        binds = [{"command_id": C.attrs_of(sc).get("ID") or C.attrs_of(sc).get("id"),
                  "keys": C.attrs_of(sc).get("keys", "")}
                 for sc in r.iter() if sc.tag.split("}")[-1].upper() == "SHORTCUT"]
        adaptive.append({"file": C.rel(p), "layout": os.path.splitext(fn)[0],
                         "binding_count": len(binds), "bindings": binds})

    # keyboard layout table
    kbd = {"file": None, "layouts": []}
    kp = os.path.join(C.CONFIG, "KeyboardLayouts.xml")
    if os.path.isfile(kp):
        r, nt = C.parse_xml_tolerant(kp)
        kbd["file"] = C.rel(kp)
        if r is None:
            failures.append({"stage": "KeyboardLayouts.xml", "error": nt})
        else:
            for el in r.iter():
                if el is r:
                    continue
                kbd["layouts"].append({"tag": el.tag.split("}")[-1],
                                       "attributes": C.attrs_of(el)})

    # ---------------- implementing files ----------------------------------
    impl = []
    for key, info in sorted(implementing_files.items()):
        rec = {"file": key, "declared_as": info["declared_as"],
               "exists": info["exists"],
               "resolved_against": info.get("resolved_against"),
               "referenced_by_command_ids": sorted(set(info["referenced_by"]))}
        if info["exists"] and os.path.splitext(key)[1].lower() in (".htm", ".html"):
            try:
                s = C.read_surface(info["abs"], R)
                rec["surface"] = {
                    "title": s["title"],
                    "control_count": len(s["controls"]),
                    "controls": s["controls"],
                    "js_functions": s["js_functions"],
                    "command_buttons": s["command_buttons"],
                    "html_comment_directives": s["html_comment_directives"],
                    "js_includes": s["js_includes"],
                }
            except Exception as exc:                     # noqa: BLE001
                failures.append({"stage": "implementing_surface", "path": key,
                                 "error": repr(exc)})
        impl.append(rec)

    # ---------------- the Commands/ catalogue (scripted commands) ---------
    cmd_dir = os.path.join(C.CONFIG, "Commands")
    commands = []
    for p in C.walk(cmd_dir, exts={".htm", ".html"}):
        try:
            s = C.read_surface(p, R)
        except Exception as exc:                         # noqa: BLE001
            failures.append({"stage": "Commands", "path": C.rel(p), "error": repr(exc)})
            continue
        stem = os.path.splitext(p)[0]
        js_side = stem + ".js"
        fns = set(s["js_functions"])
        commands.append({
            "command_file": C.rel(p),
            "companion_js": C.rel(js_side) if os.path.isfile(js_side) else None,
            "display_title": s["title"],
            "menu_location_directive": s["html_comment_directives"].get("MENU-LOCATION"),
            "reachable_from_a_menu": C.rel(p) in implementing_files,
            "dialog_controls": s["controls"],
            "dialog_control_count": len(s["controls"]),
            "dialog_buttons": s["command_buttons"],
            "api_hooks_implemented": sorted(fns & {
                "canAcceptCommand", "commandButtons", "receiveArguments",
                "isDOMRequired", "windowDimensions", "objectTag", "displayHelp",
                "initializeUI", "applyBehavior", "inspectBehavior",
                "behaviorFunction", "identifyBehaviorArguments", "isDomRequired"}),
            "js_functions": sorted(fns),
            "localized_strings_used": s["localized_strings_used"],
        })

    # ---------------- envelope --------------------------------------------
    method = {
        "task": "1 - command and menu surface",
        "how": [
            "menus.xml parsed with ElementTree after a tolerant pass that binds the "
            "MMString namespace prefix; every element and every attribute is carried "
            "through, unknown attributes land in other_attributes so nothing is dropped",
            "every mmstring:name / mmstring:label key resolved against the shipped "
            "en_US ZString tables decoded by dw_zstrings.py",
            "each file= reference resolved against Configuration/ and, when it is an "
            "HTML extension surface, opened and parsed for its form controls, its "
            "commandButtons() dialog buttons and its JS entry points",
            "Configuration/Commands/*.htm walked independently so scripted commands "
            "that are only reachable by API or by another command are still catalogued",
            "the three shipped keyboard shortcut sets and the 23 adaptive keyboard "
            "layout sets parsed and keyed by command id",
        ],
        "not_done": [
            "menu labels are the shipped en_US strings; other locales were not decoded",
            "enabler / command / checked attributes are captured as JavaScript source "
            "text; they were not evaluated (that would require running the app)",
        ],
        "string_tables": smeta,
    }
    doc = C.envelope("handshake.studio.dreamweaver.command_surface.v1", method, {
        "counts": {
            "menubars": len(menubars),
            "context_menubars": len(menubars) - 1,
            "shortcutlists": len(shortcutlists),
            "menugroups": len(menugroups),
            "menu_nodes_by_type": counters,
            "flat_invocable_entries": len(flat),
            "flat_entries_with_a_shortcut": sum(1 for f in flat if f.get("shortcut_key")),
            "flat_entries_with_inline_command_js": sum(1 for f in flat if f.get("command_js")),
            "flat_entries_backed_by_a_file": sum(1 for f in flat if f.get("implementing_file")),
            "distinct_implementing_files": len(impl),
            "implementing_files_missing_on_disk": sum(1 for i in impl if not i["exists"]),
            "scripted_command_surfaces_in_Commands_dir": len(commands),
            "shortcut_sets": len(sets),
            "adaptive_keyboard_layout_sets": len(adaptive),
            "string_keys_available": len(exact),
        },
        "menus_xml_parse_note": note,
        "active_shortcut_set_on_clean_install": active,
        "menubars": menubars,
        "shortcutlists": shortcutlists,
        "menugroups": menugroups,
        "other_top_level_nodes": other_top,
        "flat_command_index": flat,
        "implementing_files": impl,
        "scripted_commands": commands,
        "keyboard_shortcut_sets": sets,
        "adaptive_keyboard_layout_sets": adaptive,
        "keyboard_layouts": kbd,
        "excluded_ai": C.excluded_ai(
            "command and menu surface",
            candidates=[f.get("id") for f in flat] + [f.get("label") for f in flat]
                       + [b.get("id") for b in menubars]
                       + [i["file"] for i in impl]
                       + [c["command_file"] for c in commands],
            extra_note="Checked every menu item id, every resolved English menu "
                       "label, every menubar id, every implementing file path and "
                       "every scripted command file in Configuration/Commands."),
        "failures": failures,
    })
    size = C.write_json(out_path, doc)
    return doc, size


if __name__ == "__main__":
    out = sys.argv[1]
    doc, size = build(out)
    import json
    print(json.dumps(doc["counts"], indent=1))
    print("failures:", len(doc["failures"]))
    print("bytes:", size)
