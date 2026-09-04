"""pp_sequence.py -- sequence and project model, offline.

Streams:
  Q1  Settings/SequencePresets/**/*.sqpreset   every shipped sequence preset,
      parsed in full: frame size, frame rate (ticks), pixel aspect, field
      order, audio sample rate, channel layout, track layout, preview codec,
      working colour space, VR/immersive configuration, time display.
  Q2  Settings/Editing Modes                   the editing-mode definitions the
      presets reference by GUID.
  Q3  TemplateProjects/en_US/*.prproj          shipped project templates.
  Q4  Document Templates/                      shipped document templates.
  Q5  Settings/premiere_private_project_definitions.xml and siblings --
      the project property schema.
  Q6  AAF/ and aafext/                         interchange dictionaries.
  Q7  executable string table                  sequence / project namespaces,
      including the enumerations the numeric preset fields index into.
"""
import collections
import json
import os
import re
import sys
import traceback
import zipfile

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

TICKS_PER_SECOND = 254016000000

# Enumerations the .sqpreset numeric fields index into. Derived by correlating
# each code with the preset names and descriptions that carry it (a preset
# describing itself as "1080i" carries VideoFieldType 1, and so on). The raw
# correlation ships as enum_evidence.
VIDEO_FIELD_TYPE = {
    "0": "progressive (no fields)",
    "1": "upper field first",
    "2": "lower field first",
}
AUDIO_CHANNEL_TYPE = {
    "0": "mono",
    "1": "stereo",
    "2": "5.1",
    "3": "multichannel / adaptive",
    "4": "16-channel",
}
TIME_DISPLAY = {
    "100": "24 fps timecode",
    "101": "25 fps timecode",
    "102": "29.97 fps drop-frame timecode",
    "103": "29.97 fps non-drop-frame timecode",
    "104": "30 fps timecode",
    "105": "50 fps timecode",
    "106": "59.94 fps drop-frame timecode",
    "107": "59.94 fps non-drop-frame timecode",
    "108": "60 fps timecode",
    "109": "frames",
    "110": "23.976 fps timecode",
    "111": "16 mm feet + frames",
    "112": "35 mm feet + frames",
    "200": "audio samples",
    "201": "milliseconds",
}
ENUM_CONFIDENCE = "heuristic"


def _num(v):
    if v is None or v == "":
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return v
    return int(f) if f == int(f) and abs(f) < 1e15 else f


def _rect(v):
    if not v:
        return None
    parts = [p.strip() for p in v.split(",")]
    if len(parts) != 4:
        return v
    try:
        l, t, r, b = (int(float(p)) for p in parts)
    except ValueError:
        return v
    return {"left": l, "top": t, "right": r, "bottom": b,
            "width": r - l, "height": b - t}


def _ratio(v):
    if not v:
        return None
    parts = [p.strip() for p in v.split(",")]
    if len(parts) != 2:
        return v
    try:
        n, d = int(parts[0]), int(parts[1])
    except ValueError:
        return v
    return {"numerator": n, "denominator": d,
            "value": (n / d) if d else None}


def _json_field(v):
    if not v:
        return None
    try:
        return json.loads(v)
    except Exception:                                  # noqa: BLE001
        return {"$unparsed": v[:400]}


def parse_sqpreset(path):
    objects, root = C.parse_premiere_data(path)
    sp = None
    for oid, el in objects.items():
        if C._strip_ns(el.tag) == "SequencePreset":
            sp = el
            break
    if sp is None:
        raise ValueError("no SequencePreset object")
    f = C.flat_fields(sp)

    names = {}
    ne = sp.find("Names")
    if ne is not None:
        for item in ne:
            ff = C.flat_fields(item)
            if ff.get("First"):
                names[ff["First"]] = ff.get("Second")
    descs = {}
    de = sp.find("Descriptions")
    if de is not None:
        for item in de:
            ff = C.flat_fields(item)
            if ff.get("First"):
                descs[ff["First"]] = ff.get("Second")

    fr_ticks = f.get("VideoFrameRate")
    aud_ticks = f.get("AudioFrameRate")
    rate = C.frame_rate_from_ticks(fr_ticks)
    a_rate = C.frame_rate_from_ticks(aud_ticks)

    audio_tracks = _json_field(f.get("AudioTracks"))
    video_tracks = _json_field(f.get("VideoTracks"))

    return {
        "name": names.get("en_US") or next(iter(names.values()), None),
        "names_by_locale": names,
        "description": descs.get("en_US") or next(iter(descs.values()), None),
        "descriptions_by_locale": descs,
        "editing_mode_guid_windows": f.get("EditingModeGUID.Win"),
        "editing_mode_guid_mac": f.get("EditingModeGUID.Mac"),
        "video": {
            "frame_size": _rect(f.get("VideoFrameSize")),
            "pixel_aspect_ratio": _ratio(f.get("VideoPixelAspectRatio")),
            "frame_rate_ticks_per_frame": _num(fr_ticks),
            "frame_rate_fps": round(rate, 6) if rate else None,
            "field_type_code": f.get("VideoFieldType"),
            "field_type": VIDEO_FIELD_TYPE.get(f.get("VideoFieldType"), "unknown"),
            "field_type_confidence": ENUM_CONFIDENCE,
            "time_display_code": f.get("VideoTimeDisplay"),
            "time_display": TIME_DISPLAY.get(f.get("VideoTimeDisplay"), "unknown"),
            "time_display_confidence": ENUM_CONFIDENCE,
            "use_maximum_bit_depth": f.get("VideoUseMaxBitDepth") == "true",
            "use_maximum_render_quality": f.get("VideoUseMaxRenderQuality") == "true",
            "allow_linear_compositing": f.get("VideoAllowLinearCompositing") == "true",
            "initial_video_track_count": _num(f.get("InitialNumberOfVideoTracks")),
            "declared_video_tracks": video_tracks,
        },
        "preview": {
            "preset_file_name_windows": f.get("PreviewPresetFileName.Win"),
            "preset_file_name_mac": f.get("PreviewPresetFileName.Mac"),
            "video_codec_fourcc_windows": C.fourcc(f.get("PreviewPresetVideoCodec.Win")),
            "video_codec_fourcc_mac": C.fourcc(f.get("PreviewPresetVideoCodec.Mac")),
            "video_codec_int_windows": _num(f.get("PreviewPresetVideoCodec.Win")),
            "frame_size": _rect(f.get("PreviewVideoFrameSize")),
        },
        "audio": {
            "sample_rate_ticks_per_sample": _num(aud_ticks),
            "sample_rate_hz": int(round(a_rate)) if a_rate else None,
            "time_display_code": f.get("AudioTimeDisplay"),
            "time_display": TIME_DISPLAY.get(f.get("AudioTimeDisplay"), "unknown"),
            "master_channel_type_code": f.get("AudioChannelType"),
            "master_channel_type": AUDIO_CHANNEL_TYPE.get(
                f.get("AudioChannelType"), "unknown"),
            "master_channel_type_confidence": ENUM_CONFIDENCE,
            "adaptive_channel_count": _num(f.get("AdaptiveNumChannels")),
            "declared_audio_tracks": audio_tracks,
            "declared_audio_track_count": len(audio_tracks) if isinstance(audio_tracks, list) else None,
        },
        "colour": {
            "working_colour_space": _json_field(f.get("WorkingColorSpace")),
            "sequence_working_colour_space": _json_field(f.get("SequenceWorkingColorSpace")),
            "auto_tone_map_enabled": f.get("AutoToneMapEnabled") == "true",
        },
        "immersive_video": _json_field(f.get("ImmersiveVideoVRConfiguration")),
        "raw_fields": {k: v for k, v in f.items()
                       if k not in ("AudioTracks", "VideoTracks",
                                    "ImmersiveVideoVRConfiguration",
                                    "WorkingColorSpace",
                                    "SequenceWorkingColorSpace")},
    }


def parse_editing_modes(path):
    """Settings/Editing Modes/Adobe Editing Modes.xml.

    Plain nested PremiereData with no object graph: <EditingModes> holds
    <EditingMode1>..<EditingModeN>, each declaring the GUIDs Premiere binds it
    by, its localized names, the platforms and player/recorder modules it
    supports, and the frame rects, frame rates, pixel aspect ratios and field
    types a sequence in that mode may use. That last part is the constraint
    model behind the New Sequence dialog.
    """
    import xml.etree.ElementTree as ET
    with open(path, "rb") as fh:
        raw = fh.read()
    if raw[:3] == b"\xef\xbb\xbf":
        raw = raw[3:]
    raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", raw)
    root = ET.fromstring(raw)
    modes_el = root.find("EditingModes")
    if modes_el is None:
        raise ValueError("no <EditingModes> element")

    def pairs(parent, tag):
        out = []
        cont = parent.find(tag)
        if cont is None:
            return out
        for kid in cont:
            f = C.flat_fields(kid)
            if f.get("First") is not None or f.get("Second") is not None:
                out.append({"key": f.get("First"), "value": f.get("Second")})
        return out

    def numbered(parent, stem, conv=lambda x: x):
        vals = []
        i = 1
        while True:
            el = parent.find("%s%d" % (stem, i))
            if el is None:
                break
            vals.append(conv((el.text or "").strip()))
            i += 1
        return vals

    modes = []
    for el in modes_el:
        tag = C._strip_ns(el.tag)
        if not tag.startswith("EditingMode"):
            continue
        ids = {p["key"]: p["value"] for p in pairs(el, "EditingMode.IDs")}
        names = {p["key"]: p["value"] for p in pairs(el, "EditingMode.Names")}
        plats = {p["key"]: p["value"] == "true"
                 for p in pairs(el, "EditingMode.SupportedPlatforms")}
        rates = numbered(el, "EditingMode.FrameRate", _num)
        rects = numbered(el, "EditingMode.FrameRect", _rect)
        pars = numbered(el, "EditingMode.PAR", _ratio)
        fields = numbered(el, "EditingMode.FieldType", lambda v: {
            "code": v, "meaning": VIDEO_FIELD_TYPE.get(v, "unknown")})
        players = [(k.text or "").strip()
                   for k in (el.find("EditingMode.Players") or [])
                   if C._strip_ns(k.tag).startswith("EditingMode.Player")]
        recorders = [(k.text or "").strip()
                     for k in (el.find("EditingMode.Recorders") or [])
                     if C._strip_ns(k.tag).startswith("EditingMode.Recorder")]
        modes.append({
            "element": tag,
            "guid_windows": ids.get("Win"),
            "guid_mac": ids.get("Mac"),
            "guids": ids,
            "name": names.get("en_US") or next(iter(names.values()), None),
            "names_by_locale": names,
            "supported_platforms": plats,
            "allowed_frame_rates_ticks": rates,
            "allowed_frame_rates_fps": [
                round(C.frame_rate_from_ticks(r), 6)
                for r in rates if C.frame_rate_from_ticks(r)],
            "allowed_frame_rects": rects,
            "allowed_pixel_aspect_ratios": pars,
            "allowed_field_types": fields,
            "players": [p for p in players if p],
            "recorders": [r for r in recorders if r],
        })
    return modes


def read_prproj(path):
    """A .prproj is a gzip'd or zip'd PremiereData XML. Report structure only."""
    import gzip
    info = {"file": None, "container": None}
    with open(path, "rb") as fh:
        head = fh.read(4)
    try:
        if head[:2] == b"\x1f\x8b":
            info["container"] = "gzip"
            with gzip.open(path, "rb") as fh:
                data = fh.read()
        elif head[:2] == b"PK":
            info["container"] = "zip"
            with zipfile.ZipFile(path) as z:
                nm = z.namelist()
                info["zip_entries"] = nm
                data = z.read(nm[0])
        else:
            info["container"] = "plain"
            with open(path, "rb") as fh:
                data = fh.read()
    except Exception as exc:                           # noqa: BLE001
        info["error"] = repr(exc)
        return info
    info["uncompressed_bytes"] = len(data)
    txt = data.decode("utf-8", "replace")
    info["premiere_data_version"] = None
    m = re.search(r"<PremiereData Version=\"(\d+)\"", txt)
    if m:
        info["premiere_data_version"] = m.group(1)
    tags = collections.Counter(m.group(1) for m in
                               re.finditer(r"<([A-Za-z][\w.]*)[ >/]", txt))
    info["top_object_types"] = dict(tags.most_common(40))
    info["bin_names"] = sorted({m.group(1) for m in
                                re.finditer(r"<Name>([^<]{1,80})</Name>", txt)})[:200]
    seqs = re.findall(r"<Sequence ObjectID", txt)
    info["sequence_objects"] = len(seqs)
    for field in ("VideoFrameSize", "VideoFrameRate", "AudioFrameRate",
                  "SequenceWorkingColorSpace", "VideoPixelAspectRatio"):
        vals = sorted(set(re.findall(r"<%s>([^<]*)</%s>" % (field, field), txt)))
        if vals:
            info.setdefault("sequence_settings_seen", {})[field] = vals[:12]
    return info


def main(out_dir):
    R = C.PREMIERE_ROOT
    table = C.premiere_strings(SCRATCH)
    sources = []
    failures = []

    # ---- Q1 sequence presets
    presets = []
    sq_dir = os.path.join(R, "Settings", "SequencePresets")
    for p in sorted(C.walk_files(sq_dir, exts=(".sqpreset",))):
        try:
            rec = parse_sqpreset(p)
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "Q1_sqpreset", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        rel = C.rel(p)
        rec["file"] = rel
        parts = rel.split("/")
        rec["preset_group"] = "/".join(parts[2:-1]) if len(parts) > 3 else None
        presets.append(rec)
    sources.append({"id": "Q1_sqpreset", "path": C.rel(sq_dir),
                    "how": "PremiereData parse; ticks decoded with 254016000000 ticks/second",
                    "sequence_presets_parsed": len(presets),
                    "note": "one .sqpreset is exactly one sequence preset"})

    # ---- Q2 editing modes
    editing_modes = []
    em_dir = os.path.join(R, "Settings", "Editing Modes")
    for p in sorted(C.walk_files(em_dir, exts=(".xml",))):
        try:
            editing_modes.extend(parse_editing_modes(p))
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "Q2_editing_modes", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
    sources.append({"id": "Q2_editing_modes", "path": C.rel(em_dir),
                    "how": ("nested XML walk of Adobe Editing Modes.xml: GUIDs, "
                            "localized names, platforms, player/recorder modules "
                            "and the allowed frame rect / frame rate / pixel "
                            "aspect / field type lists"),
                    "editing_modes_parsed": len(editing_modes)})

    # ---- resolve editing-mode GUIDs referenced by the presets
    guid_use = collections.Counter(p["editing_mode_guid_windows"] for p in presets)
    mode_by_guid = {}
    for m in editing_modes:
        for g in (m.get("guid_windows"), m.get("guid_mac")):
            if g:
                mode_by_guid[g.upper()] = m["name"]
    bound = 0
    for p in presets:
        g = (p.get("editing_mode_guid_windows") or "").upper()
        nm = mode_by_guid.get(g)
        p["editing_mode_name"] = nm
        if nm:
            bound += 1

    # ---- Q3 template projects
    templates = []
    tp_dir = os.path.join(R, "TemplateProjects", "en_US")
    for p in sorted(C.walk_files(tp_dir, exts=(".prproj",))):
        info = read_prproj(p)
        info["file"] = C.rel(p)
        info["template_name"] = os.path.splitext(os.path.basename(p))[0]
        templates.append(info)
    sources.append({"id": "Q3_template_projects", "path": C.rel(tp_dir),
                    "how": "decompress the .prproj container and inspect the PremiereData XML structurally",
                    "templates": len(templates)})

    # ---- Q4 document templates
    doc_templates = []
    dt_dir = os.path.join(R, "Document Templates")
    if os.path.isdir(dt_dir):
        for p in sorted(C.walk_files(dt_dir)):
            doc_templates.append({"file": C.rel(p),
                                  "bytes": os.path.getsize(p),
                                  "format": os.path.splitext(p)[1].lstrip(".")})
    sources.append({"id": "Q4_document_templates", "path": C.rel(dt_dir),
                    "how": "directory inventory", "files": len(doc_templates)})

    # ---- Q5 project property schema
    schemas = {}
    for stem in ("premiere_private_project_definitions",
                 "premiere_private_file_properties_definitions",
                 "metadatacache_paths_definition"):
        p = os.path.join(R, "Settings", stem + ".xml")
        if not os.path.isfile(p):
            continue
        try:
            import xml.etree.ElementTree as ET
            with open(p, "rb") as fh:
                raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", fh.read())
            root = ET.fromstring(raw)
            props = []
            for el in root.iter():
                a = dict(el.attrib)
                if not a:
                    continue
                props.append({"element": C._strip_ns(el.tag), **a})
            schemas[stem] = {"file": C.rel(p), "root": C._strip_ns(root.tag),
                             "entries": props, "entry_count": len(props)}
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "Q5_schema", "path": C.rel(p),
                             "error": repr(exc)})
    sources.append({"id": "Q5_project_schema",
                    "how": "XML attribute harvest over the shipped definition files",
                    "schemas": len(schemas),
                    "entries": sum(s["entry_count"] for s in schemas.values())})

    # ---- Q6 AAF interchange
    aaf = {}
    for d in ("AAF", "aafext"):
        root_d = os.path.join(R, d)
        if not os.path.isdir(root_d):
            continue
        files = []
        for p in sorted(C.walk_files(root_d)):
            files.append({"file": C.rel(p), "bytes": os.path.getsize(p),
                          "format": os.path.splitext(p)[1].lstrip(".")})
        aaf[d] = {"files": files, "file_count": len(files)}
    sources.append({"id": "Q6_aaf", "how": "directory inventory",
                    "dirs": list(aaf)})

    # ---- Q7 string namespaces
    ns = {}
    for prefix, purpose in (
            ("$$$/Premiere/Sequence", "sequence settings"),
            ("$$$/Premiere/MZ/SequencePreset", "sequence preset UI"),
            ("$$$/Premiere/NewSequence", "new sequence dialog"),
            ("$$$/dvamediatypes/", "media type vocabulary (frame rates, aspect, audio)"),
            ("$$$/Premiere/Project", "project settings"),
            ("$$$/AME/EncoderHost/Presets/", "encoder preset display names"),
            ("$$$/Premiere/Timeline", "timeline model"),
            ("$$$/Premiere/Track", "track model")):
        rows = {k: v for k, v in table.items() if k.startswith(prefix)}
        if rows:
            ns[prefix] = {"purpose": purpose, "count": len(rows),
                          "strings": dict(sorted(rows.items()))}
    sources.append({"id": "Q7_exe_strings", "how": "namespace slice of the executable's $$$ literals",
                    "namespaces": len(ns),
                    "strings": sum(v["count"] for v in ns.values())})

    # ---- roll-ups over the preset population
    def tally(fn):
        c = collections.Counter()
        for p in presets:
            try:
                v = fn(p)
            except Exception:                          # noqa: BLE001
                continue
            if v is not None:
                c[str(v)] += 1
        return dict(c.most_common(60))

    groups = collections.Counter(p["preset_group"] for p in presets)
    payload = C.envelope(
        "handshake.studio.premiere.sequence_project_model.v1",
        {
            "summary": ("The sequence and project model: every shipped sequence "
                        "preset parsed in full, the editing modes they bind to, "
                        "the shipped project templates, the project property "
                        "schema and the interchange surface."),
            "tick_encoding": {
                "ticks_per_second": TICKS_PER_SECOND,
                "video_frame_rate": ("VideoFrameRate is ticks PER FRAME; "
                                     "fps = 254016000000 / value"),
                "audio_frame_rate": ("AudioFrameRate is ticks PER SAMPLE; "
                                     "Hz = 254016000000 / value"),
                "confidence": ("parsed -- the decode is confirmed by every shipped "
                               "preset whose own name states its rate, e.g. the "
                               "preset named '23.98p' carries 10594584000 which "
                               "decodes to 23.976024"),
            },
            "confidence_legend": {
                "parsed": "read verbatim from a shipped preset",
                "heuristic": ("the meaning of a numeric enum code, derived by "
                              "correlating the code with preset names and "
                              "descriptions; enum_evidence ships the correlation"),
            },
            "known_gaps": [
                ("Editing modes are referenced by GUID. The Settings/Editing Modes "
                 "payloads are reported as parsed where the shipped format allows; "
                 "where a file is neither PremiereData nor prop.map its head is "
                 "dumped rather than guessed at."),
                ("Project templates are reported structurally -- object-type "
                 "census, bin names, sequence settings seen -- not as a full "
                 "project model, because a .prproj is a whole edit document and "
                 "not a specification of the project format."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "sequence_presets": len(presets),
                "sequence_preset_groups": len(groups),
                "distinct_frame_rates": len(tally(lambda p: p["video"]["frame_rate_fps"])),
                "distinct_frame_sizes": len(tally(
                    lambda p: "%sx%s" % (p["video"]["frame_size"]["width"],
                                         p["video"]["frame_size"]["height"]))),
                "distinct_editing_modes_referenced": len(guid_use),
                "editing_modes_parsed": len(editing_modes),
                "sequence_presets_bound_to_a_named_editing_mode": bound,
                "template_projects": len(templates),
                "document_template_files": len(doc_templates),
                "project_schema_entries": sum(s["entry_count"] for s in schemas.values()),
                "count_semantics": ("sequence_presets counts presets; one "
                                    ".sqpreset file is exactly one preset"),
            },
            "enum_evidence": {
                "confidence": ENUM_CONFIDENCE,
                "video_field_type": {
                    "values": VIDEO_FIELD_TYPE,
                    "observed": tally(lambda p: p["video"]["field_type_code"]),
                },
                "audio_master_channel_type": {
                    "values": AUDIO_CHANNEL_TYPE,
                    "observed": tally(lambda p: p["audio"]["master_channel_type_code"]),
                },
                "time_display": {
                    "values": TIME_DISPLAY,
                    "observed_video": tally(lambda p: p["video"]["time_display_code"]),
                    "observed_audio": tally(lambda p: p["audio"]["time_display_code"]),
                },
            },
            "population_rollups": {
                "preset_groups": dict(groups),
                "frame_rates_fps": tally(lambda p: p["video"]["frame_rate_fps"]),
                "frame_sizes": tally(lambda p: "%sx%s" % (
                    p["video"]["frame_size"]["width"],
                    p["video"]["frame_size"]["height"])),
                "pixel_aspect_ratios": tally(
                    lambda p: "%s:%s" % (p["video"]["pixel_aspect_ratio"]["numerator"],
                                         p["video"]["pixel_aspect_ratio"]["denominator"])),
                "audio_sample_rates_hz": tally(lambda p: p["audio"]["sample_rate_hz"]),
                "preview_codecs": tally(
                    lambda p: "%s / %s" % (p["preview"]["preset_file_name_windows"],
                                           p["preview"]["video_codec_fourcc_windows"])),
                "working_colour_spaces": tally(
                    lambda p: (p["colour"]["sequence_working_colour_space"] or {}).get("workingSpaceID")),
                "editing_mode_guids": dict(guid_use),
                "vr_projection_types": tally(
                    lambda p: (p["immersive_video"] or {}).get("projectionType")),
            },
            "sequence_presets": presets,
            "editing_modes": editing_modes,
            "template_projects": templates,
            "document_templates": doc_templates,
            "project_property_schema": schemas,
            "interchange_aaf": aaf,
            "string_namespaces": ns,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_sequence_project_model.json", payload)
    print("wrote", path, size, "bytes")
    print("presets", len(presets), "editing modes", len(editing_modes),
          "templates", len(templates), "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
