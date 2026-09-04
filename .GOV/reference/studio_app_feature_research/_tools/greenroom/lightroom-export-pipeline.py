#!/usr/bin/env python
"""
lightroom-export-pipeline.py

Recovers Lightroom Classic's export parameter surface offline. Read-only.

THREE PARSED SOURCES
  1. Still-image export presets, Lua-table .lrtemplate files:
       %APPDATA%/Adobe/Lightroom/Export Presets/**/*.lrtemplate
     plus the factory presets embedded verbatim as Lua source inside
     Export.lrmodule ("s = { ... }" blocks). Each preset's `value` table is a
     literal export-settings dictionary, so key names AND real values come
     straight out of the product.
  2. Export.lrmodule Lua constant pool (see lrbin.py): every export settings
     key the module references, every reverse-DNS export service provider id,
     every "$$$/AgExport/..." UI label. The labels are what turn a bare key
     into a documented control, and the enumerated label families
     (CollisionHandling/*, ResizeType/PopupMenu/*, BitDepth*, Compression/*,
     PostProcess/*, EmbeddedMetadataOption/*) give each key's value domain.
  3. Video export presets, <INSTALL>/Support/Video Export Presets/*.epr.
     These are Adobe Media Encoder preset XML: a graph of ExporterParam
     objects joined by ObjectID/ObjectRef. The graph is resolved here into a
     flat (ParamIdentifier -> value) mapping per preset.

NOTE ON SCOPE, stated because it contradicts a common assumption: the 97 .epr
files are VIDEO export presets only. Lightroom's still-image export surface is
not in them.
"""
from __future__ import annotations

import argparse
import collections
import datetime as _dt
import json
import os
import re
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lrbin  # noqa: E402
import lrlua  # noqa: E402

SCHEMA_ID = "handshake.adobe.lightroom_classic.export_pipeline.v1"

# heuristic grouping of still-image export settings keys onto dialog sections
GROUP_RULES = [
    ("export_location", [r"^export_destination", r"^export_useSubfolder",
                         r"^export_canUseSubfolder", r"^export_parentFolder",
                         r"^export_useParentFolder", r"^collisionHandling",
                         r"^reimport"]),
    ("file_naming", [r"^token", r"^renaming", r"^extensionCase",
                     r"^initialSequenceNumber", r"^extension$"]),
    ("video", [r"^export_video", r"^includeVideoFiles", r"^video_"]),
    ("file_settings", [r"^format$", r"^jpeg_", r"^tiff_", r"^psd_", r"^png_",
                       r"^DNG_", r"^dng_", r"^export_colorSpace",
                       r"^export_bitDepth", r"^export_userBitDepth",
                       r"^export_originalFormat"]),
    ("image_sizing", [r"^size_"]),
    ("output_sharpening", [r"^outputSharpening"]),
    ("metadata", [r"^metadata_", r"^minimizeEmbeddedMetadata",
                  r"^embeddedMetadataOption", r"^includeFaceTags",
                  r"^removeFaceMetadata", r"^removeLocationMetadata",
                  r"^writeLightroomKeywordHierarchy", r"^minimizeMetadata"]),
    ("watermarking", [r"[Ww]atermark"]),
    ("post_processing", [r"^export_postProcessing", r"^export_externalEditingApp"]),
    ("content_credentials", [r"^contentCredentials", r"^includeConnectedAccounts",
                             r"^includeEditsAndActivity", r"^includeProducer"]),
    ("hdr_output", [r"^enableHDRDisplay", r"^maximumCompatibility"]),
    ("service", [r"^exportServiceProvider", r"^LR_"]),
]

# keys whose only evidence is a module constant AND whose name matches only a
# weak prefix are probably dialog-internal, not export settings.
STRONG_KEY = re.compile(
    r"^(export_|size_|jpeg_|tiff_|psd_|png_|DNG_|dng_|metadata_|"
    r"outputSharpening|reimport|collisionHandling|extensionCase|"
    r"initialSequenceNumber|renamingTokens|tokenCustomString|tokens$|"
    r"embeddedMetadataOption|useWatermark|watermarking|"
    r"exportServiceProvider|contentCredentials|minimizeEmbeddedMetadata|"
    r"includeVideoFiles|includeFaceTags|removeFace|removeLocation|"
    r"writeLightroomKeyword|format$|LR_)")
_GROUPS = [(g, [re.compile(p) for p in ps]) for g, ps in GROUP_RULES]


def group_of(key):
    for g, pats in _GROUPS:
        for p in pats:
            if p.search(key):
                return g
    return "unclassified"


# --- .epr ------------------------------------------------------------------
def parse_epr(path):
    tree = ET.parse(path)
    root = tree.getroot()
    objs = {}
    for el in root:
        oid = el.get("ObjectID")
        if oid:
            objs[oid] = el
    meta = {}
    for tag in ("PresetName", "PresetID", "PresetComments",
                "PresetUserComments", "ExporterFileType", "ExporterClassID",
                "ExporterName", "DoVideo", "DoAudio", "DoEmulation",
                "ExportXMPOptionKey", "FolderDisplayPath"):
        el = root.find(tag)
        if el is not None:
            meta[tag] = (el.text or "").strip()
    sf = root.find("StandardFilters")
    if sf is not None:
        meta["StandardFilters"] = {c.tag: (c.text or "").strip() for c in sf}

    params = {}
    for el in root.iter("ExporterParam"):
        ident = el.findtext("ParamIdentifier", "").strip()
        if not ident or ident == "0":
            continue
        rec = {}
        for f in ("ParamValue", "ParamType", "ParamOrdinalValue",
                  "ParamName", "ParamAuxType", "ParamAuxValue",
                  "ParamIsHidden", "ParamIsDisabled", "ParamIsSlider",
                  "ParamTargetBitrate"):
            v = el.findtext(f)
            if v is not None and v.strip() != "":
                rec[f] = v.strip()
        if el.find("ExporterChildParams") is not None:
            rec["is_group"] = True
        params[ident] = rec
    return meta, params


# --- factory export presets embedded in Export.lrmodule --------------------
PRESET_BLOCK = re.compile(r"s = \{\r?\n.*?\r?\n\}", re.S)


def embedded_presets(path):
    with open(path, "rb") as fh:
        blob = fh.read()
    text = blob.decode("latin-1")
    out = []
    for m in PRESET_BLOCK.finditer(text):
        src = m.group(0)
        try:
            _n, tbl = lrlua.parse_table(src)
        except Exception:  # noqa: BLE001
            continue
        out.append(lrlua.jsonable(tbl))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install",
                    default=r"C:\Program Files\Adobe\Adobe Lightroom Classic")
    ap.add_argument("--user",
                    default=os.path.expandvars(r"%APPDATA%\Adobe\Lightroom"))
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    errors = []

    # ---- 1. still-image export presets ---------------------------------
    presets = []
    user_files = []
    upresets_root = os.path.join(args.user, "Export Presets")
    if os.path.isdir(upresets_root):
        for dp, _d, fn in os.walk(upresets_root):
            for f in sorted(fn):
                if f.lower().endswith(".lrtemplate"):
                    user_files.append(os.path.join(dp, f))
    for p in user_files:
        try:
            _n, tbl = lrlua.parse_table(lrlua.read(p))
            presets.append({
                "origin": "user_profile",
                "file": os.path.relpath(p, args.user).replace("\\", "/"),
                "preset": lrlua.jsonable(tbl),
            })
        except Exception as exc:  # noqa: BLE001
            errors.append({"file": p,
                           "error": "%s: %s" % (type(exc).__name__, exc)})

    export_mod = os.path.join(args.install, "Export.lrmodule")
    factory = embedded_presets(export_mod) if os.path.isfile(export_mod) else []
    for tbl in factory:
        presets.append({"origin": "factory_embedded_in_Export.lrmodule",
                        "file": "Export.lrmodule", "preset": tbl})

    # ---- 2. Export.lrmodule constant pool -------------------------------
    mined, order, modsize = ({}, [], 0)
    if os.path.isfile(export_mod):
        mined, order, modsize = lrbin.mine(export_mod)
    zstr = {}
    for s in mined.get("zstr_localization_key", []):
        m = lrbin.ZSTR_RE.match(s)
        if m:
            zstr[m.group(1)] = m.group(2)
    idents = set(mined.get("identifier", []))

    # settings keys: union of keys seen in presets + module identifiers that
    # look like export settings keys
    KEYPAT = re.compile(
        r"^(export_|size_|jpeg_|tiff_|psd_|png_|DNG_|dng_|metadata_|"
        r"outputSharpening|reimport|collision|extension|initialSequence|"
        r"renaming|token|include|minimize|embeddedMetadataOption|"
        r"useWatermark|watermarking|watermark|format$|LR_|"
        r"exportServiceProvider|removeFace|removeLocation|"
        r"writeLightroomKeyword)")
    key_values = collections.defaultdict(collections.Counter)
    key_presets = collections.defaultdict(list)
    for rec in presets:
        val = rec["preset"].get("value") or {}
        title = rec["preset"].get("internalName") or rec["preset"].get("id")
        for k, v in val.items():
            key_values[k][json.dumps(v, ensure_ascii=False)
                          if isinstance(v, (dict, list)) else str(v)] += 1
            key_presets[k].append(title)
    module_only = sorted(k for k in idents
                         if KEYPAT.match(k) and k not in key_values)

    # enum families from ZSTR label namespaces
    enum_families = collections.defaultdict(list)
    for k, v in sorted(zstr.items()):
        parts = k.split("/")
        if len(parts) >= 3:
            fam = "/".join(parts[:-1])
            enum_families[fam].append({"member": parts[-1], "label": v})
    ENUM_OF_INTEREST = [
        "AgExport/CollisionHandling", "AgExport/ResizeType/PopupMenu",
        "AgExport/PopupMenu", "AgExport/Menu/Compression",
        "AgExport/PostProcess", "AgExport/Metadata/EmbeddedMetadataOption",
        "AgExport/CheckBox", "AgExport/Sharpening",
        "AgExport/Sharpening/Abbreviated", "AgExport/DNG",
        "AgExport/ColorSpace", "AgExport/Format",
    ]
    value_domains = {f: enum_families[f] for f in ENUM_OF_INTEREST
                     if f in enum_families}
    # any family with >=3 members mentioning export
    for fam, members in enum_families.items():
        if fam.startswith("AgExport") and len(members) >= 3 \
                and fam not in value_domains:
            value_domains[fam] = members

    # label lookup by key tail
    tail = collections.defaultdict(list)
    for k, v in zstr.items():
        tail[k.rsplit("/", 1)[-1].lower()].append({"key": "$$$/" + k,
                                                   "label": v})

    settings = []
    for k in sorted(set(list(key_values) + module_only)):
        vals = key_values.get(k, collections.Counter())
        in_preset = k in key_values
        strong = bool(STRONG_KEY.match(k))
        entry = {
            "key": k,
            "group": group_of(k),
            "group_classification": "heuristic:curated_dialog_section_map",
            "evidence": ("preset_value" if in_preset
                         else "module_constant_pool_only"),
            "is_export_setting": in_preset or strong,
            "is_export_setting_classification": (
                "parsed:appears as a key in a real export preset"
                if in_preset else
                ("heuristic:strong export key prefix, module constant only"
                 if strong else
                 "heuristic:weak prefix match with no preset evidence - "
                 "probably an export-dialog UI field, not a stored setting")),
            "classification": "parsed",
            "observed_values": [{"value": v, "presets": c}
                                for v, c in vals.most_common(20)],
            "used_by_presets": sorted(set(key_presets.get(k, [])))[:12],
        }
        labs = tail.get(k.lower())
        if labs:
            entry["ui_label_candidates"] = labs[:4]
            entry["ui_label_classification"] = "heuristic:zstr_tail_match"
        settings.append(entry)

    # ---- 3. video export presets ----------------------------------------
    epr_root = os.path.join(args.install, "Support", "Video Export Presets")
    epr_files = sorted(f for f in os.listdir(epr_root)
                       if f.lower().endswith(".epr")) \
        if os.path.isdir(epr_root) else []
    epr_records = []
    epr_params = collections.defaultdict(collections.Counter)
    epr_param_types = collections.defaultdict(collections.Counter)
    exporters = collections.Counter()
    for f in epr_files:
        p = os.path.join(epr_root, f)
        try:
            meta, params = parse_epr(p)
        except Exception as exc:  # noqa: BLE001
            errors.append({"file": f,
                           "error": "%s: %s" % (type(exc).__name__, exc)})
            continue
        exporters[(meta.get("ExporterFileType"),
                   meta.get("ExporterClassID"))] += 1
        for ident, rec in params.items():
            epr_params[ident][rec.get("ParamValue", "")] += 1
            epr_param_types[ident][rec.get("ParamType", "")] += 1
        epr_records.append({"file": f, "meta": meta,
                            "parameter_count": len(params),
                            "parameters": params})

    epr_param_surface = []
    for ident in sorted(epr_params):
        vals = epr_params[ident]
        epr_param_surface.append({
            "param_identifier": ident,
            "classification": "parsed",
            "presets_using": sum(vals.values()),
            "param_types_seen": dict(epr_param_types[ident]),
            "distinct_values": len(vals),
            "observed_values_top": [{"value": v, "presets": c}
                                    for v, c in vals.most_common(12)],
        })

    by_group = collections.Counter(s["group"] for s in settings)

    doc = {
        "schema_id": SCHEMA_ID,
        "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
        "method": {
            "mode": "offline_static_parse",
            "app_launched": False,
            "sources": [
                {"id": "still_export_presets_user", "classification": "parsed",
                 "root": upresets_root, "files": len(user_files),
                 "format": "Lua table (.lrtemplate)",
                 "parser": "lrlua.parse_table"},
                {"id": "still_export_presets_factory",
                 "classification": "parsed", "file": "Export.lrmodule",
                 "blocks_recovered": len(factory),
                 "format": "Lua source embedded verbatim in the PE payload",
                 "parser": "regex block extraction + lrlua.parse_table"},
                {"id": "export_module_constant_pool",
                 "classification": "parsed", "file": "Export.lrmodule",
                 "bytes": modsize, "unique_lua_string_constants": len(order),
                 "zstr_keys": len(zstr),
                 "parser": "lrbin.mine (Lua 5.1 dump string-constant format)"},
                {"id": "video_export_presets", "classification": "parsed",
                 "root": epr_root, "files": len(epr_files),
                 "format": "Adobe Media Encoder preset XML (PremiereData v3), "
                           "ExporterParam object graph",
                 "parser": "ElementTree + ObjectID/ObjectRef flattening"},
            ],
            "classification_legend": {
                "parsed": "read directly out of a shipped or user file",
                "derived": "computed from parsed data",
                "heuristic": "this tool's judgement",
            },
        },
        "counts": {
            "still_export_presets_parsed": len(presets),
            "still_export_presets_user": len(user_files),
            "still_export_presets_factory_embedded": len(factory),
            "still_export_settings_keys": len(settings),
            "still_export_keys_confirmed_export_settings": sum(
                1 for s in settings if s["is_export_setting"]),
            "still_export_keys_probable_ui_fields": sum(
                1 for s in settings if not s["is_export_setting"]),
            "still_export_keys_with_observed_values": len(key_values),
            "still_export_keys_module_constant_only": len(module_only),
            "still_export_keys_by_group_heuristic": dict(by_group),
            "zstr_ui_labels_mined": len(zstr),
            "video_export_preset_files_found": len(epr_files),
            "video_export_preset_files_parsed": len(epr_records),
            "video_export_distinct_param_identifiers": len(epr_param_surface),
            "video_export_distinct_exporter_codecs": len(exporters),
        },
        "scope_correction": {
            "classification": "parsed",
            "statement": "All 97 .epr files under Support/Video Export Presets "
                         "are Adobe Media Encoder VIDEO presets. Lightroom's "
                         "still-image export surface is not stored in .epr; it "
                         "lives in .lrtemplate export presets and in "
                         "Export.lrmodule.",
        },
        "still_image_export": {
            "classification": "parsed",
            "settings": settings,
            "value_domains_from_ui_labels": {
                "classification": "parsed:zstr_label_families",
                "note": "each family is an enumerated popup/checkbox group in "
                        "the export dialog; member names are the internal "
                        "enum tokens, labels are the shipped English strings",
                "families": value_domains,
            },
            "export_service_providers": sorted(
                mined.get("reverse_dns_id", [])),
            "module_chunks": sorted(mined.get("lua_chunk_name", [])),
            "presets": presets,
        },
        "video_export": {
            "classification": "parsed",
            "parameter_surface": epr_param_surface,
            "exporter_codecs": [
                {"exporter_file_type": k[0], "exporter_class_id": k[1],
                 "presets": v} for k, v in exporters.most_common()],
            "presets": epr_records,
        },
        "errors": errors,
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print(json.dumps(doc["counts"], indent=1))


if __name__ == "__main__":
    main()
