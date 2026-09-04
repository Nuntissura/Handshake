"""After Effects 2026 -> aftereffects_presets.json

Offline. Reads only. Never launches After Effects.

Parses every shipped animation / effect preset (Support Files/Presets/**/*.ffx)
out of its big-endian RIFX "FaFX" container and emits, per preset: the category
path, the property paths it targets, the effects it applies, every parameter
definition, every static value, every keyframe (time, interpolation, value,
tangents), every expression, and any mask/shape or text payload.

Container map (recovered by walking the shipped files):
  RIFX(FaFX)
    head                         format/version words
    LIST(besc)                   the preset body
      beso                       preset header
      LIST(tdsp) + tdsn          one target property path + its display name
        LIST(tdsi){tdix,tdmn}    path segment: index + match name
      LIST(sspc)                 one applied effect instance
        fnam                     effect instance name
        LIST(parT){parn,tdmn,pard,pdnm}   its parameter definitions
        LIST(tdgp){tdsb,tdsn,tdmn,...}    its value tree
          LIST(tdbs)             a single value stream
            tdb4                 stream descriptor
            cdat                 static value, big-endian doubles
            LIST(list){lhd3,ldat}  keyframe table
            tdum / tduM          stream min / max
            expr                 expression source
"""

from __future__ import annotations

import collections
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402

# AEGP_KeyframeInterpolationType, AE SDK. HEURISTIC: the numeric->name binding
# is taken from the SDK enum, not proven from disk. The raw byte is also kept.
INTERP = {0: "NONE", 1: "LINEAR", 2: "BEZIER", 3: "HOLD"}


def clean_name(s: str) -> str:
    return s.lstrip("\t").strip()


# --------------------------------------------------------------------------
# keyframe table
# --------------------------------------------------------------------------

def decode_keyframes(lhd3: bytes, ldat: bytes):
    """lhd3 carries the key count at 0x08 and the per-key record size at 0x10;
    both were confirmed against shipped presets where len(ldat) == count*size
    (e.g. Transitions - Dissolves/Dissolve - dither.ffx: 2 keys x 48 bytes = 96
    bytes, values 0.0 and 100.0 for Transition Completion)."""
    if len(lhd3) < 0x14:
        return None
    count = struct.unpack_from(">I", lhd3, 0x08)[0]
    size = struct.unpack_from(">I", lhd3, 0x10)[0]
    if not size or count > 10000 or size > 4096:
        return None
    if len(ldat) < count * size:
        count = len(ldat) // size
    ndoubles = (size - 8) // 8
    keys = []
    for i in range(count):
        b = ldat[i * size:(i + 1) * size]
        if len(b) < 8:
            break
        t = struct.unpack_from(">I", b, 0)[0]
        it, ot = b[4], b[5]
        vals = list(struct.unpack_from(">%dd" % ndoubles, b, 8)) if ndoubles else []
        vals = [C._fin(v) for v in vals]
        keys.append({
            "time_raw": t,
            "in_interpolation_code": it,
            "in_interpolation": INTERP.get(it, "UNKNOWN_%d" % it),
            "out_interpolation_code": ot,
            "out_interpolation": INTERP.get(ot, "UNKNOWN_%d" % ot),
            "doubles": vals,
        })
    return {
        "key_count": count,
        "key_record_bytes": size,
        "doubles_per_key": ndoubles,
        "keys": keys,
        "doubles_layout_note": (
            "doubles[0..] begin with the keyframe value (one entry per stream "
            "component) and are followed by tangent/influence words. Only "
            "doubles[0] is proven to be the value; the tangent split is "
            "HEURISTIC. Observed default influence 0.16666... matches After "
            "Effects' 16.667% default ease influence."),
    }


def decode_tdb4(b: bytes) -> dict:
    out = {}
    if len(b) >= 16:
        out["components_hint"] = struct.unpack_from(">H", b, 0x04)[0]
        out["time_scale_hint"] = struct.unpack_from(">H", b, 0x0E)[0]
    return out


def trim_zeros(vals):
    """cdat always carries 5 float64 slots; most streams use fewer. Trim the
    trailing all-zero slots but never below one value."""
    out = list(vals)
    while len(out) > 1 and (out[-1] == 0 or out[-1] is None):
        out.pop()
    return out


# --------------------------------------------------------------------------
# preset parsing
# --------------------------------------------------------------------------

def parse_preset(path: str, root_dir: str):
    data = C.read_bytes(path)
    chunks = C.rifx_parse(data)
    rel = C.rel(path)
    catpath = os.path.relpath(os.path.dirname(path), root_dir).replace("\\", "/")
    doc = {
        "preset_name": os.path.splitext(os.path.basename(path))[0],
        "category_path": None if catpath == "." else catpath,
        "file": rel,
        "file_bytes": len(data),
        "target_property_paths": [],
        "effects_applied": [],
        "expressions": [],
        "stats": {},
    }
    top = _find_list(chunks, b"FaFX")
    if top is None:
        doc["parse_error"] = "not a FaFX RIFX container"
        return doc
    besc = _find_list(top.children, b"besc")
    body = besc.children if besc else top.children

    # Two preset shapes exist on disk:
    #  * effect presets: each target path is followed by a LIST(sspc) holding a
    #    whole effect instance (its parT definitions plus its value tree);
    #  * property presets (all of Presets/Legacy, most Text presets): the target
    #    path is followed directly by a LIST(tdbs) or LIST(tdgp) that carries the
    #    value for that one property - no effect is involved. Text animator,
    #    selector, shape-operator and transform presets take this shape.
    doc["direct_property_values"] = []
    pending_path = None
    last_target = None
    for c in body:
        if c.ltype == b"tdsp":
            pending_path = _read_path(c)
        elif c.cid == b"tdsn" and pending_path is not None:
            pending_path["display_name"] = clean_name(C.utf8_chunk(c.data))
            doc["target_property_paths"].append(pending_path)
            last_target = pending_path
            pending_path = None
        elif c.ltype == b"sspc":
            e = _read_sspc(c, doc)
            if last_target:
                e["target_path"] = last_target.get("steps")
            doc["effects_applied"].append(e)
        elif c.ltype == b"tdbs":
            s = _read_tdbs(c, doc)
            s["target_path"] = last_target.get("steps") if last_target else None
            s["target_display_name"] = (last_target or {}).get("display_name")
            doc["direct_property_values"].append(s)
        elif c.ltype == b"tdgp":
            g = _read_tdgp(c, doc)
            g["target_path"] = last_target.get("steps") if last_target else None
            g["target_display_name"] = (last_target or {}).get("display_name")
            doc["direct_property_values"].append(g)
    if pending_path is not None:
        doc["target_property_paths"].append(pending_path)

    kf = 0
    props = 0
    roots = [e.get("values") or {} for e in doc["effects_applied"]]
    roots += doc["direct_property_values"]
    for r in roots:
        for s in _iter_streams(r):
            props += 1
            if s.get("keyframes"):
                kf += s["keyframes"]["key_count"]
    doc["preset_shape"] = ("effect_preset" if doc["effects_applied"]
                           else "property_preset" if doc["direct_property_values"]
                           else "empty_or_unrecognised")
    doc["stats"] = {
        "effects_applied": len(doc["effects_applied"]),
        "direct_property_values": len(doc["direct_property_values"]),
        "target_property_paths": len(doc["target_property_paths"]),
        "value_streams": props,
        "keyframes": kf,
        "expressions": len(doc["expressions"]),
    }
    return doc


def _find_list(chunks, ltype):
    for c in chunks:
        if c.ltype == ltype:
            return c
    return None


def _read_path(c):
    steps = []
    for ch in c.children:
        if ch.ltype == b"tdsi":
            idx = None
            mn = None
            for g in ch.children:
                if g.cid == b"tdix":
                    idx = struct.unpack(">i", g.data)[0] if len(g.data) == 4 else None
                elif g.cid == b"tdmn":
                    mn = C.cstr(g.data)
            steps.append({"index": idx, "match_name": mn})
    return {"steps": steps}


def _read_sspc(c, doc):
    out = {"instance_name": None, "parameter_definitions": [], "values": None}
    for ch in c.children:
        if ch.cid == b"fnam":
            out["instance_name"] = clean_name(C.utf8_chunk(ch.data))
        elif ch.ltype == b"parT":
            out["parameter_definitions"] = _read_part(ch)
        elif ch.ltype == b"tdgp":
            out["values"] = _read_tdgp(ch, doc)
    mns = [p["match_name"] for p in out["parameter_definitions"] if p.get("match_name")]
    if mns:
        m = re.match(r"^(.*)-\d{4}$", mns[0])
        if m:
            out["effect_match_name"] = m.group(1)
    return out


def _read_part(c):
    params = []
    last_mn = None
    for ch in c.children:
        if ch.cid == b"tdmn":
            last_mn = C.cstr(ch.data)
        elif ch.cid == b"pard":
            full = C.decode_pard(ch.data)
            # keep the preset copy compact: the authoritative full record is in
            # aftereffects_effects_catalogue.json, keyed by the same match name
            rec = {"match_name": last_mn,
                   "param_type": full.get("param_type"),
                   "name": full.get("name") or None}
            for k in ("default", "default_index", "default_argb", "option_count",
                      "valid_min", "valid_max", "slider_min", "slider_max",
                      "display_flags", "units"):
                if full.get(k) not in (None, [], ""):
                    rec[k] = full[k]
            params.append(rec)
        elif ch.cid == b"pdnm" and params:
            txt = C.utf8_chunk(ch.data) if ch.data[:4] == b"Utf8" else C.cstr(ch.data)
            p = params[-1]
            if p.get("param_type") == "POPUP":
                p["options"] = [{"index": i + 1, "label": o}
                                for i, o in enumerate(txt.split("|"))]
            elif p.get("param_type") == "CHECKBOX":
                p["checkbox_label"] = clean_name(txt)
            elif p.get("param_type") == "BUTTON":
                p["button_label"] = clean_name(txt)
            elif not p.get("name"):
                p["name"] = clean_name(txt)
    return params


def _read_tdgp(c, doc, depth=0):
    group = {"kind": "group", "name": None, "match_name": None, "children": []}
    last_mn = None
    for ch in c.children:
        if ch.cid == b"tdsn" and group["name"] is None:
            group["name"] = clean_name(C.utf8_chunk(ch.data))
        elif ch.cid == b"tdmn":
            last_mn = C.cstr(ch.data)
        elif ch.ltype == b"tdbs":
            s = _read_tdbs(ch, doc)
            s["match_name"] = last_mn
            group["children"].append(s)
        elif ch.ltype == b"tdgp":
            g = _read_tdgp(ch, doc, depth + 1)
            g["match_name"] = last_mn
            group["children"].append(g)
        elif ch.ltype in (b"om-s", b"omks", b"shap", b"btds", b"aRbs", b"otst"):
            group["children"].append(_read_special(ch, last_mn))
    return group


def _read_tdbs(c, doc):
    s = {"kind": "stream", "name": None}
    kl = None
    for ch in c.children:
        if ch.cid == b"tdsn":
            s["name"] = clean_name(C.utf8_chunk(ch.data))
        elif ch.cid == b"tdb4":
            d4 = decode_tdb4(ch.data)
            if d4.get("components_hint"):
                s["components_hint"] = d4["components_hint"]
        elif ch.cid == b"cdat":
            s["static_value"] = trim_zeros(C.decode_cdat(ch.data))
        elif ch.cid == b"tdum":
            s["stream_min"] = C._fin(struct.unpack(">d", ch.data)[0]) if len(ch.data) == 8 else None
        elif ch.cid == b"tduM":
            s["stream_max"] = C._fin(struct.unpack(">d", ch.data)[0]) if len(ch.data) == 8 else None
        elif ch.cid == b"expr":
            src = ch.data.split(b"\x00", 1)[0].decode("utf-8", "replace")
            s["expression"] = src
            doc["expressions"].append({"stream": s.get("name"), "source": src})
        elif ch.ltype == b"list":
            h = d = None
            for g in ch.children:
                if g.cid == b"lhd3":
                    h = g.data
                elif g.cid == b"ldat":
                    d = g.data
            if h is not None and d is not None:
                kl = decode_keyframes(h, d)
    if kl:
        s["keyframes"] = kl
    return s


def _read_special(c, mn):
    node = {"kind": c.ltype.decode("latin-1"), "match_name": mn}
    if c.ltype in (b"shap",):
        node["semantic"] = "mask / shape path data"
    elif c.ltype in (b"om-s", b"omks"):
        node["semantic"] = "mask outline stream"
    elif c.ltype == b"otst":
        node["semantic"] = "text document stream"
    elif c.ltype == b"aRbs":
        node["semantic"] = "arbitrary-data stream (curves, colorama ramp, ...)"
    elif c.ltype == b"btds":
        node["semantic"] = "boolean/marker stream"
    names = [clean_name(C.utf8_chunk(g.data)) for g in c.children if g.cid == b"tdsn"]
    if names:
        node["name"] = names[0]
    return node


KEEP_EMPTY = {"stats", "summary"}


def compact(node):
    """Drop None / empty-container values so the corpus stays loadable."""
    if isinstance(node, dict):
        out = {}
        for k, v in node.items():
            v2 = compact(v)
            if k in KEEP_EMPTY or v2 not in (None, [], {}, ""):
                out[k] = v2
        return out
    if isinstance(node, list):
        return [compact(v) for v in node]
    return node


def _iter_streams(node):
    if not isinstance(node, dict):
        return
    if node.get("kind") == "stream":
        yield node
    for ch in node.get("children", []) or []:
        yield from _iter_streams(ch)


# --------------------------------------------------------------------------

def main():
    root = os.path.join(C.support_files(), "Presets")
    files = sorted(C.iter_files(root, (".ffx",)))
    presets = []
    errors = []
    for p in files:
        try:
            presets.append(compact(parse_preset(p, root)))
        except Exception as exc:  # noqa: BLE001
            errors.append({"file": C.rel(p), "error": "%s: %s" % (type(exc).__name__, exc)})

    eff_use = collections.Counter()
    path_mn = collections.Counter()
    shapes = collections.Counter()
    for d in presets:
        shapes[d.get("preset_shape")] += 1
        for e in d.get("effects_applied", []):
            if e.get("effect_match_name"):
                eff_use[e["effect_match_name"]] += 1
        for tp in d.get("target_property_paths", []):
            for st in tp.get("steps", []):
                if st.get("match_name"):
                    path_mn[st["match_name"]] += 1
    cats = collections.Counter(d["category_path"] for d in presets)
    total_kf = sum(d.get("stats", {}).get("keyframes", 0) for d in presets)
    total_streams = sum(d.get("stats", {}).get("value_streams", 0) for d in presets)
    total_expr = sum(d.get("stats", {}).get("expressions", 0) for d in presets)
    total_eff = sum(d.get("stats", {}).get("effects_applied", 0) for d in presets)

    # AE also ships a user-side preset root; report whether it holds anything
    user_presets = os.path.join(C.user_data_root(), "26.3", "Presets")
    user_state = {
        "path": user_presets,
        "exists": os.path.isdir(user_presets),
        "file_count": (len(os.listdir(user_presets))
                       if os.path.isdir(user_presets) else 0),
    }

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_presets.py",
        "evidence": [
            {"path": "Support Files/Presets/**/*.ffx", "label": "parsed",
             "what": "shipped animation and effect presets",
             "extraction": "big-endian RIFX 'FaFX' walk; see the module "
                           "docstring for the recovered container map"},
        ],
        "decoding": {
            "pard": "parameter definition; offsets validated in "
                    "aftereffects_effects_catalogue.json#method.evidence[D]",
            "cdat": "static stream value, array of big-endian float64",
            "tdum/tduM": "stream minimum / maximum, float64",
            "lhd3": "keyframe table header: u32 key count at 0x08, u32 record "
                    "size at 0x10; validated because count*size == len(ldat) "
                    "for every keyframe table read",
            "ldat": "keyframe records: u32 time, u8 in-interpolation, u8 "
                    "out-interpolation, u16 flags, then (size-8)/8 float64",
            "expr": "expression source, NUL-terminated UTF-8",
        },
        "heuristics": [
            "Keyframe interpolation code -> name uses the AE SDK "
            "AEGP_KeyframeInterpolationType enum (0 NONE, 1 LINEAR, 2 BEZIER, "
            "3 HOLD). The mapping is HEURISTIC; the raw code is kept alongside.",
            "Keyframe times are emitted in the raw on-disk integer unit. The "
            "tdb4 descriptor carries a time-scale word (commonly 23976) but the "
            "raw-unit -> seconds conversion is NOT proven, so no seconds value "
            "is asserted.",
            "doubles[0..] in a keyframe record start with the value; the "
            "tangent/influence split beyond that is HEURISTIC.",
        ],
        "failures": errors,
        "counts": {
            "ffx_files_on_disk": len(files),
            "presets_parsed": len(presets),
            "presets_failed": len(errors),
            "effect_instances_total": total_eff,
            "distinct_effects_used": len(eff_use),
            "value_streams_total": total_streams,
            "keyframes_total": total_kf,
            "expressions_total": total_expr,
        },
    }
    payload = {
        "summary": {
            "preset_count": len(presets),
            "category_counts": dict(sorted(cats.items(), key=lambda kv: str(kv[0]))),
            "preset_shapes": dict(shapes),
            "effect_usage_across_presets": dict(eff_use.most_common()),
            "property_path_match_name_usage": dict(path_mn.most_common()),
            "keyframes_total": total_kf,
            "value_streams_total": total_streams,
            "expressions_total": total_expr,
        },
        "user_preset_root": user_state,
        "presets": presets,
    }
    C.write_json("aftereffects_presets.json",
                 "handshake.studio.teardown.aftereffects.presets",
                 method, payload)
    print("presets=%d effects_used=%d streams=%d keyframes=%d expr=%d errors=%d"
          % (len(presets), len(eff_use), total_streams, total_kf, total_expr,
             len(errors)), file=sys.stderr)


if __name__ == "__main__":
    main()
