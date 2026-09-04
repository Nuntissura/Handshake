"""pp_export.py -- Premiere / Media Encoder export pipeline, offline.

Every shipped .epr in both installs is parsed in full. An .epr is a
PremiereData document:

    <PremiereData Version="3">
      <PresetName>..</PresetName>  <PresetID>guid</PresetID>
      <ExporterFileType>1095321158</ExporterFileType>   fourcc, e.g. "AIFF"
      <ExporterClassID>1061109567</ExporterClassID>     fourcc, e.g. "????"
      <DoVideo/> <DoAudio/> <FolderDisplayPath/>
      <StandardFilters>  crop / deinterlace / render quality / preview
      <ExporterParamContainer ObjectID=..>  tree of
        <ExporterParam ObjectID=..>
          <ParamIdentifier>ADBEVideoTargetBitrate</ParamIdentifier>
          <ParamType>2</ParamType> <ParamValue>10</ParamValue>
          <ParamMinValue/> <ParamMaxValue/> <FloatDecimalPlaces/>
          <ParamIsSlider/> <ParamIsHidden/> <ParamIsDisabled/> ...
          <ExporterChildParams ObjectRef=..>   nested group

The presets are the only shipped statement of the exporter parameter surface,
so the surface is reconstructed by union across all of them: every identifier
that any preset sets, with its declared type, its min/max where a preset
declares one, and the full set of values the shipped presets use.

Human labels and descriptions come from the executable's own
$$$/MediaCore/Exporters/<Identifier>Name and .../<Identifier>Desc strings.
"""
import collections
import os
import re
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

# ParamType observed across all shipped presets. Derived by tabulating the
# value shape and the identifiers carrying each code; the raw tabulation ships
# as param_type_evidence so the derivation is checkable.
PARAM_TYPE = {
    "1": "container / group header (no value of its own)",
    "2": "signed integer",
    "3": "floating point",
    "4": "boolean",
    "5": "string",
    "6": "enumerated integer chosen from a constrained list",
    "7": "button / action (no stored value)",
    "8": "tab or section group",
    "9": "file or folder path string",
    "10": "opaque / arbitrary data",
    "11": "multi-instance group (repeating rows)",
    "12": "colour",
}
PARAM_TYPE_CONFIDENCE = "heuristic"

# Facet classification of the ~500 exporter identifiers. Prefix and token rules,
# applied in order; every row records which rule matched so the classification
# can be audited. Marked heuristic.
FACET_RULES = [
    ("captions", (r"Caption", r"Subtitle", r"CEA", r"SCC", r"SRT", r"STL",
                  r"MCC", r"Sidecar")),
    ("publishing_destination", (r"^PostEncode", r"Publish", r"Destination",
                                r"Upload", r"YouTube", r"Vimeo", r"Twitter",
                                r"Facebook", r"Behance", r"FTP", r"Creative"
                                r"Cloud", r"FrameIO", r"^fio")),
    ("colour", (r"ColorSpace", r"ColorPrimaries", r"ColorRange", r"HDR",
                r"MasteringDisplay", r"ContentLightLevel", r"Gamma",
                r"TransferCharacteristic", r"Matrix", r"LUT", r"Lumetri",
                r"ToneMap", r"RenderDeepColor", r"BitDepth", r"DeepColor")),
    ("rate_control", (r"Bitrate", r"BitrateEncoding", r"VBR", r"CBR",
                      r"TargetBitrate", r"MaxBitrate", r"MinBitrate",
                      r"Quality", r"QP", r"CRF", r"DataRate", r"Pass")),
    ("gop_structure", (r"Keyframe", r"KeyFrame", r"GOP", r"BFrame", r"MFrame",
                       r"IFrame", r"ClosedGOP", r"Reference")),
    ("video_codec", (r"VideoCodec", r"Profile", r"Level", r"Codec", r"Entropy",
                     r"Encoder", r"ProRes", r"DNx", r"AVCIntra", r"XAVC",
                     r"HEVC", r"H26", r"MPEG", r"Cineform")),
    ("video_frame", (r"VideoWidth", r"VideoHeight", r"VideoAspect", r"VideoFPS",
                     r"FieldType", r"Interlace", r"Scan", r"FrameRate",
                     r"PixelAspect", r"Resize", r"Scale", r"Crop", r"Rotation",
                     r"VideoTimeInterpolation")),
    ("audio", (r"^ADBEAudio", r"Audio", r"Channel", r"SampleRate",
               r"SampleType", r"Loudness", r"Amplitude", r"Dialog")),
    ("multiplexing", (r"Multiplexer", r"Mux", r"Stream", r"Container",
                      r"Interleave", r"Fragment", r"Segment", r"Chunk")),
    ("metadata", (r"Metadata", r"XMP", r"Timecode", r"^UseZeroTimecode",
                  r"Marker", r"ContentCredential", r"C2PA")),
    ("effects_and_filters", (r"Filter", r"Blur", r"Overlay", r"Watermark",
                             r"Burn")),
    ("performance", (r"SmartRender", r"Render", r"GPU", r"Hardware",
                     r"Performance", r"Threads", r"Preview", r"Import")),
    ("vr_immersive", (r"VR", r"Immersive", r"Ambisonic", r"Stereoscopic",
                      r"Projection", r"360")),
    ("layout", (r"Group$", r"TabGroup", r"MultiGroup", r"Divider", r"Header")),
]


def classify(identifier):
    for facet, pats in FACET_RULES:
        for p in pats:
            if re.search(p, identifier):
                return facet, p
    return "other", None


def _num(v):
    if v is None or v == "":
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return v
    return int(f) if f == int(f) and abs(f) < 1e15 else f


# --- identifier -> human label / description -------------------------------
# Premiere spreads exporter labels over several namespaces and does not always
# key them by the full ParamIdentifier. The chain below is tried in order and
# the winning strategy is recorded on every row.
EXP_NS = "$$$/MediaCore/Exporters/"
HOST_NS = "$$$/MediaCore/MediaLayer/ExporterHost/Exporter/"


def _strip_ident(ident):
    s = ident
    if s.startswith("ADBE"):
        s = s[4:]
    return s


def resolve_label(ident, table):
    base = _strip_ident(ident)
    trials = [
        (EXP_NS + ident + "Name", "MediaCore/Exporters/<Identifier>Name"),
        (EXP_NS + ident, "MediaCore/Exporters/<Identifier>"),
        (HOST_NS + ident, "ExporterHost/Exporter/<Identifier>"),
        (HOST_NS + base, "ExporterHost/Exporter/<Identifier minus ADBE>"),
        (HOST_NS + base.replace("Group", ""), "ExporterHost/Exporter/<stripped, minus Group>"),
    ]
    for key, how in trials:
        if key in table:
            return table[key], how
    return None, None


def resolve_description(ident, table):
    base = _strip_ident(ident)
    trials = [
        (EXP_NS + ident + "Desc", "MediaCore/Exporters/<Identifier>Desc"),
        (EXP_NS + ident + "Description", "MediaCore/Exporters/<Identifier>Description"),
        (HOST_NS + base + "Description", "ExporterHost/Exporter/<stripped>Description"),
        (HOST_NS + base, "ExporterHost/Exporter/<stripped> (tooltip text)"),
    ]
    for key, how in trials:
        if key in table:
            return table[key], how
    return None, None


def collect_enum_vocabularies(table):
    """ExporterHost/Exporter/<Param>/<Option> = label -> option vocabularies."""
    out = collections.defaultdict(dict)
    for k, v in table.items():
        if not k.startswith(HOST_NS):
            continue
        tail = k[len(HOST_NS):]
        if "/" not in tail:
            continue
        param, option = tail.split("/", 1)
        out[param][option] = v
    return {k: v for k, v in out.items() if len(v) > 1}


def collect_exporter_namespaces(table):
    """Group $$$/MediaCore/Exporters/<Exporter>/... by exporter module."""
    out = collections.defaultdict(dict)
    for k, v in table.items():
        if not k.startswith(EXP_NS):
            continue
        tail = k[len(EXP_NS):]
        if "/" not in tail:
            out["(shared exporter parameters)"][tail] = v
            continue
        mod, rest = tail.split("/", 1)
        out[mod][rest] = v
    return {k: dict(sorted(v.items())) for k, v in sorted(out.items())}


BOOL_FIELDS = ("ParamIsHidden", "ParamIsDisabled", "ParamIsSlider",
               "ParamIsPassword", "ParamIsMultiLine", "ParamIsIndependant",
               "IsOptionalParam", "IsOptionalParamEnabled", "IsFilePathString",
               "IsParamPairGroup", "ParamDontSerializeValue",
               "ParamConstrainedListIsOptional", "ParamIsVerticallyAligned")


def parse_epr(path):
    objects, root = C.parse_premiere_data(path)
    top = {}
    for kid in root:
        tag = C._strip_ns(kid.tag)
        if len(kid) == 0 and kid.get("ObjectID") is None:
            top[tag] = (kid.text or "").strip()

    sf = root.find("StandardFilters")
    filters = C.flat_fields(sf) if sf is not None else {}

    params = []

    def walk(container_oid, path_names, depth=0):
        if depth > 25:
            return
        cont = objects.get(container_oid)
        if cont is None:
            return
        items = cont.find("ParamContainerItems")
        if items is None:
            return
        for it in items:
            ref = it.get("ObjectRef")
            pel = objects.get(ref)
            if pel is None:
                continue
            f = C.flat_fields(pel)
            ident = f.get("ParamIdentifier") or ""
            rec = {
                "identifier": ident or None,
                "index": _num(it.get("Index")),
                "group_path": list(path_names),
                "type_code": f.get("ParamType"),
                "type_label": PARAM_TYPE.get(f.get("ParamType"), "unknown"),
                "value": f.get("ParamValue") if "ParamValue" in f else None,
                "ordinal": _num(f.get("ParamOrdinalValue")),
                "name": f.get("ParamName") or None,
            }
            for src, dst in (("ParamMinValue", "min"), ("ParamMaxValue", "max"),
                             ("FloatDecimalPlaces", "decimal_places"),
                             ("ParamAuxType", "aux_type"),
                             ("ParamAuxValue", "aux_value"),
                             ("ParamTargetBitrate", "target_bitrate"),
                             ("ParamTargetID", "target_id"),
                             ("ParamFlags", "flags")):
                if f.get(src) not in (None, ""):
                    rec[dst] = _num(f.get(src))
            ui = {b: True for b in BOOL_FIELDS if f.get(b) == "true"}
            if ui:
                rec["ui_flags"] = sorted(ui)
            if pel.find("ParamArbData") is not None:
                rec["has_arbitrary_data"] = True
            params.append(rec)
            child = pel.find("ExporterChildParams")
            if child is not None and child.get("ObjectRef"):
                walk(child.get("ObjectRef"),
                     path_names + ([ident] if ident else []), depth + 1)

    epc = root.find("ExportParamContainer")
    if epc is not None and epc.get("ObjectRef"):
        walk(epc.get("ObjectRef"), [])
    else:
        for oid, el in objects.items():
            if C._strip_ns(el.tag) == "ExporterParamContainer":
                walk(oid, [])
                break

    ft = top.get("ExporterFileType")
    cid = top.get("ExporterClassID")
    return {
        "preset_name": top.get("PresetName") or None,
        "preset_id": top.get("PresetID") or None,
        "preset_comments": top.get("PresetComments") or None,
        "preset_user_comments": top.get("PresetUserComments") or None,
        "exporter_name": top.get("ExporterName") or None,
        "exporter_file_type_int": _num(ft),
        "exporter_file_type_fourcc": C.fourcc(ft),
        "exporter_class_id_int": _num(cid),
        "exporter_class_id_fourcc": C.fourcc(cid),
        "folder_display_path": top.get("FolderDisplayPath") or None,
        "does_video": top.get("DoVideo") == "true",
        "does_audio": top.get("DoAudio") == "true",
        "does_emulation": top.get("DoEmulation") == "true",
        "export_xmp_option": _num(top.get("ExportXMPOptionKey")),
        "standard_filters": {
            "crop_enabled": filters.get("CropState") == "true",
            "crop_type": _num(filters.get("CropType")),
            "crop_rect": filters.get("CropRect"),
            "deinterlace": filters.get("DeinterlaceState") == "true",
            "use_frame_blending": filters.get("UseFrameBlending") == "true",
            "use_preview_files": filters.get("UsePreview") == "true",
            "use_maximum_render_quality": filters.get("UseMaximumRenderQuality") == "true",
            "custom_start_time_ticks": _num(filters.get("CustomStartTime")),
        },
        "parameters": params,
        "parameter_count": len(params),
    }


def main(out_dir):
    table = C.premiere_strings(SCRATCH)
    sources = []
    failures = []

    roots = [("premiere", C.PREMIERE_ROOT), ("media_encoder", C.AME_ROOT)]
    presets = []
    file_counts = collections.Counter()
    for owner, root in roots:
        if not os.path.isdir(root):
            failures.append({"stage": "root_missing", "root": root})
            continue
        for p in sorted(C.walk_files(root, exts=(".epr",))):
            file_counts[owner] += 1
            try:
                rec = parse_epr(p)
            except Exception as exc:                  # noqa: BLE001
                failures.append({"stage": "epr", "path": C.rel(p, root),
                                 "owner": owner, "error": repr(exc),
                                 "traceback": traceback.format_exc()})
                continue
            rec["owner_install"] = owner
            rec["file"] = C.rel(p, root)
            rec["shipped_folder"] = os.path.basename(os.path.dirname(p))
            rp = C.LAST_PARSE_REPAIRS.get(p)
            if rp:
                rec["parse_repairs"] = rp
            presets.append(rec)

    sources.append({
        "id": "E1_epr",
        "roots": {o: r for o, r in roots},
        "how": ("full PremiereData object-graph parse of every shipped .epr, "
                "including nested ExporterChildParams groups"),
        "epr_files_found": dict(file_counts),
        "epr_files_parsed": len(presets),
        "note": ("epr_files_* are FILE counts because one .epr is exactly one "
                 "export preset; distinct_export_presets below de-duplicates "
                 "the two installs by preset GUID"),
    })
    sources.append({
        "id": "E2_exporter_labels",
        "path": "Adobe Premiere Pro.exe",
        "how": ("$$$/MediaCore/Exporters/<Identifier>Name and .../<Identifier>Desc "
                "read out of the executable's string literals"),
        "strings": sum(1 for k in table
                       if k.startswith("$$$/MediaCore/Exporters/")),
    })

    enum_vocab = collect_enum_vocabularies(table)
    exporter_ns = collect_exporter_namespaces(table)
    sources.append({
        "id": "E3_exporter_enum_vocabularies",
        "how": ("$$$/MediaCore/MediaLayer/ExporterHost/Exporter/<Param>/<Option> "
                "keys grouped per parameter; these are the shipped display names "
                "for enumerated exporter options"),
        "parameters_with_option_labels": len(enum_vocab),
        "exporter_modules_with_own_namespace": len(exporter_ns),
    })

    # ---- parameter dictionary, union across every preset
    pdict = {}
    type_evidence = collections.defaultdict(
        lambda: {"count": 0, "sample_identifiers": [], "value_shapes": collections.Counter()})
    for pr in presets:
        for p in pr["parameters"]:
            ident = p["identifier"]
            if not ident:
                continue
            code = p["type_code"]
            ev = type_evidence[str(code)]
            ev["count"] += 1
            if len(ev["sample_identifiers"]) < 12 and ident not in ev["sample_identifiers"]:
                ev["sample_identifiers"].append(ident)
            v = p.get("value")
            if v is None:
                shape = "no value element"
            elif v in ("true", "false"):
                shape = "boolean literal"
            elif re.fullmatch(r"-?\d+", v or ""):
                shape = "integer literal"
            elif re.fullmatch(r"-?\d*\.\d+", v or ""):
                shape = "decimal literal"
            elif v == "":
                shape = "empty string"
            else:
                shape = "string"
            ev["value_shapes"][shape] += 1

            d = pdict.get(ident)
            if d is None:
                facet, rule = classify(ident)
                lbl, lbl_how = resolve_label(ident, table)
                desc, desc_how = resolve_description(ident, table)
                enums = enum_vocab.get(_strip_ident(ident)) or enum_vocab.get(ident)
                d = pdict[ident] = {
                    "identifier": ident,
                    "label": lbl,
                    "label_lookup": lbl_how,
                    "description": desc,
                    "description_lookup": desc_how,
                    "enum_option_labels": enums,
                    "facet": facet,
                    "facet_rule": rule,
                    "facet_confidence": "heuristic",
                    "type_codes": collections.Counter(),
                    "used_by_preset_count": 0,
                    "used_by_formats": set(),
                    "observed_values": [],
                    "observed_value_count": 0,
                    "declared_min": None,
                    "declared_max": None,
                    "decimal_places": None,
                    "is_slider_in_any_preset": False,
                    "is_hidden_in_any_preset": False,
                    "group_paths": set(),
                }
            d["type_codes"][str(code)] += 1
            d["used_by_preset_count"] += 1
            fc = pr["exporter_file_type_fourcc"]
            if fc:
                d["used_by_formats"].add(fc)
            if p.get("group_path"):
                d["group_paths"].add(" > ".join(p["group_path"]))
            if v is not None:
                d["observed_value_count"] += 1
                if v not in d["observed_values"] and len(d["observed_values"]) < 40:
                    d["observed_values"].append(v)
            if p.get("min") is not None:
                d["declared_min"] = p["min"] if d["declared_min"] is None else min(d["declared_min"], p["min"])
            if p.get("max") is not None:
                d["declared_max"] = p["max"] if d["declared_max"] is None else max(d["declared_max"], p["max"])
            if p.get("decimal_places") is not None:
                d["decimal_places"] = p["decimal_places"]
            if p.get("ui_flags"):
                if "ParamIsSlider" in p["ui_flags"]:
                    d["is_slider_in_any_preset"] = True
                if "ParamIsHidden" in p["ui_flags"]:
                    d["is_hidden_in_any_preset"] = True

    for d in pdict.values():
        d["type_codes"] = dict(d["type_codes"])
        d["primary_type_code"] = max(d["type_codes"], key=d["type_codes"].get)
        d["primary_type_label"] = PARAM_TYPE.get(d["primary_type_code"], "unknown")
        d["used_by_formats"] = sorted(d["used_by_formats"])
        d["group_paths"] = sorted(d["group_paths"])[:12]
        d["observed_values_truncated"] = d["observed_value_count"] > len(d["observed_values"])

    # ---- format matrix
    fmt = {}
    for pr in presets:
        key = "%s|%s" % (pr["exporter_class_id_fourcc"],
                         pr["exporter_file_type_fourcc"])
        f = fmt.get(key)
        if f is None:
            f = fmt[key] = {
                "exporter_class_id_fourcc": pr["exporter_class_id_fourcc"],
                "exporter_file_type_fourcc": pr["exporter_file_type_fourcc"],
                "exporter_class_id_int": pr["exporter_class_id_int"],
                "exporter_file_type_int": pr["exporter_file_type_int"],
                "shipped_folders": set(),
                "owner_installs": set(),
                "preset_count": 0,
                "preset_names": [],
                "does_video": False,
                "does_audio": False,
                "parameter_identifiers": set(),
                "video_codec_values": collections.Counter(),
                "audio_codec_values": collections.Counter(),
                "resolutions": collections.Counter(),
                "frame_rate_values": collections.Counter(),
                "bitrate_encoding_values": collections.Counter(),
                "target_bitrate_values": collections.Counter(),
                "field_type_values": collections.Counter(),
                "colour_space_values": collections.Counter(),
                "audio_sample_rate_values": collections.Counter(),
                "audio_channel_values": collections.Counter(),
                "caption_format_values": collections.Counter(),
            }
        f["shipped_folders"].add(pr["shipped_folder"])
        f["owner_installs"].add(pr["owner_install"])
        f["preset_count"] += 1
        if len(f["preset_names"]) < 400:
            f["preset_names"].append(pr["preset_name"])
        f["does_video"] |= pr["does_video"]
        f["does_audio"] |= pr["does_audio"]
        w = h = None
        for p in pr["parameters"]:
            i = p["identifier"]
            if not i:
                continue
            f["parameter_identifiers"].add(i)
            v = p.get("value")
            if v in (None, ""):
                continue
            if i == "ADBEVideoCodec":
                f["video_codec_values"][v] += 1
            elif i == "ADBEAudioCodec":
                f["audio_codec_values"][v] += 1
            elif i == "ADBEVideoWidth":
                w = v
            elif i == "ADBEVideoHeight":
                h = v
            elif i == "ADBEVideoFPS":
                f["frame_rate_values"][v] += 1
            elif i == "ADBEVideoBitrateEncoding":
                f["bitrate_encoding_values"][v] += 1
            elif i == "ADBEVideoTargetBitrate":
                f["target_bitrate_values"][v] += 1
            elif i == "ADBEVideoFieldType":
                f["field_type_values"][v] += 1
            elif i == "ADBEExportColorSpace":
                f["colour_space_values"][v] += 1
            elif i == "ADBEAudioRatePerSecond":
                f["audio_sample_rate_values"][v] += 1
            elif i == "ADBEAudioNumChannels":
                f["audio_channel_values"][v] += 1
            elif i in ("ADBECaptionFormat", "ADBECaptionExportOption",
                       "ADBECaptionStreamFormat"):
                f["caption_format_values"][v] += 1
        if w and h:
            f["resolutions"]["%sx%s" % (w, h)] += 1

    for f in fmt.values():
        f["shipped_folders"] = sorted(f["shipped_folders"])
        f["owner_installs"] = sorted(f["owner_installs"])
        f["parameter_identifiers"] = sorted(f["parameter_identifiers"])
        f["parameter_count"] = len(f["parameter_identifiers"])
        for k in list(f):
            if isinstance(f[k], collections.Counter):
                f[k] = dict(f[k].most_common(40))

    # frame rate values in .epr are ticks-per-frame; convert
    fps_seen = {}
    for f in fmt.values():
        for tick, n in f["frame_rate_values"].items():
            r = C.frame_rate_from_ticks(tick)
            if r:
                fps_seen[tick] = round(r, 6)

    distinct_ids = {p["preset_id"] for p in presets if p["preset_id"]}
    by_owner = collections.Counter(p["owner_install"] for p in presets)
    facets = collections.Counter(d["facet"] for d in pdict.values())

    payload = C.envelope(
        "handshake.studio.premiere.export_pipeline.v1",
        {
            "summary": ("The full export parameter surface, reconstructed by "
                        "parsing every shipped .epr in the Premiere and Media "
                        "Encoder installs and taking the union of the "
                        "identifiers, types, bounds and values they set."),
            "how_to_read": {
                "exporter_parameter_dictionary": (
                    "one row per distinct ParamIdentifier: its human label and "
                    "description from the executable's string table, its "
                    "declared type, the bounds any preset declares, the value "
                    "set the shipped presets use, and which container formats "
                    "reference it"),
                "format_matrix": (
                    "one row per (ExporterClassID, ExporterFileType) fourcc "
                    "pair: the container. Lists its preset count, whether it "
                    "carries video and/or audio, the codec / resolution / frame "
                    "rate / rate-control / colour-space / audio values its "
                    "presets use, and its full identifier set"),
                "frame_rate_encoding": (
                    "ADBEVideoFPS is stored in Premiere ticks per frame with "
                    "254016000000 ticks per second; frame_rate_tick_decode "
                    "gives the decoded rate for every value seen"),
                "presets": "every shipped preset with its full parameter list",
            },
            "confidence_legend": {
                "parsed": "read verbatim from a shipped .epr or from the executable's string table",
                "heuristic": "derived (ParamType labels, facet classification); marked as such at every use",
            },
            "known_gaps": [
                ("A parameter's enumerated option NAMES are supplied by the "
                 "exporter plug-in at runtime and are not serialized into the "
                 "presets. What is recoverable offline is the integer value set "
                 "the shipped presets actually use, which is given per "
                 "identifier and per format."),
                ("ADBEVideoCodec values are exporter-local integers, not a "
                 "global codec enum; the same integer means different codecs "
                 "under different ExporterClassIDs, so they are reported per "
                 "format and never merged."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "export_presets_parsed": len(presets),
                "distinct_preset_guids": len(distinct_ids),
                "presets_by_install": dict(by_owner),
                "distinct_container_formats": len(fmt),
                "distinct_parameter_identifiers": len(pdict),
                "parameter_rows_total": sum(p["parameter_count"] for p in presets),
                "identifiers_with_a_shipped_label": sum(
                    1 for d in pdict.values() if d["label"]),
                "identifiers_by_facet": dict(facets),
                "count_semantics": ("one .epr file is exactly one export preset, "
                                    "so export_presets_parsed is both a file and "
                                    "an entity count; every other number counts "
                                    "entities"),
            },
            "param_type_enum": {
                "confidence": PARAM_TYPE_CONFIDENCE,
                "values": PARAM_TYPE,
                "note": ("Derived from the value shape and identifier set "
                         "carrying each code; param_type_evidence ships the raw "
                         "tabulation."),
            },
            "param_type_evidence": {
                code: {"count": ev["count"],
                       "value_shapes": dict(ev["value_shapes"]),
                       "sample_identifiers": ev["sample_identifiers"]}
                for code, ev in sorted(type_evidence.items(),
                                       key=lambda kv: int(kv[0]) if kv[0].isdigit() else 999)
            },
            "frame_rate_tick_decode": {
                "ticks_per_second": 254016000000,
                "values": fps_seen,
            },
            "exporter_parameter_dictionary": sorted(
                pdict.values(), key=lambda d: (d["facet"], d["identifier"])),
            "exporter_enum_option_labels": {
                "note": ("Display names for enumerated exporter options, keyed by "
                         "the parameter's short name. The integer a preset stores "
                         "indexes into the exporter's own list; these are the "
                         "shipped labels for those options."),
                "vocabularies": {k: dict(sorted(v.items()))
                                 for k, v in sorted(enum_vocab.items())},
            },
            "exporter_module_string_namespaces": {
                "note": ("Per-exporter string namespaces from the executable. "
                         "These carry the settings-panel labels, tooltips, "
                         "constraint messages and option names for each "
                         "exporter module."),
                "modules": exporter_ns,
            },
            "presets": [
                {k: v for k, v in pr.items() if k != "parameters"} |
                {"parameters": [
                    {kk: vv for kk, vv in p.items()
                     if kk in ("identifier", "value", "type_code", "ordinal",
                               "group_path", "min", "max", "decimal_places",
                               "aux_type", "aux_value", "has_arbitrary_data")
                     and vv not in (None, [], "")}
                    for p in pr["parameters"]]}
                for pr in presets],
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_export_pipeline.json", payload)
    print("wrote", path, size, "bytes")
    print("presets", len(presets), "formats", len(fmt), "identifiers",
          len(pdict), "labelled", sum(1 for d in pdict.values() if d["label"]),
          "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
