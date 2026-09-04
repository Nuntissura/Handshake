"""dw_templates_css.py -- Task 6: templates, snippets, CSS designer, gradients,
media queries, Sass, starter assets.

Sources actually read (all offline):
  Configuration/Snippets/**/*.csn        -- every shipped snippet, with the exact
                                            text it inserts before/after selection
  Configuration/Snippets/dwSnippets.json -- the snippet keyboard-trigger map
  Configuration/VisualCSS/XML/CSSPropertiesData.xml -- the CSS Designer property
                                            surface: control type per property,
                                            default value, and the exact menu of
                                            values/units each property offers
  Configuration/VisualCSS/XML/UILayout.html -- the CSS Designer category grouping
  Configuration/VisualCSS/XML/CSSVendorTransforms.xml -- vendor prefix expansion
  Configuration/VisualCSS/CSSProperties/CSSProperties.xml -- the recognised property list
  Configuration/Css/Transitions/TransitionData.xml -- animatable properties and
                                            timing-function catalogue
  Configuration/MediaQuery/MediaFeaturesData.xml -- media features, their control
                                            type and their unit lists
  Configuration/VisualMediaQuery/**       -- the visual media query ruler model
  Configuration/GradientEditor/Gradient.html -- gradient swatch model
  Configuration/SVGOptions/options.json   -- SVG export defaults
  Configuration/Templates/**              -- shipped template/player assets
  Configuration/DocumentTypes/NewDocuments/** -- the starter documents
  Configuration/Responsive Starter Assets/** -- the shipped starter site
  Configuration/SassFrameworks/**         -- bundled Sass frameworks
  Configuration/Shared/LiveEdit/ResponsiveAssets/** -- bundled Bootstrap versions
"""
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import dw_common as C                                       # noqa: E402
from dw_zstrings import load_all_strings, resolve           # noqa: E402

JS_OBJ_RE = re.compile(r"var\s+(\w+)\s*=\s*\{(.*?)\n\}\s*;", re.S)
KV_RE = re.compile(r"['\"]?([\w\-]+)['\"]?\s*:\s*("
                   r"'(?:[^'\\]|\\.)*'|\"(?:[^\"\\]|\\.)*\"|[^,\n}]+)")


def build(out_path):
    exact, lower, smeta = load_all_strings(C.INSTALL_ROOT)
    failures = []

    def R(key):
        return resolve(key, exact, lower)

    def tree(el):
        rec = {"node": el.tag.split("}")[-1], "attributes": C.attrs_of(el)}
        t = (el.text or "").strip()
        if t:
            rec["text"] = t
        kids = [tree(c) for c in list(el)]
        if kids:
            rec["children"] = kids
        return rec

    def resolve_dollar(v):
        """'$$$/VCss/width' -> the shipped English tooltip, when present.

        The ZString table stores these keys with their leading slash intact
        ('/VCss/width'), so the slash-keeping form is tried first.
        """
        if isinstance(v, str) and v.startswith("$$$"):
            for key in (v[3:], v[4:] if v.startswith("$$$/") else None):
                if not key:
                    continue
                val, how = R(key)
                if val is not None:
                    return {"key": key, "value": val, "resolution": how}
            return {"key": v[3:], "value": None, "resolution": "unresolved"}
        return None

    # ---------------- snippets ---------------------------------------------
    sn_dir = os.path.join(C.CONFIG, "Snippets")
    snippets = []
    for p in sorted(C.walk(sn_dir, exts={".csn"})):
        r, note = C.parse_xml_tolerant(p)
        if r is None:
            failures.append({"stage": "snippet", "path": C.rel(p), "error": note})
            continue
        a = C.attrs_of(r)
        inserts = {}
        for el in r.iter():
            if el.tag.split("}")[-1] == "insertText":
                loc = C.attrs_of(el).get("location", "unspecified")
                inserts[loc] = (el.text or "")
        rel_folder = os.path.relpath(os.path.dirname(p), sn_dir).replace("\\", "/")
        snippets.append({
            "snippet_name": a.get("name"),
            "description": a.get("description"),
            "folder": rel_folder,
            "top_level_group": rel_folder.split("/")[0],
            "preview_mode": a.get("preview"),
            "snippet_type": a.get("type"),
            "inserts_before_selection": inserts.get("beforeSelection"),
            "inserts_after_selection": inserts.get("afterSelection"),
            "wraps_selection": bool((inserts.get("beforeSelection") or "").strip()
                                    and (inserts.get("afterSelection") or "").strip()),
            "all_insert_blocks": inserts,
            "file": C.rel(p),
            "all_attributes": a,
            "provenance": "parsed",
        })
    snippet_triggers = None
    sj = os.path.join(sn_dir, "dwSnippets.json")
    if os.path.isfile(sj):
        try:
            snippet_triggers = {"file": C.rel(sj),
                                "map": json.loads(C.read_text(sj)),
                                "provenance": "parsed"}
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "dwSnippets.json", "error": repr(exc)})

    # ---------------- CSS designer property surface -------------------------
    vc_dir = os.path.join(C.CONFIG, "VisualCSS")
    css_designer = []
    p = os.path.join(vc_dir, "XML", "CSSPropertiesData.xml")
    r, note = C.parse_xml_tolerant(p)
    if r is None:
        failures.append({"stage": "CSSPropertiesData", "error": note})
    else:
        for el in r.iter():
            if el.tag.split("}")[-1] != "property":
                continue
            a = C.attrs_of(el)
            items = []
            for it in el.iter():
                if it.tag.split("}")[-1] != "item":
                    continue
                ia = C.attrs_of(it)
                items.append({
                    "value": ia.get("name"),
                    "item_type": ia.get("type", "value"),
                    "is_separator": ia.get("type") == "separator",
                    "takes_a_numeric_buddy_value": ia.get("showbuddycontrol") == "true",
                    "all_attributes": ia,
                })
            css_designer.append({
                "property": a.get("name"),
                "display_name": a.get("displayname"),
                "control_type": a.get("controltype"),
                "default_value": a.get("defaultvalue"),
                "supports_negative_values": a.get("supportsnegativevalues"),
                "tooltip_key": a.get("tooltip"),
                "tooltip_text": (resolve_dollar(a.get("tooltip")) or {}).get("value"),
                "option_count": len([i for i in items if not i["is_separator"]]),
                "options": items,
                "all_attributes": a,
                "provenance": "parsed",
            })

    css_recognised = []
    p = os.path.join(vc_dir, "CSSProperties", "CSSProperties.xml")
    r, note = C.parse_xml_tolerant(p)
    if r is None:
        failures.append({"stage": "CSSProperties", "error": note})
    else:
        css_recognised = [C.attrs_of(el).get("name") for el in r.iter()
                          if el.tag.split("}")[-1] == "property"]

    # CSS Designer category grouping, read out of the shipped UILayout table
    css_categories = []
    p = os.path.join(vc_dir, "XML", "UILayout.html")
    if os.path.isfile(p):
        txt = C.read_text(p)
        cur = None
        for m in re.finditer(r"<td\b([^>]*)>(.*?)</td>", txt, re.S | re.I):
            attrs = C.parse_attrs(m.group(1))
            body = re.sub(r"<[^>]+>", "", m.group(2)).strip()
            if attrs.get("class"):
                res = resolve_dollar(body)
                cur = {"category_id": attrs["class"],
                       "category_label_key": body if body.startswith("$$$/") else None,
                       "category_label": (res or {}).get("value") or body,
                       "declared_row_span": attrs.get("rowspan"),
                       "properties": []}
                css_categories.append(cur)
            elif cur is not None and body:
                for part in body.split(","):
                    part = part.strip()
                    if part:
                        cur["properties"].append(part)
        for c in css_categories:
            c["property_count"] = len(c["properties"])
            c["provenance"] = "parsed from the shipped UILayout.html category table"

    vendor_transforms = []
    p = os.path.join(vc_dir, "XML", "CSSVendorTransforms.xml")
    r, note = C.parse_xml_tolerant(p)
    if r is None:
        failures.append({"stage": "CSSVendorTransforms", "error": note})
    else:
        for el in r.iter():
            if el.tag.split("}")[-1] != "Transform":
                continue
            a = C.attrs_of(el)
            rules = []
            for f in list(el):
                fa = C.attrs_of(f)
                rules.append({
                    "when_value": fa.get("value"),
                    "emits": [C.attrs_of(t) for t in list(f)],
                    "all_attributes": fa,
                })
            vendor_transforms.append({"property": a.get("property"),
                                      "rule_count": len(rules), "rules": rules,
                                      "provenance": "parsed"})

    # ---------------- CSS transitions ---------------------------------------
    transitions = None
    p = os.path.join(C.CONFIG, "Css", "Transitions", "TransitionData.xml")
    r, note = C.parse_xml_tolerant(p)
    if r is None:
        failures.append({"stage": "TransitionData", "error": note})
    else:
        groups = {}
        for el in list(r):
            t = el.tag.split("}")[-1]
            groups[t] = [(c.text or "").strip() for c in list(el) if (c.text or "").strip()]
        transitions = {"file": C.rel(p),
                       "groups": {k: {"count": len(v), "values": v}
                                  for k, v in groups.items()},
                       "provenance": "parsed"}

    # ---------------- media queries -----------------------------------------
    media = None
    p = os.path.join(C.CONFIG, "MediaQuery", "MediaFeaturesData.xml")
    r, note = C.parse_xml_tolerant(p)
    if r is None:
        failures.append({"stage": "MediaFeaturesData", "error": note})
    else:
        feats, lists = [], {}
        for el in r.iter():
            t = el.tag.split("}")[-1]
            a = C.attrs_of(el)
            if t == "mediafeature":
                feats.append({"feature": a.get("name"), "control_type": a.get("type"),
                              "value_list": a.get("list"), "tooltip": a.get("tooltip"),
                              "all_attributes": a})
            elif a.get("type") == "list" or t.endswith("s") and list(el):
                vals = [(c.text or "").strip() or C.attrs_of(c).get("name")
                        for c in list(el)]
                vals = [v for v in vals if v]
                if vals:
                    lists[t] = vals
        for f in feats:
            if f["value_list"] and f["value_list"] in lists:
                f["allowed_values"] = lists[f["value_list"]]
        media = {"file": C.rel(p), "feature_count": len(feats), "features": feats,
                 "value_lists": lists, "provenance": "parsed"}

    vmq_constants = {}
    vmq_dir = os.path.join(C.CONFIG, "VisualMediaQuery")
    vp = os.path.join(vmq_dir, "js", "vmqconstants.js")
    if os.path.isfile(vp):
        src = C.read_text(vp)
        for m in JS_OBJ_RE.finditer(src):
            obj = {}
            for km in KV_RE.finditer(m.group(2)):
                v = km.group(2).strip().rstrip(",").strip()
                obj[km.group(1)] = v.strip("'\"")
            vmq_constants[m.group(1)] = obj
    vmq_files = sorted(C.rel(x) for x in C.walk(vmq_dir))

    # ---------------- gradient editor ---------------------------------------
    gradient = None
    p = os.path.join(C.CONFIG, "GradientEditor", "Gradient.html")
    if os.path.isfile(p):
        src = C.read_text(p)
        js, _ = C.extract_js(src, os.path.dirname(p))
        nums = dict(re.findall(r"var\s+(\w+)\s*=\s*(\d+)\s*;", js))
        gradient = {
            "file": C.rel(p),
            "js_functions": [f["name"] for f in C.js_functions(js)],
            "numeric_constants": nums,
            "max_saved_swatches": nums.get("maxNumberOfSwatches"),
            "provenance": "parsed",
            "note": "Gradient.html is the saved-swatch strip only. The gradient "
                    "stop editor itself is native; its parameters appear in the "
                    "CSS Designer property surface under background-image.",
        }

    svg_options = None
    p = os.path.join(C.CONFIG, "SVGOptions", "options.json")
    if os.path.isfile(p):
        try:
            svg_options = {"file": C.rel(p), "defaults": json.loads(C.read_text(p)),
                           "provenance": "parsed"}
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "SVGOptions", "error": repr(exc)})

    # ---------------- templates and starter documents ------------------------
    templates = []
    tdir = os.path.join(C.CONFIG, "Templates")
    for p in sorted(C.walk(tdir)):
        ext = os.path.splitext(p)[1].lower()
        rec = {"file": C.rel(p), "bytes": os.path.getsize(p),
               "asset_kind": {".html": "page template", ".htm": "page template",
                              ".swf": "shipped Flash video asset",
                              ".png": "image", ".gif": "image"}.get(ext, ext or "unknown")}
        if ext in (".html", ".htm") and rec["bytes"] < 100000:
            rec["content"] = C.read_text(p)
        templates.append(rec)

    starter_docs = []
    ndir = os.path.join(C.CONFIG, "DocumentTypes", "NewDocuments")
    for p in sorted(C.walk(ndir)):
        rec = {"file": C.rel(p), "name": os.path.basename(p),
               "bytes": os.path.getsize(p)}
        if rec["bytes"] < 60000:
            rec["content"] = C.read_text(p)
        starter_docs.append(rec)

    starter_assets = []
    sadir = os.path.join(C.CONFIG, "Responsive Starter Assets")
    for p in sorted(C.walk(sadir)):
        rec = {"file": C.rel(p), "bytes": os.path.getsize(p),
               "starter_site": os.path.relpath(p, sadir).split(os.sep)[0]}
        if os.path.splitext(p)[1].lower() in (".css", ".html", ".htm") \
                and rec["bytes"] < 200000:
            rec["content"] = C.read_text(p)
        starter_assets.append(rec)

    sass = []
    sdir = os.path.join(C.CONFIG, "SassFrameworks")
    for p in sorted(C.walk(sdir, exts={".scss", ".sass"})):
        sass.append({"file": C.rel(p),
                     "framework": os.path.relpath(p, sdir).split(os.sep)[0],
                     "bytes": os.path.getsize(p)})
    sass_frameworks = {}
    for s in sass:
        sass_frameworks.setdefault(s["framework"], 0)
        sass_frameworks[s["framework"]] += 1

    bootstrap = []
    bdir = os.path.join(C.CONFIG, "Shared", "LiveEdit", "ResponsiveAssets")
    bversion = None
    bvp = os.path.join(bdir, "bootstrapVersionData.json")
    if os.path.isfile(bvp):
        try:
            bversion = json.loads(C.read_text(bvp))
        except Exception as exc:                            # noqa: BLE001
            failures.append({"stage": "bootstrapVersionData", "error": repr(exc)})
    if os.path.isdir(bdir):
        for d in sorted(os.listdir(bdir)):
            fp = os.path.join(bdir, d)
            if os.path.isdir(fp):
                bootstrap.append({
                    "bundle": d,
                    "file_count": sum(1 for _ in C.walk(fp)),
                    "path": C.rel(fp),
                })

    method = {
        "task": "6 - templates, snippets, CSS designer, gradients, media queries",
        "how": [
            "Every .csn snippet file is parsed to its exact insertText blocks, so "
            "the export carries the literal text each snippet writes, not just its "
            "name. beforeSelection + afterSelection together mean the snippet "
            "wraps the selection; that flag is derived and marked as such.",
            "CSSPropertiesData.xml is the CSS Designer's own property surface: for "
            "each property it gives the control type, the default value, whether "
            "negatives are allowed and the exact ordered menu of values and units "
            "the panel offers, including which entries take a numeric buddy value. "
            "All of that is read literally.",
            "UILayout.html is the shipped table that assigns each property to a "
            "CSS Designer category; the category label keys are resolved against "
            "the string table.",
            "CSSVendorTransforms.xml gives the exact vendor-prefix fan-out rules.",
            "MediaFeaturesData.xml gives each media feature its control type and, "
            "where it names a list, the unit or option list it draws from.",
        ],
        "not_done": [
            "Snippet bodies are stored verbatim including whitespace; they are not "
            "normalised, because indentation is part of what the snippet inserts.",
            "The gradient stop editor and the CSS Designer panel chrome are native "
            "surfaces; only their declarative data files are recoverable here.",
        ],
        "string_tables": smeta,
    }

    doc = C.envelope("handshake.studio.dreamweaver.templates_css.v1", method, {
        "counts": {
            "snippets": len(snippets),
            "snippet_groups": len({s["top_level_group"] for s in snippets}),
            "snippets_that_wrap_the_selection": sum(1 for s in snippets if s["wraps_selection"]),
            "snippets_block_type": sum(1 for s in snippets if s["snippet_type"] == "block"),
            "snippets_wrap_type": sum(1 for s in snippets if s["snippet_type"] == "wrap"),
            "snippet_keyboard_triggers": len((snippet_triggers or {}).get("map", {})
                                             .get("snippets", {})),
            "css_designer_properties": len(css_designer),
            "css_designer_property_option_entries": sum(p["option_count"] for p in css_designer),
            "css_designer_control_types": sorted({p["control_type"] for p in css_designer
                                                  if p["control_type"]}),
            "css_designer_categories": len(css_categories),
            "css_designer_category_property_slots": sum(c["property_count"]
                                                        for c in css_categories),
            "css_properties_recognised": len(css_recognised),
            "css_vendor_transform_properties": len(vendor_transforms),
            "css_vendor_transform_rules": sum(t["rule_count"] for t in vendor_transforms),
            "transition_groups": {k: v["count"] for k, v in
                                  (transitions or {"groups": {}})["groups"].items()},
            "media_features": (media or {}).get("feature_count"),
            "media_value_lists": len((media or {}).get("value_lists", {})),
            "visual_media_query_constant_groups": len(vmq_constants),
            "shipped_template_assets": len(templates),
            "starter_documents": len(starter_docs),
            "responsive_starter_asset_files": len(starter_assets),
            "sass_files": len(sass),
            "sass_frameworks": sass_frameworks,
            "bootstrap_bundles": [b["bundle"] for b in bootstrap],
        },
        "snippets": snippets,
        "snippet_keyboard_triggers": snippet_triggers,
        "css_designer_property_surface": css_designer,
        "css_designer_categories": css_categories,
        "css_properties_recognised": css_recognised,
        "css_vendor_transforms": vendor_transforms,
        "css_transitions": transitions,
        "media_query_model": media,
        "visual_media_query": {
            "constants": vmq_constants,
            "files": vmq_files,
            "provenance": "parsed: the ruler/breakpoint limits are the literal "
                          "numeric constants shipped in vmqconstants.js",
        },
        "gradient_editor": gradient,
        "svg_export_defaults": svg_options,
        "shipped_templates": templates,
        "starter_documents": starter_docs,
        "responsive_starter_assets": starter_assets,
        "sass_frameworks": sass,
        "bootstrap_bundles": bootstrap,
        "bootstrap_version_data": bversion,
        "excluded_ai": C.excluded_ai(
            "templates, snippets, CSS designer and media queries",
            candidates=[s["snippet_name"] for s in snippets]
                       + [s["description"] for s in snippets]
                       + [p["property"] for p in css_designer]
                       + [p["control_type"] for p in css_designer]
                       + [f["feature"] for f in (media or {"features": []})["features"]],
            extra_note="Checked every snippet name and description, every CSS "
                       "Designer property and control type, and every media feature."),
        "failures": failures,
    })
    size = C.write_json(out_path, doc)
    return doc, size


if __name__ == "__main__":
    doc, size = build(sys.argv[1])
    print(json.dumps(doc["counts"], indent=1))
    print("failures:", len(doc["failures"]))
    for f in doc["failures"][:10]:
        print("  ", f)
    print("bytes:", size)
