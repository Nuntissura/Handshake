"""After Effects 2026 -> aftereffects_layer_property_model.json

Offline. Reads only. Never launches After Effects.

Recovers the composition / layer / property / keyframe object model from four
independent on-disk sources and cross-links them:

A. Structural evidence - every shipped .ffx preset and Required/secret.aep is a
   real serialized After Effects property graph. Walking them yields
   parent->child property containment edges keyed by the real match names, and
   the match-name -> display-name mapping (tdmn next to tdsn).
B. Vocabulary evidence - the "ADBE ..." match-name strings embedded in the
   shipped binaries (BEE.dll, FLT.dll, AfterFXLib.dll and the plug-ins), which
   is the full stream vocabulary rather than just the part a preset happens to
   touch.
C. Enumerations - the shipped menu tables ($$$/AE/MenuID/<n>/<Item>) give the
   exact, ordered option sets for blending modes, track mattes, mask modes,
   keyframe interpolation, layer quality and sampling. Menus are located by
   CONTENT signature, never by a hardcoded menu number.
D. Layer styles - aelib.dll embeds a second, fully typed Effects XML declaring
   the Photoshop layer-style stream groups with defaults and ranges.
"""

from __future__ import annotations

import collections
import os
import re
import struct
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402
import ae_effects as EFF  # noqa: E402

PARAM_SUFFIX = re.compile(r"-\d{4}$")


# --------------------------------------------------------------------------
# A. structural evidence
# --------------------------------------------------------------------------

def harvest_structure():
    """Walk every .ffx and .aep for containment edges and name mappings."""
    edges = collections.Counter()
    names = collections.defaultdict(collections.Counter)
    seen_in = collections.defaultdict(set)
    files = 0

    targets = list(C.iter_files(os.path.join(C.support_files(), "Presets"), (".ffx",)))
    targets += list(C.iter_files(C.support_files(), (".aep", ".aet")))
    for p in targets:
        try:
            data = C.read_bytes(p)
        except OSError:
            continue
        files += 1
        rel = C.rel(p)
        _walk(C.rifx_parse(data), None, edges, names, seen_in, rel)
        _paths(C.rifx_parse(data), edges, seen_in, rel)
    return edges, names, seen_in, files


def _walk(chunks, parent_mn, edges, names, seen_in, rel):
    """Containment from the value tree.

    A tdmn names the LIST(tdgp)/LIST(tdbs) that FOLLOWS it, and that child's
    own first tdsn is that property's display name, so the pair
    (tdmn at parent level, first tdsn inside the child) is the real
    match-name -> display-name mapping. Returns this level's own first tdsn so
    the caller can bind it.
    """
    last_mn = None
    own_name = None
    for c in chunks:
        if c.children:
            if c.ltype in (b"tdgp", b"tdbs", b"sspc", b"om-s", b"shap"):
                mn = last_mn
                if mn and parent_mn and mn != parent_mn:
                    edges[(parent_mn, mn)] += 1
                    seen_in[(parent_mn, mn)].add(rel)
                child_name = _walk(c.children, mn or parent_mn, edges, names,
                                   seen_in, rel)
                if mn and child_name and child_name not in ("-_0_/-", ""):
                    names[mn][child_name] += 1
            else:
                _walk(c.children, parent_mn, edges, names, seen_in, rel)
            continue
        if c.cid == b"tdmn":
            last_mn = C.cstr(c.data)
        elif c.cid == b"tdsn" and own_name is None:
            own_name = C.utf8_chunk(c.data).lstrip("\t").strip()
        elif c.cid == b"fnam" and own_name is None:
            own_name = C.utf8_chunk(c.data).lstrip("\t").strip()
    return own_name


def _paths(chunks, edges, seen_in, rel):
    """Containment from the target property paths: LIST(tdsp) holds an ordered
    chain of LIST(tdsi){tdix,tdmn} segments, which is a literal property path."""
    for c in chunks:
        if c.ltype == b"tdsp":
            chain = []
            for ch in c.children:
                if ch.ltype == b"tdsi":
                    for g in ch.children:
                        if g.cid == b"tdmn":
                            chain.append(C.cstr(g.data))
            chain = [x for x in chain if x and x != "ADBE End of path sentinel"]
            for a, b in zip(chain, chain[1:]):
                edges[(a, b)] += 1
                seen_in[(a, b)].add(rel)
        elif c.children:
            _paths(c.children, edges, seen_in, rel)


# --------------------------------------------------------------------------
# B. vocabulary evidence
# --------------------------------------------------------------------------

# A match name is a plain identifier-ish string. Reject candidates that carry
# punctuation only sentence text would have, which is how the raw scan picks up
# error messages that quote a match name.
ADBE_RE = re.compile(rb"(?<=\x00)(ADBE [A-Za-z0-9][A-Za-z0-9 _:/&.+\-]{0,58})\x00")
BAD_MN = re.compile(r"( {2,})|([.,]$)|(#\{)|[\"'()\[\]]")


def harvest_vocabulary():
    per_file = collections.defaultdict(set)
    for p in C.iter_files(C.support_files(), (".dll", ".exe", ".aex"),
                          skip_dirs=("node_modules", "CEPHtmlEngine")):
        data = C.read_bytes(p)
        s = {m.group(1).decode("latin-1") for m in ADBE_RE.finditer(data)}
        s = {x for x in s if not BAD_MN.search(x)}
        if s:
            per_file[C.rel(p)] = s
    inv = collections.defaultdict(list)
    for f, s in per_file.items():
        for v in s:
            inv[v].append(f)
    return inv


# --------------------------------------------------------------------------
# C. enumerations located by content signature
# --------------------------------------------------------------------------

ENUM_SIGNATURES = [
    ("layer_blending_modes",
     {"Normal", "Multiply", "Screen", "Overlay", "Luminosity", "Stencil Alpha"},
     "Layer blending mode (Mode column)"),
    ("track_matte_modes",
     {"Alpha Matte", "Luma Matte", "Alpha Inverted Matte", "No Track Matte"},
     "Track matte type (TrkMat column)"),
    ("mask_modes",
     {"Add", "Subtract", "Intersect", "Difference", "Lighten", "Darken", "None"},
     "Mask mode"),
    ("keyframe_interpolation",
     {"Linear", "Continuous Bezier", "Bezier", "Hold"},
     "Keyframe interpolation / assistant"),
    ("layer_quality",
     {"Best", "Draft", "Wireframe"},
     "Layer quality"),
    ("frame_blending",
     {"Frame Mix", "Pixel Motion"},
     "Frame blending mode"),
    ("time_display",
     {"Timecode", "Frames", "Feet + Frames"},
     "Time display format"),
    ("view_layout",
     {"1 View", "2 Views", "4 Views"},
     "Composition viewer layout"),
]


def find_menu_enums(idx):
    menus = collections.defaultdict(list)
    for k in sorted(C.keys_under("AE/MenuID/", idx)):
        parts = k.split("/")
        if len(parts) < 4:
            continue
        label = idx[k]["text"].replace("&", "")
        menus[parts[2]].append({"label": label, "string_key": k,
                                "command_name": parts[3]})
    found = {}
    for name, sig, desc in ENUM_SIGNATURES:
        best = None
        for mno, items in menus.items():
            labels = {i["label"] for i in items}
            if sig <= labels:
                score = len(labels)
                if best is None or score < best[1]:
                    best = (mno, score, items)
        if best:
            mno, _score, items = best
            seen = set()
            options = []
            for i in items:
                if i["label"] in seen:
                    continue
                seen.add(i["label"])
                options.append(i)
            found[name] = {
                "semantic": desc,
                "menu_id": mno,
                "option_count": len(options),
                "options": options,
                "evidence": "$$$/AE/MenuID/%s/* located by content signature %s"
                            % (mno, sorted(sig)),
            }
    return found, menus


# --------------------------------------------------------------------------
# D. layer styles XML embedded in aelib.dll
# --------------------------------------------------------------------------

def layer_styles():
    p = os.path.join(C.support_files(), "aelib.dll")
    data = C.read_bytes(p)
    out = []
    for m in re.finditer(rb"<\?xml[^>]{0,120}\?>", data):
        s = m.start()
        e = data.find(b"</Effects>", s)
        if e < 0 or e - s > 400_000:
            continue
        body = data[s:e + len(b"</Effects>")]
        if b"layer style" not in body[:1200].lower():
            continue
        text = body.decode("utf-8", "replace")
        try:
            root = ET.fromstring(text[text.index("<Effects>"):])
        except Exception:
            continue
        for eff in root.findall("Effect"):
            out.append({
                "match_name": eff.get("matchname"),
                "display_name": C.strip_zstring_key(eff.get("name", "")),
                "external_id": eff.get("external_id"),
                "parameters": EFF._xml_params(eff),
                "source": C.rel(p) + " (embedded layer-style Effects XML)",
            })
        if out:
            break
    return out


# --------------------------------------------------------------------------

LAYER_TYPE_HINTS = [
    ("footage", ["ADBE AV Layer"], "AV layer backed by a footage item"),
    ("composition", ["ADBE AV Layer"], "AV layer whose source is a CompItem"),
    ("solid", ["ADBE Solid"], "solid colour layer"),
    ("text", ["ADBE Text Layer", "ADBE Text Properties"], "text layer"),
    ("shape", ["ADBE Vector Layer", "ADBE Root Vectors Group"], "shape layer"),
    ("camera", ["ADBE Camera Layer", "ADBE Camera Options Group"], "camera"),
    ("light", ["ADBE Light Layer", "ADBE Light Options Group"], "light"),
    ("null", ["ADBE Null Layer"], "null object"),
    ("adjustment", ["ADBE Adjustment Layer"], "adjustment layer"),
    ("guide", ["ADBE Guide Layer"], "guide layer"),
]

GROUP_TOPICS = {
    "transform": ("ADBE Transform Group",),
    "material_options_3d": ("ADBE Material Options Group",),
    "geometry_options_3d": ("ADBE Extrsn Options Group", "ADBE Plane Options Group"),
    "camera_options": ("ADBE Camera Options Group",),
    "light_options": ("ADBE Light Options Group",),
    "audio": ("ADBE Audio Group",),
    "masks": ("ADBE Mask Parade", "ADBE Mask Atom"),
    "effects": ("ADBE Effect Parade",),
    "layer_styles": ("ADBE Layer Styles",),
    "time_remap": ("ADBE Time Remapping",),
    "text": ("ADBE Text Properties",),
    "shape": ("ADBE Root Vectors Group",),
    "data": ("ADBE Data Group",),
}


def main():
    idx = C.build_english_index()
    edges, names, seen_in, files = harvest_structure()
    vocab = harvest_vocabulary()
    enums, menus = find_menu_enums(idx)
    styles = layer_styles()

    # match-name catalogue
    catalogue = []
    all_mn = set(names) | set(vocab) | {a for a, _ in edges} | {b for _, b in edges}
    for mn in sorted(all_mn):
        if PARAM_SUFFIX.search(mn):
            continue          # effect parameter slots live in the effects file
        disp = names.get(mn) or collections.Counter()
        children = sorted({b for a, b in edges if a == mn and not PARAM_SUFFIX.search(b)})
        parents = sorted({a for a, b in edges if b == mn})
        rec = {
            "match_name": mn,
            "display_names_observed": [n for n, _ in disp.most_common(4)],
            "child_match_names": children,
            "parent_match_names": parents,
            "in_binaries": len(vocab.get(mn, [])),
        }
        if vocab.get(mn):
            rec["example_binary"] = sorted(vocab[mn])[0]
        catalogue.append({k: v for k, v in rec.items() if v not in (None, [], 0)})

    edge_rows = [{"parent": a, "child": b, "observations": n,
                  "example_file": sorted(seen_in[(a, b)])[0]}
                 for (a, b), n in edges.most_common()
                 if not PARAM_SUFFIX.search(b)]

    by_mn = {c["match_name"]: c for c in catalogue}
    topics = {}
    for topic, roots in GROUP_TOPICS.items():
        present = [r for r in roots if r in by_mn]
        topics[topic] = {
            "root_match_names_present": present,
            "root_match_names_absent": [r for r in roots if r not in by_mn],
            "subtree": {r: by_mn[r].get("child_match_names", []) for r in present},
        }

    layer_types = []
    for name, mns, desc in LAYER_TYPE_HINTS:
        layer_types.append({
            "layer_type": name,
            "description": desc,
            "match_names_present_on_disk": [m for m in mns if m in by_mn or m in vocab],
            "match_names_not_found": [m for m in mns
                                      if m not in by_mn and m not in vocab],
        })

    keyframe_model = {
        "container": "LIST(list) { lhd3 header, ldat records } attached to a "
                     "value stream (LIST(tdbs))",
        "header_fields": {"key_count": "u32 @0x08", "key_record_bytes": "u32 @0x10"},
        "record_fields": {
            "time": "u32 @0x00, raw on-disk unit",
            "in_interpolation": "u8 @0x04",
            "out_interpolation": "u8 @0x05",
            "values_and_tangents": "(record_bytes-8)/8 big-endian float64 from "
                                   "@0x08; the leading entries are the value, "
                                   "one per stream component",
        },
        "interpolation_codes_observed": None,   # filled below
        "default_ease_influence_observed": 0.16666666666,
        "default_ease_influence_note":
            "0.1666666.. appears verbatim in shipped preset keyframe records "
            "and matches After Effects' 16.667% default ease influence.",
        "static_value_container": "cdat, array of big-endian float64",
        "stream_bounds": "tdum (minimum) and tduM (maximum), float64",
        "expression_container": "expr, NUL-terminated UTF-8 source on the stream",
        "spatial_vs_temporal":
            "Spatial streams carry inTangents/outTangents identifiers in the "
            "expression vocabulary (see aftereffects_scripting_expressions.json); "
            "the shipped presets only exercise temporal keyframes, so the "
            "spatial tangent layout inside ldat is NOT proven here.",
    }
    # interpolation codes actually present in shipped presets
    codes = collections.Counter()
    for p in C.iter_files(os.path.join(C.support_files(), "Presets"), (".ffx",)):
        d = C.read_bytes(p)
        for c in C.rifx_iter(C.rifx_parse(d)):
            if c.ltype == b"list":
                h = dd = None
                for g in c.children:
                    if g.cid == b"lhd3":
                        h = g.data
                    elif g.cid == b"ldat":
                        dd = g.data
                if not h or not dd or len(h) < 0x14:
                    continue
                cnt = struct.unpack_from(">I", h, 0x08)[0]
                size = struct.unpack_from(">I", h, 0x10)[0]
                if not size or size > 4096:
                    continue
                for i in range(min(cnt, len(dd) // size)):
                    b = dd[i * size:(i + 1) * size]
                    if len(b) >= 6:
                        codes[(b[4], b[5])] += 1
    keyframe_model["interpolation_codes_observed"] = {
        "in_out_code_pairs": {"%d,%d" % k: v for k, v in codes.most_common()},
        "naming": "The AE SDK AEGP_KeyframeInterpolationType enum reads 0 NONE, "
                  "1 LINEAR, 2 BEZIER, 3 HOLD. That naming is HEURISTIC here; "
                  "only the raw codes above are read off disk.",
    }

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_layer_model.py",
        "evidence": [
            {"id": "A", "label": "parsed",
             "path": "Support Files/Presets/**/*.ffx + Support Files/Required/secret.aep",
             "what": "serialized property graphs; containment edges from both "
                     "the value tree (tdmn before LIST(tdgp)/LIST(tdbs)) and "
                     "the target property paths (LIST(tdsp)/LIST(tdsi))",
             "files_read": files},
            {"id": "B", "label": "parsed",
             "path": "Support Files/**/*.dll|*.exe|*.aex",
             "what": "'ADBE <name>' match-name vocabulary embedded as C strings"},
            {"id": "C", "label": "parsed",
             "path": "$$$/AE/MenuID/<n>/<Item> string tables",
             "what": "ordered option sets for the layer enumerations; each menu "
                     "is located by CONTENT signature, not by a hardcoded id"},
            {"id": "D", "label": "parsed", "path": "Support Files/aelib.dll",
             "what": "embedded layer-style Effects XML with typed parameters"},
        ],
        "failures_and_limits": [
            "Containment edges are evidence of what the shipped presets and "
            "secret.aep actually contain. A property group that no shipped file "
            "instantiates has no edges, so child_match_names can be empty for a "
            "match name that exists in the binary vocabulary. Both signals are "
            "reported side by side (in_binaries vs child_match_names).",
            "Spatial keyframe tangents are not exercised by any shipped preset, "
            "so their byte layout inside ldat is NOT decoded.",
            "Keyframe times are raw on-disk integers; the unit-to-seconds "
            "conversion is not proven and is therefore not asserted.",
            "Enumeration option order is menu order, which is the order After "
            "Effects presents; the numeric stream value for each option is not "
            "stated in the menu table and is not asserted.",
        ],
        "counts": {
            "structure_files_read": files,
            "match_names_catalogued": len(catalogue),
            "containment_edges": len(edge_rows),
            "match_names_in_binary_vocabulary": len(vocab),
            "enumerations_recovered": len(enums),
            "menu_tables_available": len(menus),
            "layer_styles": len(styles),
            "layer_style_parameters": sum(_count_params(s["parameters"])
                                          for s in styles),
        },
    }

    payload = {
        "summary": {
            "match_names_catalogued": len(catalogue),
            "containment_edges": len(edge_rows),
            "enumerations": {k: v["option_count"] for k, v in enums.items()},
            "layer_styles": len(styles),
        },
        "layer_types": layer_types,
        "property_group_topics": topics,
        "enumerations": enums,
        "keyframe_and_interpolation_model": keyframe_model,
        "layer_styles": styles,
        "property_containment_edges": edge_rows,
        "match_name_catalogue": catalogue,
    }
    C.write_json("aftereffects_layer_property_model.json",
                 "handshake.studio.teardown.aftereffects.layer_property_model",
                 method, payload)
    print("match_names=%d edges=%d enums=%d styles=%d"
          % (len(catalogue), len(edge_rows), len(enums), len(styles)),
          file=sys.stderr)


def _count_params(params):
    n = 0
    for p in params or []:
        n += 1
        n += _count_params(p.get("children"))
    return n


if __name__ == "__main__":
    main()
