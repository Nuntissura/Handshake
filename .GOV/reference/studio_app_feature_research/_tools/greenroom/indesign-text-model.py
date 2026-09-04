#!/usr/bin/env python3
"""Handshake Studio green room: InDesign paragraph / character / story attribute model.

Projects the parsed scripting object model (indesign_dom_full.json, produced by
indesign-sce2-parse.py from the installed idrc_SCE2 resources) into the text-engine slice:
every paragraph, character, story, table-text and CJK attribute, with its value type, and
with every enumerated type expanded inline to its full enumerator set.

No application is launched; this reads the already-parsed offline export plus, for the
primitive-type evidence table, nothing further.

Output: indesign_text_model.json
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
import struct
from pathlib import Path

ENGLISH_LOCALES = {1, 2}

# Some user-visible enumerated values are NOT in the SCE2 enumerations: composer names,
# kinsoku set names and mojikumi set names are shipped as localized strings. These value
# lists are pulled from the English (locale_id 1/2) idrc_PMST tables and are labelled with
# their source so they are never confused with the parsed SCE2 enumerators.
PMST_VALUE_PROBES = {
    "composer_names": ["composer"],
    "kinsoku_sets": ["kinsoku"],
    "mojikumi_sets": ["mojikumi"],
    "leading_models": ["leading model", "aki"],
    "grid_settings": ["gyoudori", "grid align", "icf"],
    "justification_ui": ["justification", "word spacing", "letter spacing", "glyph scaling"],
    "hyphenation_ui": ["hyphenat"],
    "opentype_ui": ["opentype", "ligature", "stylistic set", "swash", "titling"],
}


def parse_pmst(data: bytes):
    if len(data) < 12:
        return None
    loc, _u, cnt = struct.unpack_from("<III", data, 0)
    o, out = 12, {}
    for _ in range(cnt):
        if o + 2 > len(data):
            break
        kl = struct.unpack_from("<H", data, o)[0]; o += 2
        k = data[o:o + kl]; o += kl
        if o + 2 > len(data):
            break
        vl = struct.unpack_from("<H", data, o)[0]; o += 2
        v = data[o:o + vl]; o += vl
        if len(k) != kl or len(v) != vl:
            break
        try:
            out[k.decode("ascii")] = v.decode("utf-8")
        except UnicodeDecodeError:
            continue
    return {"locale_id": loc, "strings": out}


def plugin_of(p: Path) -> str:
    for part in p.parts:
        if part.startswith("(") and part.endswith("Resources)"):
            return part.strip("()").replace(" Resources", "")
    return "APP_ROOT"

# Classes that make up the text engine surface. Matched case-insensitively against the
# scripting class name; this list is the selection criterion and is stated in the output.
CLASS_PATTERNS = [
    r"^text$", r"^texts$", r"^story$", r"^stories$", r"^paragraph", r"^character",
    r"^word$", r"^line$", r"^insertion point", r"^text style range", r"^text column",
    r"^text frame", r"^text default", r"^text preference", r"^text variable",
    r"^text path", r"^text wrap", r"^story preference", r"^story window",
    r"^table$", r"^cell$", r"^row$", r"^column$", r"^cell style", r"^table style",
    r"^footnote", r"^endnote", r"^note$", r"^hyperlink text",
    r"^ruby", r"^tatechuyoko", r"^warichu", r"^kinsoku", r"^mojikumi",
    r"^composite font", r"^grid", r"^CJK", r"^nested", r"^GREP style", r"^line style",
    r"^bullet", r"^numbering", r"^tab stop", r"^indent", r"^drop cap",
    r"^find (text|grep|glyph|transliterate|change)", r"^change (text|grep|glyph|transliterate)",
    r"^find/change", r"^language$", r"^dictionary", r"^user dictionary",
    r"^autocorrect", r"^spell", r"^hyphenation", r"^justification",
    r"^baseline", r"^leading", r"^kerning", r"^tracking", r"^font$", r"^fonts$",
    r"^style$", r"^paragraph style", r"^character style", r"^object style",
    r"^index", r"^table of contents", r"^cross reference", r"^condition",
    r"^track change", r"^smart text", r"^optical", r"^glyph",
]
CLASS_RE = re.compile("|".join(CLASS_PATTERNS), re.I)

# Topical buckets. Keyword match on the attribute name; labelled heuristic in the output.
TOPICS = {
    "composer": ["composer"],
    "justification": ["justification", "word spacing", "letter spacing", "glyph scaling",
                      "single word", "balance ragged", "align to", "justif"],
    "hyphenation": ["hyphen", "capitalized words", "ladder", "shortest word",
                    "min word letters", "after first", "before last"],
    "kinsoku": ["kinsoku"],
    "mojikumi": ["mojikumi"],
    "grid_alignment": ["grid align", "grid gyoudori", "align to baseline", "baseline grid",
                       "grid", "gyoudori"],
    "opentype": ["opentype", "ligature", "discretionary", "swash", "titling", "ordinal",
                 "fraction", "contextual", "stylistic set", "figure style", "position",
                 "slashed zero", "alternate", "small cap", "all caps", "superscript",
                 "subscript"],
    "cjk": ["cjk", "ruby", "tatechuyoko", "warichu", "kenten", "shatai", "tsume",
            "rensuuji", "jidori", "burasagari", "rotate single byte", "kumi", "mojizume",
            "leading model", "vertical", "japanese", "kanji", "kana", "bunri"],
    "indent_spacing": ["indent", "space before", "space after", "leading", "left margin",
                       "right margin", "first line"],
    "rules_and_decoration": ["rule above", "rule below", "underline", "strike through",
                             "shading", "border", "outline", "shadow"],
    "kerning_tracking": ["kerning", "tracking", "pair kern"],
    "size_and_scale": ["point size", "horizontal scale", "vertical scale", "baseline shift",
                       "skew", "leading"],
    "style_features": ["nested", "grep style", "line style", "drop cap", "bullet",
                       "numbering", "based on", "next style", "style"],
    "tabs": ["tab list", "tab stop", "align on", "leader"],
    "span_split": ["span", "split column", "keep", "start paragraph"],
    "footnote_endnote": ["footnote", "endnote"],
    "language_spelling": ["language", "dictionary", "hyphenation exception", "spell",
                          "autocorrect", "no break"],
    "find_change": ["find", "change"],
    "frame_and_flow": ["column", "inset", "vertical justification", "text frame",
                       "first baseline", "overflow", "flow", "wrap"],
}


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dom", type=Path, required=True)
    ap.add_argument("--root", type=Path, required=True,
                    help="InDesign install root, used only for the PMST value vocabulary")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    dom = json.loads(args.dom.read_text(encoding="utf-8"))
    props = {p["script_id"]: p for p in dom["properties"]}
    meths = {m["script_id"]: m for m in dom["methods"]}
    enums = {e["script_id"]: e for e in dom["enumerations"]}

    # ---- primitive type evidence: which property names use each unnamed type id ---------
    type_use = collections.defaultdict(list)
    for p in dom["properties"]:
        type_use[p["type_id"]].append(p["name"])
    prim_evidence = []
    for tid, hint in dom["primitive_type_hints"].items():
        n = int(tid, 16)
        names = type_use.get(n, [])
        prim_evidence.append({"type_id_hex": tid, "inferred_name": hint,
                              "basis": "heuristic, inferred from the members that use it",
                              "property_uses": len(names),
                              "example_properties": sorted(set(names))[:14]})
    prim_evidence.sort(key=lambda x: -x["property_uses"])

    def expand_type(t, tid):
        out = dict(t)
        if tid in enums:
            e = enums[tid]
            out["enumeration"] = {
                "script_id": tid, "tag": e["tag"], "name": e["name"],
                "description": e["description"],
                "values": [{"name": x["name"], "tag": x["tag"],
                            "description": x["description"],
                            **({"added_by_plugin": x["added_by_plugin"]}
                               if "added_by_plugin" in x else {})}
                           for x in e["enumerators"]],
            }
        return out

    # ---- text classes ---------------------------------------------------------------------
    classes = []
    for c in dom["classes"]:
        if not CLASS_RE.search(c["name"]):
            continue
        cp = []
        for m in c["properties"]:
            p = props.get(m["id"])
            if not p:
                continue
            cp.append({
                "name": p["name"], "tag": p["tag"], "script_id": p["script_id"],
                "description": p["description"],
                "type_id": p["type_id"], "type": expand_type(p["type"], p["type_id"]),
                "flags": p["flags"], "plugin": p["plugin"],
                "version_stamp": p["version_stamp"],
            })
        cm = []
        for m in c["methods"]:
            f = meths.get(m["id"])
            if not f:
                continue
            cm.append({
                "name": f["name"], "tag": f["tag"], "script_id": f["script_id"],
                "description": f["description"],
                "reply": {"description": f["reply"]["description"],
                          "type": expand_type(f["reply"]["type"], f["reply"]["type_id"])},
                "parameters": [
                    {"name": pp["name"], "tag": pp["tag"], "description": pp["description"],
                     "type": expand_type(pp["type"], pp["type_id"]),
                     "trailer_status": pp.get("trailer_status"),
                     **({"default_enumerator_tag": pp["default_enumerator_tag"]}
                        if "default_enumerator_tag" in pp else {}),
                     **({"default_raw": pp["default_raw"]} if "default_raw" in pp else {})}
                    for pp in f["parameters"]
                ],
            })
        classes.append({
            "name": c["name"], "tag": c["tag"], "script_id": c["script_id"],
            "plural_name": c["plural_name"], "description": c["description"],
            "plugin": c["plugin"], "suite_id": c["suite_id"],
            "property_count": len(cp), "method_count": len(cm),
            "properties": sorted(cp, key=lambda x: x["name"]),
            "methods": sorted(cm, key=lambda x: x["name"]),
        })
    classes.sort(key=lambda c: -c["property_count"])

    # ---- flat attribute vocabulary across those classes -----------------------------------
    attr: dict[int, dict] = {}
    for c in classes:
        for p in c["properties"]:
            a = attr.setdefault(p["script_id"], {**p, "owning_classes": []})
            a["owning_classes"].append(c["name"])
    attributes = sorted(attr.values(), key=lambda x: x["name"])

    # ---- topical grouping (heuristic) ------------------------------------------------------
    topics = {}
    for topic, keys in TOPICS.items():
        sel = [a["name"] for a in attributes
               if any(k in a["name"].lower() for k in keys)]
        topics[topic] = {"selector_keywords": keys, "basis": "heuristic keyword match",
                         "attribute_count": len(sel), "attributes": sorted(set(sel))}

    # ---- enumerations reachable from the text model ----------------------------------------
    used_enum_ids = {a["type_id"] for a in attributes if a["type_id"] in enums}
    for c in classes:
        for m in c["methods"]:
            for pp in m["parameters"]:
                e = pp["type"].get("enumeration")
                if e:
                    used_enum_ids.add(e["script_id"])
    text_enums = []
    for eid in sorted(used_enum_ids):
        e = enums[eid]
        text_enums.append({
            "script_id": eid, "tag": e["tag"], "name": e["name"],
            "description": e["description"], "plugin": e["plugin"],
            "value_count": len(e["enumerators"]),
            "values": [{"name": x["name"], "tag": x["tag"], "description": x["description"]}
                       for x in e["enumerators"]],
        })
    text_enums.sort(key=lambda e: e["name"])

    # ---- PMST-sourced value vocabulary ------------------------------------------------------
    pmst_vocab = {k: [] for k in PMST_VALUE_PROBES}
    pmst_seen = set()
    pmst_files = [f for f in args.root.rglob("*.idrc") if f.parent.name == "idrc_PMST"]
    pmst_en = 0
    for f in pmst_files:
        try:
            with f.open("rb") as fh:
                head = fh.read(4)
                if len(head) < 4 or struct.unpack("<I", head)[0] not in ENGLISH_LOCALES:
                    continue
                rec = parse_pmst(head + fh.read())
        except Exception:  # noqa: BLE001
            continue
        if not rec:
            continue
        pmst_en += 1
        plug = plugin_of(f)
        for k, v in rec["strings"].items():
            hay = (k + " " + v).lower()
            for bucket, keys in PMST_VALUE_PROBES.items():
                if any(kw in hay for kw in keys):
                    sig = (bucket, k, v)
                    if sig in pmst_seen:
                        continue
                    pmst_seen.add(sig)
                    pmst_vocab[bucket].append({"key": k, "english": v, "plugin": plug})
    for b in pmst_vocab:
        pmst_vocab[b].sort(key=lambda x: (x["plugin"], x["key"]))

    plug_hist = collections.Counter(a["plugin"] for a in attributes)

    doc = {
        "schema_id": "handshake.reference.indesign_text_model@1",
        "generated_at": now(),
        "derived_from": str(args.dom),
        "upstream_schema": dom["schema_id"],
        "upstream_generated_at": dom["generated_at"],
        "method": (
            "Projection of the parsed InDesign scripting object model onto the text engine. "
            "Every class name, attribute name, description, value type and enumerator below is "
            "PARSED from the installed idrc_SCE2 resources by indesign-sce2-parse.py; the "
            "application was never launched. Two things are heuristic and labelled: (1) which "
            "classes count as 'text model' - the selection is a documented regex list, given in "
            "class_selection_patterns, applied to the scripting class name; (2) the topical "
            "buckets in topics[], which are keyword matches on attribute names and exist only as "
            "an index, not as an authority. Primitive type names in primitive_type_evidence are "
            "inferred from the members that use each unnamed type id, and the evidence for each "
            "inference is included so it can be checked."
        ),
        "class_selection_patterns": CLASS_PATTERNS,
        "totals": {
            "text_classes": len(classes),
            "distinct_attributes": len(attributes),
            "class_attribute_edges": sum(c["property_count"] for c in classes),
            "class_method_edges": sum(c["method_count"] for c in classes),
            "enumerations_referenced": len(text_enums),
            "enumerator_values": sum(e["value_count"] for e in text_enums),
            "attributes_with_enumerated_type": sum(
                1 for a in attributes if a["type"].get("enumeration")),
            "plugins_contributing": len(plug_hist),
            "pmst_english_files_used": pmst_en,
            "pmst_vocabulary_entries": sum(len(v) for v in pmst_vocab.values()),
        },
        "pmst_value_vocabulary": {
            "_source": "English (locale_id 1 or 2) idrc_PMST string tables. These are UI value "
                       "names that are NOT declared as SCE2 enumerators - composer names, "
                       "kinsoku and mojikumi set names and the related dialog vocabulary - "
                       "selected by keyword. Selection is heuristic; every entry keeps its "
                       "PMST key and owning plug-in so it can be verified.",
            "_probes": PMST_VALUE_PROBES,
            **{k: v for k, v in pmst_vocab.items()},
        },
        "attributes_by_plugin": [{"plugin": k, "attributes": v} for k, v in plug_hist.most_common()],
        "primitive_type_evidence": prim_evidence,
        "topics": topics,
        "classes": classes,
        "attributes": attributes,
        "enumerations": text_enums,
    }
    outp = args.out / "indesign_text_model.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    t = doc["totals"]
    print(f"[text] classes={t['text_classes']} attributes={t['distinct_attributes']} "
          f"edges={t['class_attribute_edges']} methods={t['class_method_edges']}")
    print(f"[text] enums={t['enumerations_referenced']} values={t['enumerator_values']} "
          f"enum-typed attrs={t['attributes_with_enumerated_type']}")
    print(f"[text] pmst vocabulary entries={t['pmst_vocabulary_entries']} "
          f"from {t['pmst_english_files_used']} english PMST files")
    print(f"[text] -> {outp} ({outp.stat().st_size/1048576:.1f} MB)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
