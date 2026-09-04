"""pp_media_io.py -- the importable and exportable format matrix, offline.

Premiere does not ship a manifest of what it can open. The import surface is
therefore reconstructed from four independent kinds of on-disk evidence, and
every row states which of them it rests on:

  M1  importer string namespaces
      $$$/MediaCore/Importers/<Module>/... in the executable. One namespace per
      importer module, carrying that importer's display name, its format
      description strings, its source-settings parameter labels and its error
      messages. An importer module having a namespace is direct evidence that
      the importer is present in this build.

  M2  codec and container libraries shipped alongside the executable
      The MainConcept mc_dec_* / mc_enc_* / mc_demux_* / mc_mux_* modules,
      libav*, the RED, ProRes, ARRI, Sony, Canon, CineForm, DNxHD and Kakadu
      SDK modules. A shipped decoder module is direct evidence of decode
      support; the version resource of each is read from the PE file.

  M3  source-settings parameter surfaces
      Camera-raw formats expose a source-settings effect. Those are declared as
      "AE.ADBE <Format>.SourceSettings" match names in the executable and carry
      their own parameter namespaces.

  M4  the export side, already fully parsed
      MediaIO/systempresets folder names encode
      <ExporterClassID fourcc>_<ExporterFileType fourcc>. Cross-referenced here
      so the matrix shows import and export together.

  M5  install/adm, arriimagesdk_plugins, FirstMile inventory.
"""
import collections
import os
import re
import struct
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

# Shipped SDK / codec modules and what each one is. The module names are read
# from the directory; the mapping to a codec family is a naming read and is
# labelled derived, except where the module's own version resource names it.
CODEC_MODULES = {
    "mc_dec_aac": ("MainConcept", "decode", "AAC audio"),
    "mc_dec_avc": ("MainConcept", "decode", "H.264 / AVC video"),
    "mc_dec_dv100": ("MainConcept", "decode", "DVCPRO HD (DV100) video"),
    "mc_dec_mp2v": ("MainConcept", "decode", "MPEG-2 video"),
    "mc_dec_mp4v": ("MainConcept", "decode", "MPEG-4 Part 2 video"),
    "mc_dec_mpa": ("MainConcept", "decode", "MPEG audio (MP1/MP2)"),
    "mc_enc_aac": ("MainConcept", "encode", "AAC audio"),
    "mc_enc_avc": ("MainConcept", "encode", "H.264 / AVC video"),
    "mc_enc_avcsr": ("MainConcept", "encode", "H.264 smart-render"),
    "mc_enc_dv100": ("MainConcept", "encode", "DVCPRO HD (DV100) video"),
    "mc_enc_mp2sr": ("MainConcept", "encode", "MPEG-2 smart-render"),
    "mc_enc_mp2v": ("MainConcept", "encode", "MPEG-2 video"),
    "mc_enc_mp4v": ("MainConcept", "encode", "MPEG-4 Part 2 video"),
    "mc_enc_mpa": ("MainConcept", "encode", "MPEG audio"),
    "mc_demux_mp2": ("MainConcept", "demux", "MPEG-2 program/transport stream"),
    "mc_demux_mp4": ("MainConcept", "demux", "MP4 / QuickTime container"),
    "mc_demux_mxf": ("MainConcept", "demux", "MXF container"),
    "mc_mux_mp2": ("MainConcept", "mux", "MPEG-2 program/transport stream"),
    "mc_mux_mp4": ("MainConcept", "mux", "MP4 container"),
    "mc_mfimport": ("MainConcept", "import", "Media Foundation bridge"),
    "mc_trans_video_colorspace": ("MainConcept", "transform", "colour-space conversion"),
    "libavcodec": ("FFmpeg", "decode/encode", "wide codec set"),
    "libavformat": ("FFmpeg", "demux/mux", "wide container set"),
    "libavutil": ("FFmpeg", "support", "shared utilities"),
    "libmp3lame": ("LAME", "encode", "MP3 audio"),
    "libmpg123": ("mpg123", "decode", "MPEG audio"),
    "REDDecoder-x64": ("RED", "decode", "REDCODE RAW (R3D)"),
    "REDR3D-x64": ("RED", "decode", "REDCODE RAW (R3D) core"),
    "REDCuda-x64": ("RED", "accelerate", "REDCODE RAW, CUDA"),
    "REDOpenCL-x64": ("RED", "accelerate", "REDCODE RAW, OpenCL"),
    "ProResOpt": ("Apple", "decode/encode", "Apple ProRes"),
    "ProResRAW": ("Apple", "decode", "Apple ProRes RAW"),
    "DNxSDK-vs2019": ("Avid", "decode/encode", "Avid DNxHD / DNxHR"),
    "ArriImageSdk.8": ("ARRI", "decode", "ARRIRAW"),
    "SonyRawDev": ("Sony", "decode", "Sony RAW / X-OCN"),
    "crxdec": ("Canon", "decode", "Canon Cinema RAW Light (CRM/CR3)"),
    "codexhdedecoder": ("Codex", "decode", "Codex HDE"),
    "CFHDDecoder64": ("GoPro", "decode", "CineForm"),
    "CFHDEncoder64": ("GoPro", "encode", "CineForm"),
    "AVCIntraEncoder": ("Adobe", "encode", "AVC-Intra"),
    "kdu_as85R": ("Kakadu", "decode/encode", "JPEG 2000"),
    "kdu_vs85R": ("Kakadu", "decode/encode", "JPEG 2000"),
    "JP2KLib": ("Adobe", "decode/encode", "JPEG 2000"),
    "jpeg_wrapper": ("Adobe", "decode/encode", "JPEG"),
    "MOG_Framework_1.1.12": ("MOG", "decode/encode", "MXF / broadcast"),
    "MSDK_Pro_1.1.12": ("MOG", "decode/encode", "MXF / broadcast"),
    "SMDK-VC140-x64-4_26_0": ("Sony", "support", "Sony media SDK"),
    "Pro4OMFdll64": ("Avid", "interchange", "OMF"),
    "DQomfToolkit64": ("Avid", "interchange", "OMF toolkit"),
    "AAFCOAPI": ("AMWA", "interchange", "AAF"),
    "LibLTCWrapper": ("libltc", "decode", "linear timecode"),
    "AdobePDFL": ("Adobe", "decode", "PDF"),
    "SVGRE": ("Adobe", "decode", "SVG"),
    "AdobeSVGAGM": ("Adobe", "decode", "SVG rendering"),
    "SVGExport": ("Adobe", "encode", "SVG"),
    "adobe_c2pa": ("Adobe", "metadata", "C2PA content credentials"),
    "AdobeXMP": ("Adobe", "metadata", "XMP"),
    "AdobeXMPFiles": ("Adobe", "metadata", "XMP file handlers"),
}


def pe_version(path):
    """Read the VS_FIXEDFILEINFO product/file version out of a PE resource.

    Plain byte scan for the VS_FIXEDFILEINFO signature; the module is never
    loaded.
    """
    try:
        with open(path, "rb") as fh:
            data = fh.read(6 * 1024 * 1024)
    except OSError as exc:
        return {"error": repr(exc)}
    i = data.find(b"\xbd\x04\xef\xfe")          # VS_FFI_SIGNATURE
    if i < 0:
        return None
    try:
        (_sig, _sv, fv_ms, fv_ls, pv_ms, pv_ls) = struct.unpack_from(
            "<6I", data, i)
    except struct.error:
        return None

    def ver(ms, ls):
        return "%d.%d.%d.%d" % (ms >> 16, ms & 0xFFFF, ls >> 16, ls & 0xFFFF)

    out = {"file_version": ver(fv_ms, fv_ls),
           "product_version": ver(pv_ms, pv_ls)}
    for field in (b"F\x00i\x00l\x00e\x00D\x00e\x00s\x00c\x00r\x00i\x00p\x00t\x00i\x00o\x00n\x00",
                  b"P\x00r\x00o\x00d\x00u\x00c\x00t\x00N\x00a\x00m\x00e\x00"):
        j = data.find(field)
        if j < 0:
            continue
        k = j + len(field)
        while k < len(data) - 1 and data[k:k + 2] == b"\x00\x00":
            k += 2
        end = data.find(b"\x00\x00\x00", k)
        if end < 0:
            continue
        try:
            txt = data[k:end + 1].decode("utf-16-le", "ignore").strip("\x00").strip()
        except Exception:                              # noqa: BLE001
            continue
        if txt:
            key = ("file_description" if b"File" in field else "product_name")
            out[key] = txt
    return out


def main(out_dir):
    R = C.PREMIERE_ROOT
    table = C.premiere_strings(SCRATCH)
    sources = []
    failures = []

    # ---- M1 importer namespaces
    imp = collections.defaultdict(dict)
    for k, v in table.items():
        if not k.startswith("$$$/MediaCore/Importers/"):
            continue
        tail = k[len("$$$/MediaCore/Importers/"):]
        if "/" in tail:
            mod, rest = tail.split("/", 1)
        else:
            mod, rest = "(shared)", tail
        imp[mod][rest] = v
    importers = []
    for mod, kv in sorted(imp.items()):
        if C.looks_ai(mod):
            continue
        name = (kv.get("Name") or kv.get("DisplayName")
                or kv.get("FormatName") or None)
        importers.append({
            "importer_module": mod,
            "display_name": name,
            "string_count": len(kv),
            "strings": dict(sorted(kv.items())),
            "evidence": "M1_importer_string_namespace",
        })
    sources.append({
        "id": "M1_importer_namespaces",
        "how": ("$$$/MediaCore/Importers/<Module>/... grouped per importer "
                "module out of the executable's string literals"),
        "importer_modules": len(importers),
        "strings": sum(i["string_count"] for i in importers),
    })

    # ---- M2 shipped codec / container modules
    modules = []
    for p in sorted(C.walk_files(R, exts=(".dll",))):
        stem = os.path.splitext(os.path.basename(p))[0]
        info = CODEC_MODULES.get(stem)
        if info is None:
            continue
        vendor, role, fmt = info
        rec = {
            "module": os.path.basename(p),
            "path": C.rel(p),
            "bytes": os.path.getsize(p),
            "vendor": vendor,
            "role": role,
            "format_family": fmt,
            "attribution_confidence": "derived from the module name",
            "evidence": "M2_shipped_module",
        }
        v = pe_version(p)
        if v:
            rec["version_resource"] = v
            if v.get("file_description"):
                rec["attribution_confidence"] = (
                    "confirmed by the module's own version resource")
        modules.append(rec)
    # also record the codec-ish modules that ship but are not in the table
    unmapped = []
    for p in sorted(C.walk_files(R, exts=(".dll",))):
        stem = os.path.splitext(os.path.basename(p))[0]
        if stem in CODEC_MODULES:
            continue
        if re.match(r"^(mc_|lib(av|mp)|RED|kdu_|Arri|Sony|Canon|codex|CFHD|DNx|ProRes)",
                    stem, re.I):
            unmapped.append(C.rel(p))
    sources.append({
        "id": "M2_shipped_modules",
        "how": ("directory scan for the known codec / container / camera SDK "
                "modules, with each module's VS_FIXEDFILEINFO version resource "
                "read by byte scan; no module is loaded"),
        "modules_identified": len(modules),
        "codec_like_modules_not_in_the_lookup": unmapped,
    })

    # ---- M3 source settings surfaces
    exe = os.path.join(R, "Adobe Premiere Pro.exe")
    src_settings = []
    try:
        with open(exe, "rb") as fh:
            blob = fh.read()
        for m in re.finditer(
                rb"(?<![\x21-\x7e])((?:AE\.)?ADBE [\x20-\x7e]{2,50}\.SourceSettings)\x00",
                blob):
            src_settings.append(m.group(1).decode("latin-1"))
        del blob
    except Exception as exc:                           # noqa: BLE001
        failures.append({"stage": "M3_source_settings", "error": repr(exc)})
    src_settings = sorted(set(src_settings))
    ss_rows = []
    for mn in src_settings:
        stem = mn.split()[-1].replace(".SourceSettings", "")
        ns = {k: v for k, v in table.items()
              if stem.lower().replace(" ", "") in k.lower().replace(" ", "")
              and k.startswith("$$$/MediaCore/")}
        ss_rows.append({
            "match_name": mn,
            "format": stem,
            "parameter_strings_found": len(ns),
            "parameter_strings": dict(sorted(ns.items())) if len(ns) <= 220 else None,
            "evidence": "M3_source_settings_match_name",
        })
    sources.append({
        "id": "M3_source_settings",
        "how": ("'*.SourceSettings' match-name literals in the executable, each "
                "cross-referenced against the MediaCore string namespaces"),
        "source_settings_effects": len(ss_rows),
    })

    # ---- M4 export side
    export_formats = []
    for root_label, root in (("premiere", R), ("media_encoder", C.AME_ROOT)):
        d = os.path.join(root, "MediaIO", "systempresets")
        if not os.path.isdir(d):
            continue
        for entry in sorted(os.listdir(d)):
            full = os.path.join(d, entry)
            if not os.path.isdir(full):
                continue
            if "_" in entry:
                cls, ft = entry.split("_", 1)
            else:
                cls, ft = entry, ""
            export_formats.append({
                "folder": entry,
                "install": root_label,
                "exporter_class_id_fourcc": C.fourcc_from_hex(cls),
                "exporter_file_type_fourcc": C.fourcc_from_hex(ft),
                "shipped_preset_count": len(
                    [f for f in os.listdir(full) if f.lower().endswith(".epr")]),
                "evidence": "M4_shipped_export_presets",
            })
    merged = {}
    for f in export_formats:
        k = f["folder"]
        if k in merged:
            merged[k]["installs"].append(f["install"])
            merged[k]["shipped_preset_count"] = max(
                merged[k]["shipped_preset_count"], f["shipped_preset_count"])
        else:
            merged[k] = {**f, "installs": [f["install"]]}
            merged[k].pop("install")
    export_formats = sorted(merged.values(),
                            key=lambda x: -x["shipped_preset_count"])
    sources.append({
        "id": "M4_export_formats",
        "how": ("MediaIO/systempresets folder names decoded as "
                "<ExporterClassID fourcc>_<ExporterFileType fourcc>; the full "
                "parameter surface for each is in premiere_export_pipeline.json"),
        "export_container_formats": len(export_formats),
    })

    # ---- M5 vendor plug-in inventory
    inventory = {}
    for d in ("arriimagesdk_plugins", "FirstMile", "adm", "AAF", "aafext"):
        p = os.path.join(R, d)
        if not os.path.isdir(p):
            continue
        files = [{"file": C.rel(x), "bytes": os.path.getsize(x)}
                 for x in sorted(C.walk_files(p))]
        inventory[d] = {"file_count": len(files), "files": files[:200]}
    sources.append({"id": "M5_inventory", "how": "directory inventory",
                    "dirs": list(inventory)})

    # ---- media type vocabulary
    media_types = {k: v for k, v in table.items()
                   if k.startswith("$$$/dvamediatypes/")}

    by_vendor = collections.Counter(m["vendor"] for m in modules)
    by_role = collections.Counter(m["role"] for m in modules)

    payload = C.envelope(
        "handshake.studio.premiere.media_io.v1",
        {
            "summary": ("The import and export format surface. Premiere ships no "
                        "manifest of what it can open, so the import side is "
                        "reconstructed from the importer string namespaces in "
                        "the executable and from the codec and container modules "
                        "that ship beside it. The export side is the shipped "
                        "preset folders, whose full parameter surface lives in "
                        "premiere_export_pipeline.json."),
            "evidence_kinds": {
                "M1_importer_string_namespace": ("the executable carries a "
                                                 "string namespace for this "
                                                 "importer module"),
                "M2_shipped_module": ("a decoder / encoder / demuxer module for "
                                      "this format ships in the install"),
                "M3_source_settings_match_name": ("the format has a source-"
                                                  "settings effect, i.e. it is a "
                                                  "camera raw format with its own "
                                                  "decode parameters"),
                "M4_shipped_export_presets": ("the install ships export presets "
                                              "for this container"),
            },
            "confidence_legend": {
                "parsed": "read verbatim from a shipped file or version resource",
                "derived from the module name": ("the codec family a shipped "
                                                 "module serves, read off its "
                                                 "file name"),
                "confirmed by the module's own version resource": (
                    "the module's FileDescription states what it is"),
            },
            "known_gaps": [
                ("Import support is NOT a shipped list. A format's absence from "
                 "importers[] means no string namespace names it, not that "
                 "Premiere cannot open it: libavformat alone covers containers "
                 "no namespace mentions. Every row states its evidence so the "
                 "difference is visible."),
                ("Codec module attribution is a naming read unless the module's "
                 "own version resource confirms it; that is recorded per row."),
                ("The ONNX / OpenVINO / DirectML inference runtime modules that "
                 "ship in the install are an excluded AI surface and are not "
                 "listed as media modules."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "importer_modules_with_a_string_namespace": len(importers),
                "importer_namespace_strings": sum(i["string_count"] for i in importers),
                "codec_and_container_modules_identified": len(modules),
                "modules_by_vendor": dict(by_vendor),
                "modules_by_role": dict(by_role),
                "camera_raw_source_settings_effects": len(ss_rows),
                "export_container_formats": len(export_formats),
                "export_presets_across_all_containers": sum(
                    f["shipped_preset_count"] for f in export_formats),
                "media_type_vocabulary_strings": len(media_types),
                "count_semantics": ("module counts are file counts because one "
                                    "module is one shipped file; importer and "
                                    "format counts are entity counts"),
            },
            "importers": importers,
            "codec_and_container_modules": modules,
            "camera_raw_source_settings": ss_rows,
            "export_container_formats": export_formats,
            "media_type_vocabulary": dict(sorted(media_types.items())),
            "vendor_plugin_inventory": inventory,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_media_io.json", payload)
    print("wrote", path, size, "bytes")
    print("importers", len(importers), "modules", len(modules),
          "source settings", len(ss_rows), "export formats",
          len(export_formats), "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
