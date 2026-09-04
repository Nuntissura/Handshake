"""dw_objects_behaviors.py -- Task 3: the Insert object catalogue and the
behaviour catalogue.

Sources actually read (all offline):
  Configuration/Objects/insertbar.xml  -- the Insert panel: categories, buttons,
                                          menubuttons, enabler expressions,
                                          live-view insertion mode
  Configuration/Objects/**/*.htm       -- every object implementation: the
                                          markup it inserts, whether it needs a
                                          DOM, which dialog it pops, its own
                                          dialog controls when it has one
  Configuration/Behaviors/Actions/**   -- the behaviour catalogue: dialog
                                          controls, the JS function each writes
                                          into the page, its API hooks
  Configuration/Behaviors/Events/*.htm -- the tag -> allowed-event table with
                                          the default event marked '*'
  en_US/Resources/*.zbin               -- English labels
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
from dw_zstrings import load_all_strings, resolve           # noqa: E402

OBJ_HOOKS = {"objectTag", "isDOMRequired", "isDomRequired", "windowDimensions",
             "displayHelp", "initializeUI", "canInsertObject", "insertObject",
             "beforeInsert", "afterInsert"}
BEH_HOOKS = {"canAcceptBehavior", "behaviorFunction", "applyBehavior",
             "inspectBehavior", "identifyBehaviorArguments", "deleteBehavior",
             "windowDimensions", "displayHelp", "initializeUI"}
EVENT_TAG_RE = re.compile(r"<([A-Za-z][\w\-]*)((?:\s+on[A-Za-z]+\s*=\s*\"[^\"]*\")+)\s*>",
                          re.S)
EVENT_ATTR_RE = re.compile(r"(on[A-Za-z]+)\s*=\s*\"([^\"]*)\"")


def build(out_path):
    exact, lower, smeta = load_all_strings(C.INSTALL_ROOT)
    failures = []

    def R(key):
        return resolve(key, exact, lower)

    obj_root = os.path.join(C.CONFIG, "Objects")

    # ---------------- responsive framework component markup ----------------
    # The Responsive/Bootstrap insert objects do not carry their own markup.
    # They call copyAssetsAndInsertComponent('<component-key>') and the markup
    # for that key is stored in the framework resource XML below.
    fw_dir = os.path.join(C.CONFIG, "Shared", "LiveEdit", "Extensions",
                          "ResponsiveLayout", "assets", "resource")
    frameworks = []
    component_markup = {}          # (framework_file, key) -> markup
    component_markup_by_key = {}   # key -> [{framework, markup}]
    for p in sorted(C.walk(fw_dir, exts={".xml"})):
        r, nt = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "framework_resource", "path": C.rel(p),
                             "error": nt})
            continue
        fw = {"file": C.rel(p), "settings": {}, "components": {}}
        for el in r.iter():
            tag = el.tag.split("}")[-1]
            txt = (el.text or "").strip()
            if not len(list(el)) and txt:
                if "<" in txt or "&lt;" in txt:
                    fw["components"][tag] = txt
                    component_markup[(C.rel(p), tag)] = txt
                    component_markup_by_key.setdefault(tag, []).append(
                        {"framework_file": C.rel(p), "markup": txt})
                else:
                    fw["settings"].setdefault(tag, []).append(txt)
        fw["component_count"] = len(fw["components"])
        fw["framework_name"] = (fw["settings"].get("frameworkName") or [None])[0]
        fw["version_supported_from"] = (fw["settings"]
                                        .get("frameworkVersionSupportFrom") or [None])[0]
        frameworks.append(fw)

    COMPONENT_CALL_RE = re.compile(
        r"(?:copyAssetsAndInsertComponent|insertComponent|insertBootstrapComponent)"
        r"\s*\(\s*['\"]([^'\"]+)['\"]")

    # ---------------- objects on disk --------------------------------------
    objects_by_relpath = {}
    for p in sorted(C.walk(obj_root, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
            txt = C.read_text(p)
            js, _inc = C.extract_js(txt, os.path.dirname(p))
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "object", "path": C.rel(p), "error": repr(exc)})
            continue
        fns = set(s["js_functions"])
        body = C.js_block(js, "objectTag")
        ins = C.extract_insert_templates(body if body is not None else js)
        dom_req = C.literal_returns(js, "isDOMRequired") or \
            C.literal_returns(js, "isDomRequired")
        if dom_req is None:
            b = C.js_block(js, "isDOMRequired") or C.js_block(js, "isDomRequired")
            if b is not None:
                m = re.search(r"return\s+(true|false)", b)
                dom_req = [m.group(1)] if m else None
        wd = C.js_block(js, "windowDimensions")
        # The most common DW object idiom: objectTag() returns the file's own
        # <body>, so the body markup IS the inserted markup, verbatim.
        body_is_the_markup = bool(body and re.search(
            r"return\s+document\.body\.innerHTML", body))
        body_markup = C.body_inner_html(txt) if body_is_the_markup else None
        comp_keys = sorted(set(COMPONENT_CALL_RE.findall(body if body else js)))
        comp_markup = {k: component_markup_by_key[k]
                       for k in comp_keys if k in component_markup_by_key}
        rec = {
            "object_file": C.rel(p),
            "folder": os.path.basename(os.path.dirname(p)),
            "display_title": s["title"],
            "display_title_source": s["title_source"],
            "html_comment_directives": s["html_comment_directives"],
            "requires_dom": (dom_req[0] if dom_req else None),
            "requires_dom_provenance": "parsed from isDOMRequired() return literal"
                                       if dom_req else "not declared in file",
            "window_dimensions_js": (wd.strip() if wd else None),
            "inserts": {
                "framework_component_keys": comp_keys or None,
                "framework_component_markup": comp_markup or None,
                "body_markup": body_markup,
                "body_markup_is_the_insertion": body_is_the_markup,
                "returned_markup": ins["returns"],
                "markup_variables": ins["assignments"],
                "insert_helper_calls": ins["calls"],
                "opens_dialog": ins["popup_commands"],
            },
            "insert_extraction_provenance":
                "parsed: string literals inside objectTag() are reproduced verbatim; "
                "runtime-computed fragments are preserved as {{js:...}} placeholders "
                "so the rebuild can see exactly where a value is substituted",
            "own_dialog_controls": s["controls"],
            "own_dialog_control_count": len(s["controls"]),
            "api_hooks_implemented": sorted(fns & OBJ_HOOKS),
            "js_functions": sorted(fns),
            "js_includes": s["js_includes"],
            "localized_strings_used": s["localized_strings_used"],
        }
        objects_by_relpath[C.rel(p)] = rec

    objects_lower = {k.lower(): k for k in objects_by_relpath}

    # ---------------- insertbar.xml ----------------------------------------
    ib_path = os.path.join(obj_root, "insertbar.xml")
    root, note = C.parse_xml_tolerant(ib_path)
    categories = []
    entry_count = {"category": 0, "button": 0, "menubutton": 0, "separator": 0,
                   "checkbutton": 0, "radiobutton": 0, "other": 0}
    linked, unlinked = set(), set()

    def ib_node(el, cat, parent):
        tag = el.tag.split("}")[-1]
        entry_count[tag if tag in entry_count else "other"] = \
            entry_count.get(tag if tag in entry_count else "other", 0) + 1
        a = C.attrs_of(el)
        rec = {"entry_kind": tag, "id": a.get("id"), "category": cat,
               "parent_menubutton": parent}
        for src, dst in (("mmstring:label", "label"), ("mmstring:name", "name")):
            if src in a:
                v, how = R(a[src])
                rec[dst] = v
                rec[dst + "_string_key"] = a[src]
                rec[dst + "_resolution"] = how
        for src, dst in (("file", "object_file_declared"),
                         ("enabled", "enabler_js"),
                         ("showif", "showif_js"),
                         ("promptLiveSelection", "live_view_insert_mode"),
                         ("image", "icon"),
                         ("folder", "folder"),
                         ("command", "command_js"),
                         ("platform", "platform")):
            if src in a:
                rec[dst] = a[src]
        if a.get("file"):
            key = C.rel(os.path.normpath(os.path.join(obj_root,
                                                      a["file"].replace("\\", os.sep))))
            # insertbar.xml spells some paths with different casing than the
            # files on disk (jqueryWidgets vs jQueryWidgets, CFTextarea vs
            # CFTextArea). Windows resolves those; match case-insensitively.
            if key not in objects_by_relpath:
                key = objects_lower.get(key.lower(), key)
            rec["object_file"] = key
            obj = objects_by_relpath.get(key)
            rec["object_file_exists"] = obj is not None
            if obj is not None:
                linked.add(key)
                rec["inserts"] = obj["inserts"]
                rec["requires_dom"] = obj["requires_dom"]
                rec["own_dialog_control_count"] = obj["own_dialog_control_count"]
                rec["api_hooks_implemented"] = obj["api_hooks_implemented"]
            else:
                failures.append({"stage": "insertbar_link", "id": a.get("id"),
                                 "declared_file": a["file"],
                                 "error": "no object implementation at " + key})
        mapped = {"id", "file", "enabled", "showif", "promptLiveSelection", "image",
                  "folder", "command", "platform", "mmstring:label", "mmstring:name"}
        extra = {k: v for k, v in a.items() if k not in mapped}
        if extra:
            rec["other_attributes"] = extra
        kids = [ib_node(c, cat, a.get("id")) for c in list(el)]
        if kids:
            rec["items"] = kids
        return rec

    if root is None:
        failures.append({"stage": "insertbar.xml", "error": note})
    else:
        for el in list(root):
            if el.tag.split("}")[-1] != "category":
                continue
            a = C.attrs_of(el)
            cat_label, how = R(a.get("mmstring:name")) if a.get("mmstring:name") else (None, "none")
            entry_count["category"] += 1
            cat = {
                "category_id": a.get("id"),
                "category_label": cat_label,
                "category_label_string_key": a.get("mmstring:name"),
                "category_label_resolution": how,
                "folder": a.get("folder"),
                "items": [ib_node(c, a.get("id"), None) for c in list(el)],
                "provenance": "parsed",
            }
            categories.append(cat)

    unlinked = sorted(set(objects_by_relpath) - linked)

    # ---------------- behaviours -------------------------------------------
    beh_root = os.path.join(C.CONFIG, "Behaviors", "Actions")
    behaviors = []
    for p in sorted(C.walk(beh_root, exts={".htm", ".html"})):
        try:
            s = C.read_surface(p, R)
            txt = C.read_text(p)
            js, _inc = C.extract_js(txt, os.path.dirname(p))
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "behavior", "path": C.rel(p), "error": repr(exc)})
            continue
        fns = set(s["js_functions"])
        bf = C.js_block(js, "behaviorFunction")
        emitted = None
        if bf is not None:
            names = re.findall(r"return\s+(?:new\s+Array\s*\()?\s*['\"]([^'\"]+)['\"]", bf)
            emitted = names or None
        apply_body = C.js_block(js, "applyBehavior")
        apply_tpl = C.extract_insert_templates(apply_body) if apply_body else None
        # the MM_* helper actually written into the page
        page_fns = sorted(set(re.findall(r"function\s+(MM_\w+)\s*\(", js)))
        behaviors.append({
            "behavior_file": C.rel(p),
            "group": os.path.relpath(os.path.dirname(p), beh_root).replace("\\", "/"),
            "display_title": s["title"],
            "html_comment_directives": s["html_comment_directives"],
            "safe_in_templates": "SAFE-IN-TEMPLATES" in txt,
            "parameter_dialog_controls": s["controls"],
            "parameter_dialog_control_count": len(s["controls"]),
            "emits_page_functions_declared_by_behaviorFunction": emitted,
            "helper_functions_shipped_into_the_page": page_fns,
            "event_handler_written_by_applyBehavior": apply_tpl,
            "api_hooks_implemented": sorted(fns & BEH_HOOKS),
            "js_functions": sorted(fns),
            "js_includes": s["js_includes"],
            "localized_strings_used": s["localized_strings_used"],
            "provenance": "parsed",
        })
    behavior_shared_js = sorted(C.rel(p) for p in C.walk(beh_root, exts={".js"})
                                if not os.path.isfile(os.path.splitext(p)[0] + ".htm"))

    # ---------------- behaviour events -------------------------------------
    events = []
    ev_dir = os.path.join(C.CONFIG, "Behaviors", "Events")
    for p in sorted(C.walk(ev_dir, exts={".htm", ".html"})):
        txt = C.read_text(p)
        tags = []
        for m in EVENT_TAG_RE.finditer(txt):
            attrs = EVENT_ATTR_RE.findall(m.group(2))
            tags.append({
                "tag": m.group(1),
                "events": [a for a, _ in attrs],
                "default_event": next((a for a, v in attrs if v.strip() == "*"), None),
            })
        events.append({
            "event_model_file": C.rel(p),
            "event_model_name": os.path.splitext(os.path.basename(p))[0],
            "tag_count": len(tags),
            "distinct_events": sorted({e for t in tags for e in t["events"]}),
            "tags": tags,
            "provenance": "parsed",
        })

    method = {
        "task": "3 - Insert objects and behaviours",
        "how": [
            "insertbar.xml parsed for the Insert panel tree: every category, "
            "button and menubutton with its enabler JavaScript, its live-view "
            "insertion mode (promptLiveSelection) and its object file.",
            "Every Configuration/Objects/**/*.htm opened. The body of objectTag() "
            "is brace-matched and its string literals reproduced verbatim; "
            "runtime-computed fragments are preserved inline as {{js:...}} so the "
            "rebuild sees the exact markup shape and the exact substitution points.",
            "isDOMRequired() is read from its literal return; windowDimensions() is "
            "carried as source because it computes sizes at runtime.",
            "Behaviours: parameter dialog controls parsed from the .htm; the "
            "function the behaviour writes into the page read from "
            "behaviorFunction(); the MM_* helpers it ships read from the JS.",
            "Behaviors/Events/*.htm parsed into a tag -> allowed events table; the "
            "'*' marker is Dreamweaver's own notation for the default event.",
        ],
        "not_done": [
            "Object scripts were not executed. Where an object builds its markup "
            "from a dialog result or from document state, the export shows the "
            "template and the substitution points, not a final string.",
        ],
        "string_tables": smeta,
    }

    doc = C.envelope("handshake.studio.dreamweaver.objects_behaviors.v1", method, {
        "counts": {
            "insert_panel_categories": len(categories),
            "insert_panel_entries_by_kind": entry_count,
            "insert_panel_entries_total": sum(v for k, v in entry_count.items()
                                              if k != "category"),
            "object_implementations_on_disk": len(objects_by_relpath),
            "object_implementations_wired_into_the_insert_panel": len(linked),
            "object_implementations_not_on_the_insert_panel": len(unlinked),
            "objects_that_insert_literal_markup":
                sum(1 for o in objects_by_relpath.values()
                    if o["inserts"]["returned_markup"] or o["inserts"]["markup_variables"]
                    or o["inserts"]["insert_helper_calls"] or o["inserts"]["body_markup"]
                    or o["inserts"]["framework_component_markup"]),
            "objects_whose_body_is_the_inserted_markup":
                sum(1 for o in objects_by_relpath.values() if o["inserts"]["body_markup"]),
            "objects_that_insert_a_framework_component":
                sum(1 for o in objects_by_relpath.values()
                    if o["inserts"]["framework_component_keys"]),
            "objects_whose_framework_component_markup_was_resolved":
                sum(1 for o in objects_by_relpath.values()
                    if o["inserts"]["framework_component_markup"]),
            "responsive_framework_resource_files": len(frameworks),
            "framework_component_markup_entries": len(component_markup),
            "framework_component_distinct_keys": len(component_markup_by_key),
            "objects_that_open_a_dialog":
                sum(1 for o in objects_by_relpath.values() if o["inserts"]["opens_dialog"]),
            "objects_with_their_own_dialog_controls":
                sum(1 for o in objects_by_relpath.values() if o["own_dialog_control_count"]),
            "behaviors": len(behaviors),
            "behavior_parameter_controls_total":
                sum(b["parameter_dialog_control_count"] for b in behaviors),
            "behavior_shared_js_helpers": len(behavior_shared_js),
            "event_models": len(events),
            "event_model_tag_rows": sum(e["tag_count"] for e in events),
            "distinct_events_across_models": len({e for m in events
                                                  for e in m["distinct_events"]}),
        },
        "insertbar_parse_note": note,
        "insert_panel": categories,
        "responsive_frameworks": frameworks,
        "object_implementations": list(objects_by_relpath.values()),
        "object_implementations_not_on_the_insert_panel": unlinked,
        "behaviors": behaviors,
        "behavior_shared_js_helpers": behavior_shared_js,
        "event_models": events,
        "excluded_ai": C.excluded_ai(
            "Insert objects and behaviours",
            candidates=[c["category_id"] for c in categories]
                       + [c["category_label"] for c in categories]
                       + list(objects_by_relpath)
                       + [o["display_title"] for o in objects_by_relpath.values()]
                       + [b["behavior_file"] for b in behaviors]
                       + [b["display_title"] for b in behaviors]
                       + list(component_markup_by_key),
            extra_note="Checked every Insert category id and label, every object "
                       "file and title, every behaviour file and title, and every "
                       "responsive framework component key."),
        "failures": failures,
    })
    size = C.write_json(out_path, doc)
    return doc, size


if __name__ == "__main__":
    import json
    doc, size = build(sys.argv[1])
    print(json.dumps(doc["counts"], indent=1))
    print("failures:", len(doc["failures"]))
    for f in doc["failures"][:8]:
        print("  ", f)
    print("bytes:", size)
