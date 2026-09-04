"""After Effects 2026 -> aftereffects_text_shape_mask.json

Offline. Reads only. Never launches After Effects.

Three sub-systems, each with its own parameter set:

* Shape layers - operators under ADBE Root Vectors Group: rectangle, ellipse,
  polystar, path, fill, stroke, gradient fill/stroke, merge paths, offset
  paths, pucker & bloat, repeater, round corners, trim paths, twist, wiggle
  paths (Wiggler), wiggle transform, zigzag, taper and wave on strokes.
* Text - text document, path options, more options, animators, selectors
  (range / wiggly / expression), per-character 3D.
* Masks - mask parade, mask shape, mode, opacity, expansion and both uniform
  and variable-width feather.

Evidence
--------
A. Containment: the shipped .ffx presets and Required/secret.aep are real
   serialized property graphs; walking them yields parent -> child match-name
   edges and the display name each property carried in that file.
B. Labels: $$$/BEE/VectorStream/<Operator>/... is After Effects' own label table
   for the shape operators, and $$$/AE/Selectron/Layer/... is the Character /
   Paragraph / Text properties label table.
C. Values: every stream inside a shipped Shapes/ or Text/ preset carries its
   concrete value (cdat) and, where present, its stream bounds (tdum/tduM) and
   keyframes, which is a real behavioural corpus for these sub-systems.
D. Vocabulary: the 'ADBE Vector*', 'ADBE Text*' and 'ADBE Mask*' match-name
   strings embedded in the shipped binaries, which covers operators no shipped
   preset happens to instantiate.
"""

from __future__ import annotations

import collections
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402
import ae_layer_model as LM  # noqa: E402

PARAM_SUFFIX = re.compile(r"-\d{4}$")

SHAPE_ROOTS = ("ADBE Root Vectors Group", "ADBE Vector Group",
               "ADBE Vectors Group")
SUBSYSTEMS = {
    "shape": ("ADBE Vector", "ADBE Root Vectors"),
    "text": ("ADBE Text",),
    "mask": ("ADBE Mask",),
}

# The shape operator families, keyed by the match-name stem After Effects uses.
SHAPE_OPERATORS = {
    "ADBE Vector Shape - Rect": "Rectangle path",
    "ADBE Vector Shape - Ellipse": "Ellipse path",
    "ADBE Vector Shape - Star": "Polystar path",
    "ADBE Vector Shape - Group": "Bezier path (Path)",
    "ADBE Vector Graphic - Fill": "Fill",
    "ADBE Vector Graphic - Stroke": "Stroke",
    "ADBE Vector Graphic - G-Fill": "Gradient Fill",
    "ADBE Vector Graphic - G-Stroke": "Gradient Stroke",
    "ADBE Vector Filter - Merge": "Merge Paths",
    "ADBE Vector Filter - Offset": "Offset Paths",
    "ADBE Vector Filter - PB": "Pucker & Bloat",
    "ADBE Vector Filter - Repeater": "Repeater",
    "ADBE Vector Filter - RC": "Round Corners",
    "ADBE Vector Filter - Trim": "Trim Paths",
    "ADBE Vector Filter - Twist": "Twist",
    "ADBE Vector Filter - Roughen": "Wiggle Paths (Roughen Edges)",
    "ADBE Vector Filter - Wiggler": "Wiggle Transform",
    "ADBE Vector Filter - Zigzag": "Zig Zag",
    "ADBE Vector Filter - Reveal": "Reveal (Trim) helper",
    "ADBE Vector Transform Group": "Shape group Transform",
    "ADBE Vector Materials Group": "Shape group material options",
}

TEXT_GROUPS = {
    "ADBE Text Properties": "Text property root",
    "ADBE Text Document": "Source Text document",
    "ADBE Text Path Options": "Path Options",
    "ADBE Text More Options": "More Options",
    "ADBE Text Animators": "Animators collection",
    "ADBE Text Animator": "One text animator",
    "ADBE Text Animator Properties": "Animatable text properties",
    "ADBE Text Selectors": "Selectors collection",
    "ADBE Text Selector": "Range selector",
    "ADBE Text Wiggly Selector": "Wiggly selector",
    "ADBE Text Expressible Selector": "Expression selector",
    "ADBE Text Range Advanced": "Range selector advanced options",
    "ADBE Text Per Char 3D": "Enable Per-character 3D",
}

MASK_GROUPS = {
    "ADBE Mask Parade": "Masks collection",
    "ADBE Mask Atom": "One mask",
    "ADBE Mask Shape": "Mask Path",
    "ADBE Mask Opacity": "Mask Opacity",
    "ADBE Mask Offset": "Mask Expansion",
    "ADBE Mask Feather": "Mask Feather (uniform)",
    "ADBE Mask Interp": "Variable-width mask feather points",
}


def harvest_values():
    """Concrete stream values per (parent match name, stream display name)
    across the shipped Shapes/ and Text/ presets and secret.aep."""
    obs = collections.defaultdict(lambda: {"values": [], "names": collections.Counter(),
                                           "min": None, "max": None,
                                           "keyframed": 0, "files": set()})
    roots = [os.path.join(C.support_files(), "Presets")]
    files = list(C.iter_files(roots[0], (".ffx",)))
    files += list(C.iter_files(C.support_files(), (".aep", ".aet")))
    for p in files:
        try:
            data = C.read_bytes(p)
        except OSError:
            continue
        rel = C.rel(p)
        _collect(C.rifx_parse(data), None, obs, rel)
    out = {}
    for mn, d in obs.items():
        vals = d["values"]
        rec = {
            "match_name": mn,
            "display_names_observed": [n for n, _ in d["names"].most_common(4)],
            "observations": len(vals),
            "keyframed_observations": d["keyframed"],
            "example_files": sorted(d["files"])[:3],
        }
        flat = [v[0] for v in vals if v and v[0] is not None]
        if flat:
            rec["value_samples"] = vals[:12]
            rec["observed_min"] = min(flat)
            rec["observed_max"] = max(flat)
            common = collections.Counter(round(f, 6) for f in flat).most_common(3)
            rec["most_common_first_component"] = [
                {"value": v, "count": n} for v, n in common]
        if d["min"] is not None:
            rec["declared_stream_min"] = d["min"]
        if d["max"] is not None:
            rec["declared_stream_max"] = d["max"]
        out[mn] = rec
    return out, len(files)


def _collect(chunks, parent_mn, obs, rel):
    last_mn = None
    for c in chunks:
        if c.children:
            if c.ltype == b"tdbs" and last_mn:
                _stream(c, last_mn, obs, rel)
            elif c.ltype in (b"tdgp", b"sspc", b"om-s", b"shap"):
                _collect(c.children, last_mn or parent_mn, obs, rel)
            else:
                _collect(c.children, parent_mn, obs, rel)
            continue
        if c.cid == b"tdmn":
            last_mn = C.cstr(c.data)


def _stream(node, mn, obs, rel):
    if PARAM_SUFFIX.search(mn):
        return
    d = obs[mn]
    d["files"].add(rel)
    for ch in node.children:
        if ch.cid == b"tdsn":
            nm = C.utf8_chunk(ch.data).lstrip("\t").strip()
            if nm and nm != "-_0_/-":
                d["names"][nm] += 1
        elif ch.cid == b"cdat":
            vals = [v for v in C.decode_cdat(ch.data)]
            while len(vals) > 1 and (vals[-1] == 0 or vals[-1] is None):
                vals.pop()
            d["values"].append(vals)
        elif ch.cid == b"tdum" and len(ch.data) == 8:
            d["min"] = C._fin(struct.unpack(">d", ch.data)[0])
        elif ch.cid == b"tduM" and len(ch.data) == 8:
            d["max"] = C._fin(struct.unpack(">d", ch.data)[0])
        elif ch.ltype == b"list":
            d["keyframed"] += 1


def label_table(idx, prefix):
    rows = {}
    for k, v in C.keys_under(prefix, idx).items():
        text = v["text"]
        rec = {"string_key": k, "label": text}
        if "|" in text and len(text) < 400:
            rec["options"] = text.split("|")
            rec["option_count"] = len(rec["options"])
        rows[k] = rec
    return rows


def group_by_operator(vector_labels):
    groups = collections.defaultdict(list)
    for k, v in vector_labels.items():
        parts = k.split("/")
        if len(parts) >= 3:
            groups[parts[2]].append(v)
    return {k: sorted(v, key=lambda r: r["string_key"]) for k, v in groups.items()}


def main():
    idx = C.build_english_index()
    edges, names, seen_in, files = LM.harvest_structure()
    vocab = LM.harvest_vocabulary()
    values, value_files = harvest_values()

    children = collections.defaultdict(set)
    parents = collections.defaultdict(set)
    for (a, b), _n in edges.items():
        if PARAM_SUFFIX.search(b):
            continue
        children[a].add(b)
        parents[b].add(a)

    def node(mn, semantic=None):
        rec = {
            "match_name": mn,
            "semantic": semantic,
            "display_names_observed": [n for n, _ in
                                       (names.get(mn) or collections.Counter()).most_common(4)],
            "child_match_names": sorted(children.get(mn, ())),
            "parent_match_names": sorted(parents.get(mn, ())),
            "present_in_binaries": len(vocab.get(mn, [])),
        }
        v = values.get(mn)
        if v:
            rec["observed_values"] = {k: val for k, val in v.items()
                                      if k not in ("match_name",)}
        return {k: val for k, val in rec.items() if val not in (None, [], 0, {})}

    subsystem = {}
    for name, prefixes in SUBSYSTEMS.items():
        mns = sorted({m for m in set(vocab) | set(children) | set(parents) | set(values)
                      if m.startswith(prefixes) and not PARAM_SUFFIX.search(m)})
        subsystem[name] = {
            "match_name_count": len(mns),
            "nodes": [node(m) for m in mns],
        }

    shape_ops = {mn: node(mn, sem) for mn, sem in SHAPE_OPERATORS.items()}
    text_groups = {mn: node(mn, sem) for mn, sem in TEXT_GROUPS.items()}
    mask_groups = {mn: node(mn, sem) for mn, sem in MASK_GROUPS.items()}

    vector_labels = label_table(idx, "BEE/VectorStream/")
    selectron = label_table(idx, "AE/Selectron/")
    char_pal = label_table(idx, "AE/Character_Palette/")
    para_pal = label_table(idx, "AE/Paragraph_Palette/")

    animator_props = sorted(children.get("ADBE Text Animator Properties", ()))
    selectors = sorted(children.get("ADBE Text Selectors", ()))

    missing_ops = [mn for mn in SHAPE_OPERATORS
                   if mn not in vocab and mn not in children and mn not in values]

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_text_shape_mask.py",
        "evidence": [
            {"id": "A", "label": "parsed",
             "path": "Support Files/Presets/**/*.ffx + Support Files/Required/secret.aep",
             "what": "containment edges and per-file display names",
             "files_read": files},
            {"id": "B", "label": "parsed",
             "path": "$$$/BEE/VectorStream/* and $$$/AE/Selectron/* strings",
             "what": "After Effects' own label tables for the shape operators "
                     "and for the Character / Paragraph / Text property panels"},
            {"id": "C", "label": "parsed",
             "path": "Support Files/Presets/Shapes/** and Presets/Text/**",
             "what": "concrete stream values (cdat), stream bounds (tdum/tduM) "
                     "and keyframe presence per property",
             "files_read": value_files},
            {"id": "D", "label": "parsed",
             "path": "Support Files/**/*.dll|*.aex",
             "what": "ADBE Vector* / ADBE Text* / ADBE Mask* match-name vocabulary"},
        ],
        "failures_and_limits": [
            "observed_values are the values the SHIPPED presets happen to set. "
            "They are real on-disk values and a valid behavioural corpus, but "
            "they are NOT the factory default of the property. Factory defaults "
            "for these built-in property groups are not declared in any on-disk "
            "manifest (unlike effect parameters, which carry pard records).",
            "declared_stream_min / declared_stream_max come from the tdum/tduM "
            "chunks that the file actually carries; a property without them is "
            "reported without bounds rather than with guessed ones.",
            "The semantic label on each operator is a DERIVED mapping from the "
            "match name to the name After Effects shows in its Add menu. The "
            "match name itself is read verbatim.",
            ("shape operators named in this tool but not found on disk: %s"
             % missing_ops) if missing_ops else None,
        ],
        "counts": {
            "shape_match_names": subsystem["shape"]["match_name_count"],
            "text_match_names": subsystem["text"]["match_name_count"],
            "mask_match_names": subsystem["mask"]["match_name_count"],
            "shape_operator_families": len(shape_ops),
            "vector_stream_label_strings": len(vector_labels),
            "text_panel_label_strings": len(selectron) + len(char_pal) + len(para_pal),
            "properties_with_observed_values": len(values),
            "text_animator_properties": len(animator_props),
            "text_selector_kinds": len(selectors),
        },
    }
    method["failures_and_limits"] = [f for f in method["failures_and_limits"] if f]

    payload = {
        "summary": {
            "shape_operator_families": len(shape_ops),
            "shape_match_names": subsystem["shape"]["match_name_count"],
            "text_match_names": subsystem["text"]["match_name_count"],
            "text_animator_properties": len(animator_props),
            "text_selector_kinds": selectors,
            "mask_match_names": subsystem["mask"]["match_name_count"],
            "properties_with_observed_values": len(values),
        },
        "shape_layer": {
            "root_match_names": [r for r in SHAPE_ROOTS
                                 if r in vocab or r in children],
            "operators": shape_ops,
            "operator_label_tables": group_by_operator(vector_labels),
            "all_shape_match_names": subsystem["shape"]["nodes"],
        },
        "text_layer": {
            "groups": text_groups,
            "animator_properties": animator_props,
            "selector_kinds": selectors,
            "per_character_3d_match_name": "ADBE Text Per Char 3D",
            "character_and_paragraph_panel_labels": {
                "AE/Selectron": selectron,
                "AE/Character_Palette": char_pal,
                "AE/Paragraph_Palette": para_pal,
            },
            "all_text_match_names": subsystem["text"]["nodes"],
        },
        "masks": {
            "groups": mask_groups,
            "all_mask_match_names": subsystem["mask"]["nodes"],
            "mask_mode_enumeration_ref":
                "aftereffects_layer_property_model.json#enumerations.mask_modes",
        },
    }
    C.write_json("aftereffects_text_shape_mask.json",
                 "handshake.studio.teardown.aftereffects.text_shape_mask",
                 method, payload)
    print("shape=%d text=%d mask=%d ops=%d values=%d"
          % (subsystem["shape"]["match_name_count"],
             subsystem["text"]["match_name_count"],
             subsystem["mask"]["match_name_count"], len(shape_ops), len(values)),
          file=sys.stderr)


if __name__ == "__main__":
    main()
