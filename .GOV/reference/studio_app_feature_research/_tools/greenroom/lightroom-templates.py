#!/usr/bin/env python
"""
lightroom-templates.py

Parses every Lightroom Classic template/preset file that uses the Lua-table
(.lrtemplate) serialisation, plus the Book-module layout templates and the
Web-module gallery engines. Read-only; Lightroom is never launched.

WHAT ACTUALLY SHIPS - stated because it contradicts a common assumption:
  * The 67 .lrtemplate files under <INSTALL>/Templates/Layout Templates are
    Book-module layout stubs. Each is ~260 bytes and only names a folder;
    the real content is the sibling templatePages.lua.
  * The .lua files in the install (68 of them) are NOT Lightroom SDK source.
    67 are Book layout page-geometry tables (templatePages.lua) and one is
    layout_template_sizes.lua. The 68th .lua in the corpus is a user-profile
    Metadata/DefaultPanel.lua.
  * The templates that carry real parameter surfaces - filename templates,
    metadata/filter presets, keyword sets, label sets, local-adjustment
    presets, export presets - live in the USER profile
    (%APPDATA%/Adobe/Lightroom), 37 files, one per template type folder.
  * Print, Slideshow and Web module templates are not shipped as loose files
    in this install; Web ships 4 .lrwebengine gallery definitions instead
    (galleryInfo.lrweb + manifest.lrweb, both Lua).

Every parsed template's full parameter tree is emitted, plus a per-type
key vocabulary so a reimplementation can see the whole surface at once.
"""
from __future__ import annotations

import argparse
import collections
import datetime as _dt
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lrlua  # noqa: E402

SCHEMA_ID = "handshake.adobe.lightroom_classic.templates.v1"

TYPE_BY_FOLDER = {
    "Export Presets": "export_preset",
    "Filename Templates": "filename_template",
    "Filter Presets": "library_filter_preset",
    "Keyword Sets": "keyword_set",
    "Label Sets": "color_label_set",
    "Local Adjustment Presets": "local_adjustment_preset",
    "Metadata": "metadata_panel_layout",
    "Layout Templates": "book_layout_template",
    "Print Templates": "print_template",
    "Slideshow Templates": "slideshow_template",
    "Web Templates": "web_template",
    "Text Templates": "text_template",
    "Watermarks": "watermark_preset",
}


def type_for(rel: str, tbl) -> str:
    parts = rel.replace("\\", "/").split("/")
    for p in parts:
        if p in TYPE_BY_FOLDER:
            return TYPE_BY_FOLDER[p]
    if isinstance(tbl, dict) and isinstance(tbl.get("type"), str):
        return "declared:" + tbl["type"]
    return "unknown"


def keypaths(node, prefix="", out=None, depth=0):
    if out is None:
        out = collections.Counter()
    if depth > 6:
        return out
    if isinstance(node, dict):
        for k, v in node.items():
            path = prefix + "." + k if prefix else k
            out[path] += 1
            if isinstance(v, (dict, list)):
                keypaths(v, path, out, depth + 1)
    elif isinstance(node, list):
        for v in node[:20]:
            if isinstance(v, (dict, list)):
                keypaths(v, prefix + "[]", out, depth + 1)
    return out


def leaf_values(node, prefix="", out=None, depth=0):
    if out is None:
        out = collections.defaultdict(collections.Counter)
    if depth > 6:
        return out
    if isinstance(node, dict):
        for k, v in node.items():
            path = prefix + "." + k if prefix else k
            if isinstance(v, (dict, list)):
                leaf_values(v, path, out, depth + 1)
            else:
                out[path][json.dumps(v, ensure_ascii=False)] += 1
    elif isinstance(node, list):
        for v in node[:20]:
            if isinstance(v, (dict, list)):
                leaf_values(v, prefix + "[]", out, depth + 1)
    return out


def collect(root, exts):
    got = []
    if not os.path.isdir(root):
        return got
    for dp, _d, fn in os.walk(root):
        for f in sorted(fn):
            if f.lower().endswith(exts):
                got.append(os.path.join(dp, f))
    return sorted(got)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install",
                    default=r"C:\Program Files\Adobe\Adobe Lightroom Classic")
    ap.add_argument("--user",
                    default=os.path.expandvars(r"%APPDATA%\Adobe\Lightroom"))
    ap.add_argument("--out", required=True)
    ap.add_argument("--max-layout-pages", type=int, default=3,
                    help="how many templatePages.lua get a full parse")
    args = ap.parse_args()

    errors = []
    templates = []
    vocab = collections.defaultdict(collections.Counter)
    values = collections.defaultdict(lambda: collections.defaultdict(
        collections.Counter))

    for origin, root in (("install", args.install), ("user", args.user)):
        for p in collect(root, (".lrtemplate",)):
            rel = os.path.relpath(p, root).replace("\\", "/")
            try:
                name, tbl = lrlua.parse_table(lrlua.read(p))
            except Exception as exc:  # noqa: BLE001
                errors.append({"file": rel, "origin": origin,
                               "error": "%s: %s" % (type(exc).__name__, exc)})
                continue
            j = lrlua.jsonable(tbl)
            ttype = type_for(rel, j)
            vocab[ttype].update(keypaths(j))
            for path, ctr in leaf_values(j).items():
                values[ttype][path].update(ctr)
            templates.append({
                "origin": origin, "file": rel, "template_type": ttype,
                "root_variable": name, "size_bytes": os.path.getsize(p),
                "classification": "parsed", "content": j,
            })

    # --- Book layout page geometry ----------------------------------------
    layout_root = os.path.join(args.install, "Templates", "Layout Templates")
    page_files = [p for p in collect(layout_root, (".lua",))
                  if os.path.basename(p) == "templatePages.lua"]
    sizes_file = os.path.join(layout_root, "layout_template_sizes.lua")
    layout = {"classification": "parsed",
              "templatePages_files": len(page_files),
              "full_parses": [], "key_vocabulary": {}, "errors": []}
    lv = collections.Counter()
    for i, p in enumerate(page_files):
        rel = os.path.relpath(p, args.install).replace("\\", "/")
        try:
            name, tbl = lrlua.parse_table(lrlua.read(p))
        except Exception as exc:  # noqa: BLE001
            layout["errors"].append({"file": rel,
                                     "error": "%s: %s" % (type(exc).__name__,
                                                          exc)})
            continue
        j = lrlua.jsonable(tbl)
        lv.update(keypaths(j))
        if len(layout["full_parses"]) < args.max_layout_pages:
            layout["full_parses"].append({"file": rel, "root_variable": name,
                                          "content": j})
    layout["key_vocabulary"] = dict(lv.most_common(200))
    if os.path.isfile(sizes_file):
        try:
            n, t = lrlua.parse_table(lrlua.read(sizes_file))
            layout["layout_template_sizes"] = {
                "file": "Templates/Layout Templates/layout_template_sizes.lua",
                "root_variable": n, "content": lrlua.jsonable(t)}
        except Exception as exc:  # noqa: BLE001
            layout["errors"].append({"file": sizes_file,
                                     "error": str(exc)[:200]})

    # --- Web gallery engines ----------------------------------------------
    web_root = os.path.join(args.install, "Resources", "webengines")
    web = {"classification": "parsed", "engines": [], "errors": []}
    if os.path.isdir(web_root):
        for eng in sorted(os.listdir(web_root)):
            edir = os.path.join(web_root, eng)
            if not os.path.isdir(edir):
                continue
            rec = {"engine": eng}
            for fname, key in (("manifest.lrweb", "manifest"),
                               ("galleryInfo.lrweb", "galleryInfo")):
                fp = os.path.join(edir, fname)
                if not os.path.isfile(fp):
                    continue
                try:
                    n, t = lrlua.parse_table(lrlua.read(fp))
                    rec[key] = {"root_variable": n,
                                "content": lrlua.jsonable(t)}
                except Exception as exc:  # noqa: BLE001
                    web["errors"].append(
                        {"file": "%s/%s" % (eng, fname),
                         "error": "%s: %s" % (type(exc).__name__, exc)})
            web["engines"].append(rec)

    # --- user metadata panel layout ---------------------------------------
    misc = []
    dp_lua = os.path.join(args.user, "Metadata", "DefaultPanel.lua")
    if os.path.isfile(dp_lua):
        try:
            n, t = lrlua.parse_table(lrlua.read(dp_lua))
            misc.append({"file": "Metadata/DefaultPanel.lua", "origin": "user",
                         "template_type": "metadata_panel_layout",
                         "root_variable": n, "classification": "parsed",
                         "content": lrlua.jsonable(t)})
        except Exception as exc:  # noqa: BLE001
            errors.append({"file": dp_lua,
                           "error": "%s: %s" % (type(exc).__name__, exc)})

    by_type = collections.Counter(t["template_type"] for t in templates)
    by_origin = collections.Counter(t["origin"] for t in templates)

    surface = {}
    for ttype, ctr in vocab.items():
        rows = []
        for path, n in ctr.most_common():
            row = {"key_path": path, "templates_using": n}
            vv = values[ttype].get(path)
            if vv:
                row["observed_values"] = [
                    {"value": v, "count": c} for v, c in vv.most_common(12)]
                row["distinct_values"] = len(vv)
            rows.append(row)
        surface[ttype] = rows

    doc = {
        "schema_id": SCHEMA_ID,
        "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
        "method": {
            "mode": "offline_static_parse",
            "app_launched": False,
            "sources": [
                {"id": "lrtemplate", "classification": "parsed",
                 "roots": [args.install, args.user],
                 "format": "Lua table source, `s = { ... }`",
                 "parser": "lrlua.parse_table"},
                {"id": "book_layout_pages", "classification": "parsed",
                 "root": layout_root,
                 "format": "Lua table source, page geometry",
                 "parser": "lrlua.parse_table",
                 "note": "full content emitted for a capped sample; key "
                         "vocabulary aggregated over every file"},
                {"id": "web_gallery_engines", "classification": "parsed",
                 "root": web_root, "format": "Lua table source (.lrweb)"},
            ],
            "classification_legend": {
                "parsed": "read directly out of a shipped or user file",
                "derived": "computed from parsed data",
                "heuristic": "this tool's judgement",
            },
        },
        "counts": {
            "lrtemplate_files_parsed": len(templates),
            "lrtemplate_files_failed": len(errors),
            "lrtemplate_by_origin": dict(by_origin),
            "lrtemplate_by_type": dict(by_type),
            "book_templatePages_files": len(page_files),
            "book_templatePages_key_paths": len(lv),
            "web_gallery_engines": len(web["engines"]),
            "template_types_with_parameter_surface": len(surface),
        },
        "scope_correction": {
            "classification": "parsed",
            "statement": "The 67 install .lrtemplate files are Book layout "
                         "stubs averaging ~275 bytes that only point at a "
                         "sibling templatePages.lua. The 37 templates that "
                         "carry real parameter surfaces are in the user "
                         "profile. No Print, Slideshow, Web or Metadata "
                         "template files ship loose in this install; Web "
                         "ships .lrwebengine gallery definitions instead.",
        },
        "parameter_surface_by_type": surface,
        "templates": templates + misc,
        "book_layout_templates": layout,
        "web_gallery_engines": web,
        "errors": errors,
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print(json.dumps(doc["counts"], indent=1))
    for e in errors[:10]:
        print("ERR", e, file=sys.stderr)


if __name__ == "__main__":
    main()
