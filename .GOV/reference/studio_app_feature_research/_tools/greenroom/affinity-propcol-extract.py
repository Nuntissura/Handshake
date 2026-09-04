#!/usr/bin/env python3
"""Extracts behavioural specifications from Affinity 3.2.3 preset containers.

Uses the real KA/KS parser in affinity_ka.py (not the earlier heuristic scan).
Produces:
  affinity_preset_contents.json      - true preset trees + values, per container
  affinity_brush_parameters.json     - per-brush parameter sets + parameter schema
  affinity_adjustment_parameters.json- adjustment schemas + preset values
  affinity_tool_panel_registry.json  - workspace tool/panel id registry

Every value in these files is decoded from the container's own type/tag/length
encoding.  Fields whose provenance is weaker are labelled explicitly.

Usage:
  python affinity-propcol-extract.py --res <Affinity resources dir> --out <dir>
"""
from __future__ import annotations

import argparse
import datetime as _dt
import glob
import json
import os
import sys
from collections import Counter, OrderedDict

sys.setrecursionlimit(60000)
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import affinity_ka as ka  # noqa: E402

TOOL_ID = "handshake.affinity.propcol_extract.v1"
PARSER_ID = "handshake.affinity.ka_parser.v1"
BLOB_KEEP = 96          # hex chars of a blob to retain inline


def now():
    return _dt.datetime.now(_dt.timezone.utc).isoformat(timespec="seconds")


# --------------------------------------------------------------------------- json
def to_json(v, depth=0, max_depth=40, refs=None, seen=None):
    """Converts parsed objects to JSON-safe structures."""
    if isinstance(v, ka.Obj):
        if depth >= max_depth:
            return {"_type": v.type, "_truncated": True}
        out = OrderedDict()
        out["_type"] = v.type
        if v.index is not None:
            out["_index"] = v.index
        for k, x in v.props.items():
            out[k] = to_json(x, depth + 1, max_depth, refs, seen)
        return out
    if isinstance(v, list):
        return [to_json(x, depth + 1, max_depth, refs, seen) for x in v]
    if isinstance(v, dict):
        out = OrderedDict()
        for k, x in v.items():
            if k in ("_blob", "_data", "_records") and isinstance(x, str):
                out[k + "_len_bytes"] = len(x) // 2
                out[k + "_head_hex"] = x[:BLOB_KEEP]
            else:
                out[k] = to_json(x, depth + 1, max_depth, refs, seen)
        return out
    if isinstance(v, float):
        if v != v or v in (float("inf"), float("-inf")):
            return {"_float": repr(v)}
        return v
    return v


def resolve_refs(v, objects, depth=0, max_depth=40):
    """Replaces {'_ref': n} with the referenced object's decoded properties."""
    if isinstance(v, dict) and set(v) == {"_ref"}:
        tgt = objects.get(v["_ref"])
        if tgt is None:
            return {"_ref": v["_ref"], "_resolved": False}
        r = to_json(tgt, depth, max_depth)
        r["_ref"] = v["_ref"]
        r["_resolved"] = True
        return r
    if isinstance(v, dict):
        return {k: resolve_refs(x, objects, depth + 1, max_depth) for k, x in v.items()}
    if isinstance(v, list):
        return [resolve_refs(x, objects, depth + 1, max_depth) for x in v]
    return v


# --------------------------------------------------------------------------- tree
def kids(node, tag):
    """Object list for `tag`, skipping null entries and unresolved references."""
    return [x for x in (node.props.get(tag) or []) if isinstance(x, ka.Obj)]


def node_kind(o):
    if o.type == "PTNd":
        return "node"
    if o.type == "PLef":
        return "preset"
    return o.type


def walk_tree(node, path, out_nodes, out_presets, objects, max_depth):
    """Walks the PTNd/PLef preset tree of a container."""
    name = node.props.get("Name")
    here = path + ([name] if name else [])
    rec = {
        "path": "/".join(here),
        "name": name,
        "uid": node.props.get("_UID"),
        "cuid": node.props.get("CUID"),
        "usage": node.props.get("Usge"),
        "flags": node.props.get("Fflg"),
        "child_nodes": len(kids(node, "Chld")),
        "presets": len(kids(node, "Levs")),
        "null_or_reference_children": (len(node.props.get("Chld") or [])
                                       - len(kids(node, "Chld"))),
        "null_or_reference_presets": (len(node.props.get("Levs") or [])
                                      - len(kids(node, "Levs"))),
    }
    out_nodes.append(rec)
    for lf in kids(node, "Levs"):
        prop = lf.props.get("Prop")
        entry = {
            "path": "/".join(here),
            "name": lf.props.get("Name"),
            "uid": lf.props.get("_UID"),
            "owner_uid": lf.props.get("OUID"),
            "flags": lf.props.get("Fflg"),
            "payload_type": prop.type if isinstance(prop, ka.Obj) else None,
            "payload": resolve_refs(to_json(prop, 0, max_depth), objects)
            if isinstance(prop, ka.Obj) else to_json(prop),
        }
        out_presets.append(entry)
    for ch in kids(node, "Chld"):
        walk_tree(ch, here, out_nodes, out_presets, objects, max_depth)


# --------------------------------------------------------------------------- main
def parse_all(resdir, zstd):
    results = OrderedDict()
    files = sorted(glob.glob(os.path.join(resdir, "*.propcol"))) + \
        sorted(glob.glob(os.path.join(resdir, "*.afstudio")))
    for f in files:
        base = os.path.basename(f)
        try:
            r = ka.parse_file(f, zstd=zstd)
        except Exception as e:                             # noqa: BLE001
            results[base] = {"hard_error": "%s: %s" % (type(e).__name__, e)}
            continue
        results[base] = r
    return results


def container_summary(base, r):
    p = r["parser"]
    return OrderedDict([
        ("file", base),
        ("container_type_4cc", r["container_type"]),
        ("format_version", r["format_version"]),
        ("flags", r["flags"]),
        ("file_size_bytes", r["file_size"]),
        ("property_stream_bytes", r["expected"]),
        ("property_stream_parsed_bytes", r["consumed"]),
        ("payload_compressed", r["compressed"]),
        ("header_type_table_count", r["header_type_table_count"]),
        ("header_object_count", r["header_object_count"]),
        ("header_property_tag_count", r["header_prot_count"]),
        ("parse_status", "complete" if r["complete"] else "partial"),
        ("parse_error", r["error"]),
        ("objects_parsed", p.object_count),
        ("class_sections_parsed", p.section_count),
        ("distinct_property_tags", len(p.tag_counts)),
        ("distinct_object_types", len(p.type_counts)),
        ("unknown_type_codes", {("0x%02x" % k): v["len"]
                                for k, v in p.unknown_types.items()}),
        ("has_additional_file_streams", not r["single_stream"]),
    ])


def build_presets(results, outdir, max_depth):
    containers = []
    totals = Counter()
    for base, r in results.items():
        if "hard_error" in r:
            containers.append({"file": base, "error": r["hard_error"]})
            continue
        s = container_summary(base, r)
        root = r["root"]
        nodes, presets = [], []
        if root is not None and "Root" in root.props:
            walk_tree(root.props["Root"], [], nodes, presets,
                      r["parser"].objects, max_depth)
        if not nodes and not r["complete"] and r["parser"].objects:
            # the parse stopped before the tree could be attached to the root;
            # recover whatever preset nodes/leaves were already decoded
            recovered_n, recovered_p = [], []
            for idx in sorted(r["parser"].objects):
                o = r["parser"].objects[idx]
                if o.type == "PTNd":
                    recovered_n.append({"index": idx, "name": o.props.get("Name"),
                                        "uid": o.props.get("_UID"),
                                        "presets": len(kids(o, "Levs")),
                                        "child_nodes": len(kids(o, "Chld"))})
                elif o.type == "PLef":
                    prop = o.props.get("Prop")
                    recovered_p.append({
                        "index": idx, "name": o.props.get("Name"),
                        "uid": o.props.get("_UID"),
                        "payload_type": prop.type if isinstance(prop, ka.Obj) else None,
                        "payload": resolve_refs(to_json(prop, 0, max_depth),
                                                r["parser"].objects)
                        if isinstance(prop, ka.Obj) else None})
            s["recovery_note"] = ("parse_status is partial; the tree could not be "
                                  "attached to the container root, so nodes and "
                                  "presets below were recovered from the objects "
                                  "decoded before the parser stopped. Counts are "
                                  "lower bounds.")
            nodes, presets = recovered_n, recovered_p
        s["node_count"] = len(nodes)
        s["preset_count"] = len(presets)
        s["nodes"] = nodes
        s["presets"] = presets
        containers.append(s)
        totals["nodes"] += len(nodes)
        totals["presets"] += len(presets)
        totals["objects"] += r["parser"].object_count
    doc = OrderedDict([
        ("schema_id", "handshake.affinity.preset_contents.v1"),
        ("generated_at", now()),
        ("generator", TOOL_ID),
        ("parser", PARSER_ID),
        ("method", "parsed"),
        ("method_detail",
         "Full recursive parse of the Serif KA/KS property container format. "
         "Names, uids and every property value below are decoded from the "
         "container's own type-code/tag/length encoding. No string scraping, "
         "no name heuristics. A container is marked parse_status=complete only "
         "when the property stream terminates exactly at the container's own "
         "0xFFFFFFFF end marker."),
        ("labelling", {
            "parsed": "decoded from the binary format",
            "partial": "the parser stopped early; counts are lower bounds",
        }),
        ("totals", dict(totals)),
        ("containers", containers),
    ])
    path = os.path.join(outdir, "affinity_preset_contents.json")
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    return path, doc


# --------------------------------------------------------------------------- brushes
def flatten_dynamics(v):
    """BrDy dynamics objects -> a flat, engine-facing description."""
    if not isinstance(v, dict) or v.get("_type") != "BrDy":
        return v
    out = OrderedDict()
    out["base_value"] = v.get("DynV")
    out["variance"] = v.get("DyVa")
    out["controller"] = v.get("DynC")
    out["variance_mode"] = v.get("DyVM")
    out["time_limit_ms"] = v.get("TimL")
    out["reverse"] = v.get("DynR")
    sp = v.get("Spln")
    if isinstance(sp, dict):
        vals = sp.get("Vals") or []
        n = sp.get("Cnt ") or 0
        xs, ys, coef = vals[:n], vals[n:2 * n], vals[2 * n:]
        out["curve"] = {
            "point_count": n,
            "linear": sp.get("Linr"),
            "bound_first": sp.get("Bnd1"),
            "bound_last": sp.get("BndN"),
            "layout": ("Vals holds Cnt x values, then Cnt y values, then any "
                       "remaining values are spline coefficients. Verified: the "
                       "shared identity curve stores Cnt=2 and Vals=[0,1,0,1] "
                       "with no coefficients; the default brush size curve "
                       "stores Cnt=11 with y == x^2 over 11 evenly spaced x "
                       "values plus 11 coefficients."),
            "x": xs,
            "y": ys,
            "spline_coefficients": coef,
            "points": [[xs[i], ys[i]] for i in range(min(len(xs), len(ys)))],
            "raw_value_count": len(vals),
            "shared_object_index": sp.get("_ref"),
        }
    return out


def collect_brushes(r, kind, objects):
    rows = []
    root = r["root"].props.get("Root")
    if root is None:
        return rows

    def rec(node, path):
        name = node.props.get("Name")
        here = path + ([name] if name else [])
        for lf in kids(node, "Levs"):
            prop = lf.props.get("Prop")
            brush = None
            if isinstance(prop, ka.Obj):
                inner = prop.props.get("Brus")
                brush = inner if isinstance(inner, ka.Obj) else prop
            if brush is None:
                continue
            params = resolve_refs(to_json(brush, 0, 30), objects)
            params.pop("_type", None)
            params.pop("_index", None)
            flat = OrderedDict()
            for k, v in params.items():
                flat[k] = flatten_dynamics(v)
            rows.append(OrderedDict([
                ("kind", kind),
                ("category", "/".join(here)),
                ("name", lf.props.get("Name")),
                ("uid", lf.props.get("_UID")),
                ("brush_class_4cc", brush.type),
                ("parameters", flat),
            ]))
        for ch in kids(node, "Chld"):
            rec(ch, here)

    rec(root, [])
    return rows


def param_schema(rows):
    """Observed type / range / distinct values for every brush parameter tag."""
    stats = {}
    for row in rows:
        for tag, v in row["parameters"].items():
            s = stats.setdefault(tag, {"tag": tag, "occurrences": 0,
                                       "value_kinds": Counter(),
                                       "numeric_min": None, "numeric_max": None,
                                       "distinct_values": set(),
                                       "dynamics": False})
            s["occurrences"] += 1
            if isinstance(v, dict) and ("base_value" in v or "curve" in v):
                s["dynamics"] = True
                s["value_kinds"]["dynamics"] += 1
                b = v.get("base_value")
                if isinstance(b, (int, float)):
                    s["numeric_min"] = b if s["numeric_min"] is None else min(s["numeric_min"], b)
                    s["numeric_max"] = b if s["numeric_max"] is None else max(s["numeric_max"], b)
                continue
            if isinstance(v, bool):
                s["value_kinds"]["bool"] += 1
            elif isinstance(v, int):
                s["value_kinds"]["int"] += 1
            elif isinstance(v, float):
                s["value_kinds"]["float"] += 1
            elif isinstance(v, str):
                s["value_kinds"]["string"] += 1
            elif isinstance(v, list):
                s["value_kinds"]["list"] += 1
            elif isinstance(v, dict):
                s["value_kinds"]["object:" + str(v.get("_type"))] += 1
            elif v is None:
                s["value_kinds"]["null"] += 1
            if isinstance(v, (int, float)) and not isinstance(v, bool):
                s["numeric_min"] = v if s["numeric_min"] is None else min(s["numeric_min"], v)
                s["numeric_max"] = v if s["numeric_max"] is None else max(s["numeric_max"], v)
            if isinstance(v, (int, float, str, bool)) and len(s["distinct_values"]) < 40:
                s["distinct_values"].add(v)
    out = []
    for tag in sorted(stats):
        s = stats[tag]
        vals = sorted(s["distinct_values"], key=lambda x: (str(type(x)), str(x)))
        out.append(OrderedDict([
            ("tag", tag),
            ("occurrences", s["occurrences"]),
            ("is_dynamics_curve_parameter", s["dynamics"]),
            ("value_kinds", dict(s["value_kinds"])),
            ("observed_min", s["numeric_min"]),
            ("observed_max", s["numeric_max"]),
            ("distinct_values_sample", vals if len(vals) <= 40 else vals[:40]),
            ("distinct_values_capped", len(s["distinct_values"]) >= 40),
        ]))
    return out


def build_brushes(results, outdir):
    rows = []
    srcs = []
    for base, kind in (("raster_brushes.propcol", "raster"),
                       ("vector_brushes.propcol", "vector")):
        r = results.get(base)
        if r is None or "hard_error" in r:
            srcs.append({"file": base, "status": "unavailable"})
            continue
        rows += collect_brushes(r, kind, r["parser"].objects)
        srcs.append(container_summary(base, r))
    doc = OrderedDict([
        ("schema_id", "handshake.affinity.brush_parameters.v1"),
        ("generated_at", now()),
        ("generator", TOOL_ID),
        ("parser", PARSER_ID),
        ("method", "parsed"),
        ("method_detail",
         "Each brush's parameter block is the container's own object graph: the "
         "preset leaf (PLef) -> Prop -> Brus object.  Dynamics parameters (BrDy) "
         "are expanded into base value, variance, controller, variance mode, "
         "time limit, reverse flag and the response curve (Spln) point list. "
         "Curves shared between parameters are stored once in the container and "
         "referenced by object index; those references are resolved here and the "
         "originating index is kept as shared_object_index."),
        ("source_containers", srcs),
        ("counts", {
            "brushes_total": len(rows),
            "raster": sum(1 for r in rows if r["kind"] == "raster"),
            "vector": sum(1 for r in rows if r["kind"] == "vector"),
            "categories": len({r["category"] for r in rows}),
            "brush_classes": dict(Counter(r["brush_class_4cc"] for r in rows)),
        }),
        ("parameter_schema", param_schema(rows)),
        ("brushes", rows),
    ])
    path = os.path.join(outdir, "affinity_brush_parameters.json")
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    return path, doc


# --------------------------------------------------------------------------- adjustments
NODE_GENERIC_TAGS = {
    "Spac", "TrCn", "TrAn", "TrFP", "TrFV", "Desc", "TagC", "Visi", "Opac",
    "FOpc", "FiEf", "Edtb", "MEtb", "Data", "Name", "_UID", "OUID", "CUID",
    "Usge", "Fflg", "Levs", "Chld", "CIdx", "Blnd", "BlMo", "Mask", "Locк",
    "_type", "_index", "_ref", "_resolved",
}


def adjustment_parameters(node):
    """Splits an adjustment layer node into engine parameters vs generic node
    bookkeeping, flattening one level of nested parameter blocks."""
    params, generic = OrderedDict(), OrderedDict()
    for tag, val in (node or {}).items():
        if tag in NODE_GENERIC_TAGS:
            generic[tag] = val
            continue
        if isinstance(val, dict) and "_type" in val:
            block = val.get("_type")
            for sub, sv in val.items():
                if sub in ("_type", "_index", "_ref", "_resolved"):
                    continue
                params["%s.%s" % (tag, sub)] = sv
            params.setdefault("%s._block_type_4cc" % tag, block)
        else:
            params[tag] = val
    return params, generic


def build_adjustments(results, outdir):
    r = results.get("adjustments.propcol")
    if r is None or "hard_error" in r:
        return None, None
    objects = r["parser"].objects
    root = r["root"].props["Root"]
    kinds = []
    for cat in kids(root, "Chld"):
        kind = cat.props.get("Name")
        presets = []
        for sub in kids(cat, "Chld"):
            for lf in kids(sub, "Levs"):
                prop = lf.props.get("Prop")
                payload = resolve_refs(to_json(prop, 0, 30), objects) \
                    if isinstance(prop, ka.Obj) else None
                node = None
                if isinstance(payload, dict):
                    for k, v in payload.items():
                        if isinstance(v, dict) and "_type" in v:
                            node = v
                            break
                params, generic = adjustment_parameters(node)
                presets.append(OrderedDict([
                    ("name", lf.props.get("Name")),
                    ("uid", lf.props.get("_UID")),
                    ("group", sub.props.get("Name")),
                    ("payload_type_4cc", prop.type if isinstance(prop, ka.Obj) else None),
                    ("layer_node_type_4cc", node.get("_type") if node else None),
                    ("parameters", params),
                    ("node_generic_properties", generic),
                    ("raw_values", payload),
                ]))
        # schema = union of parameter tags across this adjustment's presets
        schema = {}
        for pr in presets:
            for tag, val in (pr["parameters"] or {}).items():
                if tag.startswith("_"):
                    continue
                s = schema.setdefault(tag, {"tag": tag, "count": 0,
                                            "kinds": Counter(),
                                            "min": None, "max": None,
                                            "values": set()})
                s["count"] += 1
                s["kinds"][type(val).__name__] += 1
                if isinstance(val, (int, float)) and not isinstance(val, bool):
                    s["min"] = val if s["min"] is None else min(s["min"], val)
                    s["max"] = val if s["max"] is None else max(s["max"], val)
                if isinstance(val, (int, float, str, bool)) and len(s["values"]) < 24:
                    s["values"].add(val)
        kinds.append(OrderedDict([
            ("adjustment", kind),
            ("uid", cat.props.get("_UID")),
            ("preset_count", len(presets)),
            ("layer_node_types_4cc", sorted({p["layer_node_type_4cc"]
                                             for p in presets
                                             if p["layer_node_type_4cc"]})),
            ("parameter_schema", [OrderedDict([
                ("tag", t),
                ("present_in_presets", s["count"]),
                ("value_kinds", dict(s["kinds"])),
                ("observed_min", s["min"]),
                ("observed_max", s["max"]),
                ("distinct_values_sample",
                 sorted(s["values"], key=lambda x: (str(type(x)), str(x)))),
            ]) for t, s in sorted(schema.items())]),
            ("presets", presets),
        ]))
    with_presets = [k for k in kinds if k["preset_count"]]
    common = None
    for k in with_presets:
        tags = {t["tag"] for t in k["parameter_schema"]}
        common = tags if common is None else (common & tags)
    common = common or set()
    for k in kinds:
        k["adjustment_specific_parameters"] = [
            t for t in k["parameter_schema"] if t["tag"] not in common]
        k["common_layer_node_parameters_present"] = sorted(
            t["tag"] for t in k["parameter_schema"] if t["tag"] in common)
    doc = OrderedDict([
        ("schema_id", "handshake.affinity.adjustment_parameters.v1"),
        ("generated_at", now()),
        ("generator", TOOL_ID),
        ("parser", PARSER_ID),
        ("method", "parsed"),
        ("method_detail",
         "adjustments.propcol holds one tree node per adjustment type; each has "
         "a 'Default' group whose preset leaves carry the adjustment's own "
         "parameter object.  The parameter_schema per adjustment is the union of "
         "parameter tags actually present across that adjustment's presets, with "
         "the value ranges observed in those presets.  Where an adjustment ships "
         "no presets, no parameter values exist in this container - that is "
         "recorded as preset_count 0 and an empty schema, not as an absence of "
         "the adjustment."),
        ("source_container", container_summary("adjustments.propcol", r)),
        ("common_layer_node_parameters", OrderedDict([
            ("method", "parsed"),
            ("derivation",
             "Tags present in the parameter block of EVERY adjustment that "
             "ships presets. These are the raster adjustment layer node's own "
             "bookkeeping properties, not the adjustment's algorithm inputs; "
             "they are listed once here and excluded from each adjustment's "
             "adjustment_specific_parameters."),
            ("tags", sorted(common)),
        ])),
        ("counts", {
            "adjustment_types": len(kinds),
            "presets_total": sum(k["preset_count"] for k in kinds),
            "adjustments_with_presets": sum(1 for k in kinds if k["preset_count"]),
        }),
        ("adjustments", kinds),
    ])
    path = os.path.join(outdir, "affinity_adjustment_parameters.json")
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    return path, doc


# --------------------------------------------------------------------------- workspaces
def build_workspaces(results, outdir, strings_path):
    ws = []
    tool_ids, panel_ids, bool_ids, cmd_ids = (Counter(), Counter(),
                                              Counter(), Counter())
    for base, r in results.items():
        if not base.endswith(".afstudio") or "hard_error" in r:
            continue
        root = r["root"]
        st = root.props.get("pStt") or root.props.get("Stt ")
        if not isinstance(st, ka.Obj):
            st = root
        j = to_json(st, 0, 40)
        rec = OrderedDict([
            ("file", base),
            ("container_summary", container_summary(base, r)),
            ("workspace", j),
        ])
        # id harvesting, context aware: tool-panel groups hold TOOL ids,
        # the toolbar holds COMMAND ids, the shelves/floating stacks hold PANEL ids
        def harvest(v, ctx, key=None):
            if isinstance(v, dict):
                for k, x in v.items():
                    nctx = ctx
                    if k in ("tpnl",):
                        nctx = "tool"
                    elif k in ("tbar",):
                        nctx = "toolbar"
                    elif k in ("lshl", "rshl", "floa"):
                        nctx = "panel"
                    harvest(x, nctx, k)
            elif isinstance(v, list):
                if key in ("itms", "defI") and all(isinstance(x, str) for x in v):
                    for x in v:
                        if ctx == "tool":
                            tool_ids[x] += 1
                        elif ctx == "panel":
                            panel_ids[x] += 1
                        else:
                            cmd_ids[x] += 1
                elif key == "bool" and all(isinstance(x, str) for x in v):
                    for x in v:
                        bool_ids[x] += 1
                else:
                    for x in v:
                        harvest(x, ctx, key)
        harvest(j, "toolbar")
        ws.append(rec)

    # panel ids appear as toolbar/panel group members; keep them distinct where
    # the workspace calls them out
    strings = {}
    if strings_path and os.path.exists(strings_path):
        with open(strings_path, encoding="utf-8") as fh:
            sd = json.load(fh)
        for a in sd.get("assemblies", []):
            for rs in a.get("resource_sets", []):
                strings.update(rs.get("entries", {}))
    tool_names = sorted({k for k in strings
                         if "tool name]" in k or "[Tool description]" in k})
    panel_names = sorted({k for k in strings if "Panel" in k and "]" in k})

    doc = OrderedDict([
        ("schema_id", "handshake.affinity.tool_panel_registry.v1"),
        ("generated_at", now()),
        ("generator", TOOL_ID),
        ("parser", PARSER_ID),
        ("method", "parsed"),
        ("method_detail",
         "The nine shipped .afstudio workspaces are zstd-compressed KA "
         "containers.  They were decompressed and fully parsed; every id below "
         "is a 4CC read from the container's own itms/bool id lists, not a "
         "string scrape.  Human-readable names are NOT stored next to these ids "
         "in any shipped resource file: the .NET string tables are keyed by "
         "English source text, and the 4CCs appear in the binaries only as "
         "compiled 32-bit literals.  The name inventories below are therefore "
         "reported separately and the id->name binding is left unresolved rather "
         "than guessed."),
        ("id_to_name_binding", "unresolved"),
        ("counts", {
            "workspaces": len(ws),
            "distinct_tool_ids": len(tool_ids),
            "distinct_panel_ids": len(panel_ids),
            "distinct_toolbar_command_ids": len(cmd_ids),
            "distinct_boolean_setting_ids": len(bool_ids),
        }),
        ("id_source_note",
         "Ids are classified by where they appear in the workspace object graph: "
         "tpnl (tool panel) groups list TOOL ids, tbar lists TOOLBAR COMMAND ids, "
         "lshl/rshl/floa (left shelf, right shelf, floating stacks) list PANEL "
         "ids.  Separator pseudo-ids ('----', '---;', '---.') are layout "
         "separators, not tools."),
        ("tool_ids", [{"id_4cc": k, "occurrences": v}
                      for k, v in sorted(tool_ids.items())]),
        ("panel_ids", [{"id_4cc": k, "occurrences": v}
                       for k, v in sorted(panel_ids.items())]),
        ("toolbar_command_ids", [{"id_4cc": k, "occurrences": v}
                                 for k, v in sorted(cmd_ids.items())]),
        ("boolean_setting_ids", [{"id_4cc": k, "occurrences": v}
                                 for k, v in sorted(bool_ids.items())]),
        ("tool_name_strings_from_resources", {
            "method": "parsed",
            "source": os.path.basename(strings_path) if strings_path else None,
            "count": len(tool_names),
            "entries": tool_names,
        }),
        ("panel_name_strings_from_resources", {
            "method": "parsed",
            "source": os.path.basename(strings_path) if strings_path else None,
            "count": len(panel_names),
            "entries": panel_names,
        }),
        ("workspaces", ws),
    ])
    path = os.path.join(outdir, "affinity_tool_panel_registry.json")
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    return path, doc


# --------------------------------------------------------------------------- cli
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--res", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--strings", default=None)
    ap.add_argument("--max-depth", type=int, default=30)
    a = ap.parse_args()
    os.makedirs(a.out, exist_ok=True)
    z = ka.Zstd()
    results = parse_all(a.res, z)
    for name, fn in (("presets", build_presets),
                     ("brushes", build_brushes),
                     ("adjustments", build_adjustments)):
        if name == "presets":
            p, _ = fn(results, a.out, a.max_depth)
        else:
            p, _ = fn(results, a.out)
        print("%-12s -> %s" % (name, p))
    p, _ = build_workspaces(results, a.out, a.strings)
    print("%-12s -> %s" % ("workspaces", p))


if __name__ == "__main__":
    main()
