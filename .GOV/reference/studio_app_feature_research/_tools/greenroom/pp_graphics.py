"""pp_graphics.py -- Essential Graphics, titles and the text engine, offline.

Streams:
  G1  install/Essential Graphics/**/*.mogrt
      A .mogrt is a zip. definition.json is the Motion Graphics Template model:
      capsule identity, localized names, clientControls (the exposed parameter
      surface), usedEffects, usedFonts, usedFileTypes, usedCompRenderers,
      platformSupport and the responsive-design version. project.prgraphic is
      the graphic itself (a nested zip holding a gzip'd PremiereData project).
  G2  executable string table
      $$$/MotionGraphics/, $$$/Premiere/Graphics/, $$$/Premiere/Titler/,
      $$$/MediaCore/AEFilters/Graphics/ and the caption namespaces: the title
      and caption parameter vocabulary.
  G3  install/typesupport/
      CID CMaps and the type-support data the text engine needs for CJK.
  G4  install/otf and install/ttf
      shipped font files, with the name table read out of each font.
  G5  install/TextProcessing/
      classified: language-detection support is in scope, the transcription
      filter vocabulary belongs to an excluded AI surface.
"""
import collections
import json
import os
import re
import struct
import sys
import traceback
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

# Motion Graphics Template clientControl type codes. Derived by tabulating,
# per code, the keys the control carries and the value shape it holds across
# every shipped template; the tabulation ships as client_control_type_evidence.
MOGRT_CONTROL_TYPE = {
    1: "checkbox (boolean)",
    2: "numeric slider (carries min and max)",
    4: "colour (linear RGBA, four floats 0..1)",
    6: "source text (localized string, carries font edit permissions)",
    8: "group / layer section header (value is the layer kind)",
}
MOGRT_TYPE_CONFIDENCE = "heuristic"


def loc_strings(node):
    """{'strDB':[{'localeString':..,'str':..}]} -> {locale: text}."""
    if not isinstance(node, dict):
        return None
    db = node.get("strDB")
    if not isinstance(db, list):
        return None
    return {e.get("localeString"): e.get("str") for e in db
            if isinstance(e, dict)}


def parse_mogrt(path):
    z = zipfile.ZipFile(path)
    names = z.namelist()
    if "definition.json" not in names:
        raise ValueError("no definition.json in %s" % path)
    d = json.loads(z.read("definition.json").decode("utf-8"))

    controls = []
    for c in d.get("clientControls") or []:
        t = c.get("type")
        ui = loc_strings(c.get("uiName")) or {}
        tip = loc_strings(c.get("uiToolTip")) or {}
        suf = loc_strings(c.get("uiSuffix")) or {}
        val = c.get("value")
        val_loc = loc_strings(val)
        rec = {
            "control_id": c.get("id"),
            "type_code": t,
            "type": MOGRT_CONTROL_TYPE.get(t, "unknown"),
            "type_confidence": MOGRT_TYPE_CONFIDENCE,
            "name": ui.get("en_US") or next(iter(ui.values()), None),
            "names_by_locale": ui or None,
            "tooltip": tip.get("en_US") or None,
            "suffix": suf.get("en_US") or None,
            "can_animate": c.get("canAnimate"),
            "hidden": c.get("hidden"),
            "min": c.get("min"),
            "max": c.get("max"),
            "default": (val_loc.get("en_US") if val_loc else val),
            "default_by_locale": val_loc or None,
        }
        fe = c.get("fonteditinfo")
        if isinstance(fe, dict):
            rec["font_editing"] = {
                "font_family_editable": fe.get("capPropFontEdit"),
                "faux_style_editable": fe.get("capPropFontFauxStyleEdit"),
                "font_size_editable": fe.get("capPropFontSizeEdit"),
                "font": fe.get("fontEditValue") or None,
                "font_size": fe.get("fontSizeEditValue"),
                "all_caps": fe.get("fontFSAllCapsValue"),
                "small_caps": fe.get("fontFSSmallCapsValue"),
                "bold": fe.get("fontFSBoldValue"),
                "italic": fe.get("fontFSItalicValue"),
            }
        controls.append({k: v for k, v in rec.items() if v is not None})

    cap = loc_strings(d.get("capsuleNameLocalized")) or {}
    # usedFontsLocalized is {locale: [postscript names]} -- the template names a
    # different face per locale so CJK renders with a CJK family.
    fonts = d.get("usedFontsLocalized")
    fonts_by_locale = {}
    font_names = []
    if isinstance(fonts, dict):
        for loc, lst in fonts.items():
            names = [str(x) for x in lst] if isinstance(lst, list) else [str(lst)]
            fonts_by_locale[loc] = names
        font_names = sorted({n for v in fonts_by_locale.values() for n in v})
    elif isinstance(fonts, list):
        for f in fonts:
            s = loc_strings(f) if isinstance(f, dict) else None
            font_names.append((s or {}).get("en_US") if s else str(f))
        font_names = [f for f in font_names if f]
    src = loc_strings(d.get("sourceInfoLocalized")) or {}

    return {
        "template_name": cap.get("en_US") or d.get("capsuleName"),
        "names_by_locale": cap or None,
        "capsule_id": d.get("capsuleID"),
        "description": d.get("description") or None,
        "author_app": d.get("authorApp"),
        "api_version": d.get("apiVersion"),
        "responsive_design_version": d.get("responsiveDesignVersion"),
        "mobile_compatibility_version": d.get("mobileCompatibilityVersion"),
        "typekit_only_version": d.get("typekitOnlyVersion"),
        "internal_effects_only_version": d.get("internalEffectsOnlyVersion"),
        "aelib_compliant_version": d.get("aelibCompliantVersion"),
        "platform_support": d.get("platformSupport"),
        "tags": d.get("capsuleTags"),
        "source_info": src.get("en_US") or None,
        "used_effects": d.get("usedEffects") or [],
        "used_file_types": d.get("usedFileTypes") or [],
        "used_comp_renderers": d.get("usedCompRenderers") or [],
        "used_fonts": font_names,
        "used_fonts_by_locale": fonts_by_locale or None,
        "exposed_controls": controls,
        "exposed_control_count": len(controls),
        "package_entries": names,
        "localized_project_variants": sorted(
            n for n in names if n.endswith(".prgraphic")),
        "thumbnail_locales": sorted(
            {re.sub(r"^thumb_?", "", os.path.splitext(n)[0]) or "en_US"
             for n in names if n.startswith("thumb")}),
    }


# ---------------------------------------------------------------------------
def read_font_names(path):
    """Read the OpenType/TrueType `name` table. Pure struct reads, no shaping."""
    with open(path, "rb") as fh:
        data = fh.read()
    if len(data) < 12:
        return {"error": "too short"}
    tag = data[:4]
    if tag == b"ttcf":
        return {"format": "TrueType Collection", "note": "not unpacked"}
    try:
        num_tables = struct.unpack_from(">H", data, 4)[0]
        off = 12
        tables = {}
        for _ in range(num_tables):
            t, _cs, o, ln = struct.unpack_from(">4sIII", data, off)
            tables[t] = (o, ln)
            off += 16
        if b"name" not in tables:
            return {"error": "no name table"}
        no, _nl = tables[b"name"]
        fmt, count, str_off = struct.unpack_from(">HHH", data, no)
        out = {}
        for i in range(count):
            (pid, eid, lid, nid, length, o) = struct.unpack_from(
                ">6H", data, no + 6 + i * 12)
            raw = data[no + str_off + o: no + str_off + o + length]
            try:
                txt = (raw.decode("utf-16-be") if pid == 3
                       else raw.decode("mac-roman" if pid == 1 else "latin-1"))
            except Exception:                          # noqa: BLE001
                continue
            if lid not in (0, 0x409):
                continue
            key = {1: "family", 2: "subfamily", 3: "unique_id", 4: "full_name",
                   5: "version", 6: "postscript_name", 16: "typographic_family",
                   17: "typographic_subfamily"}.get(nid)
            if key and key not in out:
                out[key] = txt
        out["outline_format"] = ("CFF / PostScript" if b"CFF " in tables
                                 else "TrueType glyf")
        out["is_variable"] = b"fvar" in tables
        out["has_gsub"] = b"GSUB" in tables
        out["has_gpos"] = b"GPOS" in tables
        return out
    except Exception as exc:                           # noqa: BLE001
        return {"error": repr(exc)}


def main(out_dir):
    R = C.PREMIERE_ROOT
    table = C.premiere_strings(SCRATCH)
    sources = []
    failures = []

    # ---- G1 motion graphics templates
    templates = []
    eg_dir = os.path.join(R, "Essential Graphics")
    for p in sorted(C.walk_files(eg_dir, exts=(".mogrt",))):
        try:
            rec = parse_mogrt(p)
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "G1_mogrt", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        rec["file"] = C.rel(p)
        rel = C.rel(p)
        parts = rel.split("/")
        rec["category"] = parts[1] if len(parts) > 2 else "(root)"
        rec["bytes"] = os.path.getsize(p)
        templates.append(rec)
    sources.append({
        "id": "G1_mogrt", "path": C.rel(eg_dir),
        "how": ("zip entry definition.json read as JSON; clientControls "
                "flattened into an exposed parameter surface"),
        "templates_parsed": len(templates),
        "exposed_controls": sum(t["exposed_control_count"] for t in templates),
        "note": "one .mogrt file is exactly one template",
    })

    # evidence for the control-type enum
    type_ev = collections.defaultdict(
        lambda: {"count": 0, "keys": collections.Counter(),
                 "sample_names": [], "sample_values": []})
    for t in templates:
        for c in t["exposed_controls"]:
            ev = type_ev[c["type_code"]]
            ev["count"] += 1
            ev["keys"].update(k for k in c if k not in ("type", "type_confidence"))
            if c.get("name") and len(ev["sample_names"]) < 10 \
                    and c["name"] not in ev["sample_names"]:
                ev["sample_names"].append(c["name"])
            if len(ev["sample_values"]) < 4:
                ev["sample_values"].append(str(c.get("default"))[:90])

    effects_used = collections.Counter()
    fonts_used = collections.Counter()
    filetypes = collections.Counter()
    renderers = collections.Counter()
    for t in templates:
        effects_used.update(str(e) for e in t["used_effects"])
        fonts_used.update(t["used_fonts"])
        filetypes.update(str(x) for x in t["used_file_types"])
        renderers.update(str(x) for x in t["used_comp_renderers"])

    # ---- G2 string namespaces
    ns = {}
    for prefix, purpose in (
            ("$$$/MotionGraphics/", "motion graphics template model"),
            ("$$$/MediaCore/AEFilters/Graphics/", "the Graphics / text effect parameter surface"),
            ("$$$/Premiere/Graphics", "Essential Graphics panel"),
            ("$$$/Premiere/Titler", "legacy titler"),
            ("$$$/AE/Text", "text engine"),
            ("$$$/AE/EGG/", "essential graphics glue"),
            ("$$$/dvacaptioning/", "caption data model and caption formats"),
            ("$$$/Premiere/Captions", "captions UI"),
            ("$$$/AE/Path_Text/", "path text"),
            ("$$$/Premiere/FontManagement", "font management")):
        rows = {k: v for k, v in table.items()
                if k.startswith(prefix) and not C.looks_ai(k)}
        if rows:
            ns[prefix] = {"purpose": purpose, "count": len(rows),
                          "strings": dict(sorted(rows.items()))}
    sources.append({"id": "G2_exe_strings",
                    "how": ("namespace slice of the executable's $$$ literals; "
                            "keys naming an excluded AI surface are dropped"),
                    "namespaces": len(ns),
                    "strings": sum(v["count"] for v in ns.values())})

    # ---- G3 type support
    ts_dir = os.path.join(R, "typesupport")
    ts_groups = collections.Counter()
    ts_files = []
    for p in sorted(C.walk_files(ts_dir)):
        rel = C.rel(p)
        grp = rel.split("/")[1] if rel.count("/") >= 1 else "(root)"
        ts_groups[grp] += 1
        ts_files.append({"file": rel, "group": grp,
                         "bytes": os.path.getsize(p)})
    cmaps = sorted(os.path.basename(f["file"]) for f in ts_files
                   if f["group"] == "cmaps")
    sources.append({"id": "G3_typesupport", "path": C.rel(ts_dir),
                    "how": "directory inventory grouped by subdirectory",
                    "files": len(ts_files), "groups": dict(ts_groups)})

    # ---- G4 shipped fonts
    fonts = []
    for d in ("otf", "ttf"):
        fd = os.path.join(R, d)
        if not os.path.isdir(fd):
            continue
        for p in sorted(C.walk_files(fd, exts=(".otf", ".ttf", ".ttc"))):
            info = read_font_names(p)
            info.update({"file": C.rel(p), "container": d,
                         "bytes": os.path.getsize(p)})
            fonts.append(info)
    families = collections.Counter(f.get("typographic_family") or f.get("family")
                                   for f in fonts if f.get("family"))
    sources.append({"id": "G4_fonts",
                    "how": ("OpenType `name` table read directly with struct "
                            "unpacking; no font library is loaded"),
                    "font_files": len(fonts),
                    "families": len(families)})

    # ---- G5 text processing, classified
    tp_dir = os.path.join(R, "TextProcessing")
    tp = []
    for p in sorted(C.walk_files(tp_dir)):
        rel = C.rel(p)
        ai = C.looks_ai(rel) or "filter_vocab" in rel
        tp.append({
            "file": rel, "bytes": os.path.getsize(p),
            "in_scope": not ai,
            "classification": ("transcription filter vocabulary -- belongs to "
                               "an excluded AI surface" if ai
                               else "language detection support data"),
        })
    sources.append({"id": "G5_textprocessing", "path": C.rel(tp_dir),
                    "how": "inventory with per-file AI-scope classification",
                    "files": len(tp),
                    "in_scope_files": sum(1 for x in tp if x["in_scope"])})

    by_cat = collections.Counter(t["category"] for t in templates)

    payload = C.envelope(
        "handshake.studio.premiere.graphics_text.v1",
        {
            "summary": ("The motion-graphics template model and the text / title "
                        "parameter surface: every shipped .mogrt parsed into its "
                        "exposed parameter surface, plus the text-engine string "
                        "vocabulary, the shipped fonts and the CJK type-support "
                        "data."),
            "mogrt_model": (
                "A Motion Graphics Template is a zip. definition.json declares "
                "the capsule identity and the clientControls the template "
                "exposes to the Essential Graphics panel; each control has a "
                "type code, a localized display name, a default value, a "
                "can-animate flag, and for text controls a font-editing "
                "permission block that states which of family / size / faux "
                "style the editor may change. project.prgraphic holds the "
                "graphic itself as a nested zip containing a gzip'd PremiereData "
                "project, one per localized variant."),
            "confidence_legend": {
                "parsed": "read verbatim from definition.json or from a font's name table",
                "heuristic": ("the meaning of a clientControl type code, derived "
                              "from the keys and value shapes it carries; "
                              "client_control_type_evidence ships that tabulation"),
            },
            "known_gaps": [
                ("Only five clientControl type codes appear in the shipped "
                 "templates (1, 2, 4, 6, 8). The format allows others -- point, "
                 "angle, dropdown and font pickers exist in the Essential "
                 "Graphics panel -- but no shipped template uses them, so their "
                 "codes are not recoverable from this install."),
                ("project.prgraphic is reported as a container, not decoded into "
                 "a layer model: it is a whole After Effects-style composition "
                 "document rather than a specification of the graphics format."),
                ("Automatic caption GENERATION is an excluded AI surface. The "
                 "caption data model and the caption/subtitle template controls "
                 "are not AI and are included."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "motion_graphics_templates": len(templates),
                "exposed_controls_total": sum(
                    t["exposed_control_count"] for t in templates),
                "templates_by_category": dict(by_cat),
                "distinct_effects_used_by_templates": len(effects_used),
                "distinct_fonts_used_by_templates": len(fonts_used),
                "shipped_font_files": len(fonts),
                "shipped_font_families": len(families),
                "typesupport_files": len(ts_files),
                "cid_cmaps": len(cmaps),
                "textprocessing_files_in_scope": sum(
                    1 for x in tp if x["in_scope"]),
                "count_semantics": ("one .mogrt file is exactly one template; "
                                    "font and cmap counts are file counts "
                                    "because a font and a cmap are each one file"),
            },
            "client_control_type_enum": {
                "confidence": MOGRT_TYPE_CONFIDENCE,
                "values": {str(k): v for k, v in MOGRT_CONTROL_TYPE.items()},
            },
            "client_control_type_evidence": {
                str(code): {"count": ev["count"],
                            "keys_present": dict(ev["keys"]),
                            "sample_control_names": ev["sample_names"],
                            "sample_default_values": ev["sample_values"]}
                for code, ev in sorted(type_ev.items(),
                                       key=lambda kv: (kv[0] is None, kv[0]))
            },
            "effects_used_by_templates": dict(effects_used.most_common()),
            "fonts_used_by_templates": dict(fonts_used.most_common()),
            "file_types_used_by_templates": dict(filetypes),
            "comp_renderers_used_by_templates": dict(renderers),
            "motion_graphics_templates": templates,
            "shipped_fonts": fonts,
            "shipped_font_families": dict(families.most_common()),
            "type_support": {"groups": dict(ts_groups),
                             "cid_cmaps": cmaps,
                             "files": ts_files},
            "text_processing": tp,
            "string_namespaces": ns,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_graphics_text.json", payload)
    print("wrote", path, size, "bytes")
    print("templates", len(templates), "controls",
          sum(t["exposed_control_count"] for t in templates),
          "fonts", len(fonts), "cmaps", len(cmaps), "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
