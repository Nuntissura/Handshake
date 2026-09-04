#!/usr/bin/env python
"""
lightroom-profiles.py

Characterises the two profile binaries Lightroom Classic ships:

  .dcp  DNG Camera Profile. A stripped TIFF/DNG IFD ("IIRC" magic, little
        endian, magic 0x4352). Parsed here as a real IFD: every tag is read,
        DNG profile tags are named, matrix/LUT tags are reported by shape
        rather than copied. This recovers the colour model each profile
        carries: forward/colour/calibration matrices, illuminants, hue-sat
        map dimensions, look table dimensions, tone curve, encodings,
        baseline exposure offset and gain table maps.

  .lcp  Adobe Lens Correction Profile. XMP/RDF text in the stCamera
        namespace. One file holds many calibration samples (rdf:li), each
        with a focal length / focus distance / aperture and up to four
        correction models: PerspectiveModel (radial distortion),
        FisheyeModel, VignetteModel, ChromaticAberrationModel.

NO PROFILE BINARY IS COPIED. Only structure, identity and model shape are
recorded. Read-only; Lightroom is never launched.
"""
from __future__ import annotations

import argparse
import collections
import datetime as _dt
import json
import os
import re
import struct
import sys

SCHEMA_ID = "handshake.adobe.lightroom_classic.profiles.v1"

# --- DNG / DCP tag names (DNG 1.7 spec numbering) --------------------------
DNG_TAGS = {
    254: "NewSubfileType", 256: "ImageWidth", 257: "ImageLength",
    50708: "UniqueCameraModel", 50709: "LocalizedCameraModel",
    50721: "ColorMatrix1", 50722: "ColorMatrix2",
    50723: "CameraCalibration1", 50724: "CameraCalibration2",
    50725: "ReductionMatrix1", 50726: "ReductionMatrix2",
    50727: "AnalogBalance", 50728: "AsShotNeutral", 50729: "AsShotWhiteXY",
    50730: "BaselineExposure", 50731: "BaselineNoise",
    50732: "BaselineSharpness", 50734: "LinearResponseLimit",
    50778: "CalibrationIlluminant1", 50779: "CalibrationIlluminant2",
    50781: "RawDataUniqueID", 50879: "ColorimetricReference",
    50931: "CameraCalibrationSignature",
    50932: "ProfileCalibrationSignature",
    50933: "ExtraCameraProfiles", 50934: "AsShotProfileName",
    50936: "ProfileName", 50937: "ProfileHueSatMapDims",
    50938: "ProfileHueSatMapData1", 50939: "ProfileHueSatMapData2",
    50940: "ProfileToneCurve", 50941: "ProfileEmbedPolicy",
    50942: "ProfileCopyright", 50964: "ForwardMatrix1",
    50965: "ForwardMatrix2", 50966: "PreviewApplicationName",
    50970: "PreviewColorSpace",
    50981: "ProfileLookTableDims", 50982: "ProfileLookTableData",
    51041: "NoiseProfile",
    51107: "ProfileHueSatMapEncoding", 51108: "ProfileLookTableEncoding",
    51109: "BaselineExposureOffset", 51110: "DefaultBlackRender",
    51111: "NewRawImageDigest", 51125: "DefaultUserCrop",
    52525: "ProfileGainTableMap", 52543: "ProfileGainTableMap2",
    52544: "ProfileDynamicRange",
}
ILLUMINANTS = {
    0: "unknown", 1: "daylight", 2: "fluorescent", 3: "tungsten",
    4: "flash", 9: "fine weather", 10: "cloudy weather", 11: "shade",
    12: "daylight fluorescent", 13: "day white fluorescent",
    14: "cool white fluorescent", 15: "white fluorescent",
    17: "standard light A", 18: "standard light B", 19: "standard light C",
    20: "D55", 21: "D65", 22: "D75", 23: "D50",
    24: "ISO studio tungsten", 255: "other",
}
TYPE_SIZE = {1: 1, 2: 1, 3: 2, 4: 4, 5: 8, 6: 1, 7: 1, 8: 2, 9: 4, 10: 8,
             11: 4, 12: 8}
TYPE_NAME = {1: "BYTE", 2: "ASCII", 3: "SHORT", 4: "LONG", 5: "RATIONAL",
             6: "SBYTE", 7: "UNDEFINED", 8: "SSHORT", 9: "SLONG",
             10: "SRATIONAL", 11: "FLOAT", 12: "DOUBLE"}
SCALAR_TAGS = {50708, 50709, 50936, 50942, 50931, 50932, 50934, 50778,
               50779, 50941, 50730, 51109, 51110, 50937, 50981, 51107,
               51108, 50970, 50879, 52544}


def read_dcp(path):
    with open(path, "rb") as fh:
        blob = fh.read()
    if blob[:2] != b"II":
        return {"error": "not little-endian TIFF-like", "magic": blob[:4].hex()}
    magic = struct.unpack_from("<H", blob, 2)[0]
    ifd_off = struct.unpack_from("<I", blob, 4)[0]
    n = struct.unpack_from("<H", blob, ifd_off)[0]
    out = {"magic": blob[:4].decode("latin-1"), "magic_word": magic,
           "ifd_entries": n, "tags": {}, "unknown_tags": []}
    for i in range(n):
        base = ifd_off + 2 + i * 12
        tag, typ, cnt, val = struct.unpack_from("<HHII", blob, base)
        size = TYPE_SIZE.get(typ, 1) * cnt
        name = DNG_TAGS.get(tag)
        rec = {"tag": tag, "type": TYPE_NAME.get(typ, typ), "count": cnt,
               "bytes": size}
        if name is None:
            out["unknown_tags"].append(rec)
            continue
        if tag in SCALAR_TAGS:
            if typ == 2:  # ASCII
                if size <= 4:
                    raw = struct.pack("<I", val)[:size]
                else:
                    raw = blob[val:val + size]
                rec["value"] = raw.rstrip(b"\x00").decode("utf-8", "replace")
            elif typ in (3, 4) and cnt <= 4:
                if size <= 4:
                    fmt = "<%d%s" % (cnt, "H" if typ == 3 else "I")
                    rec["value"] = list(struct.unpack_from(
                        fmt, struct.pack("<I", val)))
                else:
                    fmt = "<%d%s" % (cnt, "H" if typ == 3 else "I")
                    rec["value"] = list(struct.unpack_from(fmt, blob, val))
                if tag in (50778, 50779) and rec["value"]:
                    rec["illuminant"] = ILLUMINANTS.get(rec["value"][0],
                                                        "reserved")
            elif typ == 10 and cnt == 1:  # SRATIONAL
                num, den = struct.unpack_from("<ii", blob, val)
                rec["value"] = num / den if den else None
            elif typ == 5 and cnt == 1:
                num, den = struct.unpack_from("<II", blob, val)
                rec["value"] = num / den if den else None
            elif typ == 11 and cnt == 1:
                rec["value"] = struct.unpack_from(
                    "<f", struct.pack("<I", val))[0]
        else:
            rec["value"] = "<%d bytes of payload, not copied>" % size
        out["tags"][name] = rec
    return out


# --- LCP -------------------------------------------------------------------
LI_RE = re.compile(r"<rdf:li[ >]")
MODEL_RE = re.compile(r"<stCamera:(\w+Model)\b")
HEADATTR_RE = re.compile(r'stCamera:(\w+)="([^"]*)"')
ATTR_RE = re.compile(r'stCamera:(\w+)="([^"]*)"')
NUM_RE = re.compile(r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$")


def read_lcp(path, deep, vocab, ranges):
    with open(path, "rb") as fh:
        raw = fh.read()
    txt = raw.decode("utf-8", "replace")
    head = txt[:6000]
    hattrs = dict(HEADATTR_RE.findall(head))
    models = collections.Counter(m for m in MODEL_RE.findall(txt)
                                 if m != "UniqueCameraModel")
    rec = {
        "samples": len(LI_RE.findall(txt)),
        "models": dict(models),
        "make": hattrs.get("Make", ""),
        "camera_model": hattrs.get("Model", ""),
        "camera_pretty": hattrs.get("CameraPrettyName", ""),
        "lens": hattrs.get("Lens", ""),
        "lens_pretty": hattrs.get("LensPrettyName", ""),
        "lens_id": hattrs.get("LensID", ""),
        "lens_info": hattrs.get("LensInfo", ""),
        "profile_name": hattrs.get("ProfileName", ""),
        "author": hattrs.get("Author", ""),
        "sensor_format_factor": hattrs.get("SensorFormatFactor", ""),
        "raw_profile": hattrs.get("CameraRawProfile", ""),
    }
    if deep:
        for k, v in ATTR_RE.findall(txt):
            vocab[k] += 1
            if NUM_RE.match(v):
                f = float(v)
                lo, hi = ranges.get(k, (f, f))
                ranges[k] = (min(lo, f), max(hi, f))
    return rec


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install",
                    default=r"C:\Program Files\Adobe\Adobe Lightroom Classic")
    ap.add_argument("--out", required=True)
    ap.add_argument("--lcp-deep", type=int, default=400,
                    help="how many .lcp files get a full attribute harvest")
    args = ap.parse_args()

    cam_root = os.path.join(args.install, "Resources", "CameraProfiles")
    lens_root = os.path.join(args.install, "Resources", "LensProfiles")

    # ---------------- DCP -------------------------------------------------
    dcp_files = []
    for dp, _d, fn in os.walk(cam_root):
        for f in fn:
            if f.lower().endswith(".dcp"):
                dcp_files.append(os.path.join(dp, f))
    dcp_files.sort()

    tag_presence = collections.Counter()
    unknown_tags = collections.Counter()
    unique_models = collections.Counter()
    profile_names = collections.Counter()
    illum_pairs = collections.Counter()
    hsm_dims = collections.Counter()
    look_dims = collections.Counter()
    embed_policy = collections.Counter()
    copyrights = collections.Counter()
    dyn_range = collections.Counter()
    dcp_errors = []
    dcp_examples = []
    by_family = collections.Counter()
    total_bytes = 0

    for i, p in enumerate(dcp_files):
        rel = os.path.relpath(p, args.install).replace("\\", "/")
        sub = os.path.relpath(p, cam_root).replace("\\", "/").split("/")
        fam = "/".join(sub[:-1]) if len(sub) > 1 else "<root>"
        by_family[fam] += 1
        try:
            total_bytes += os.path.getsize(p)
            d = read_dcp(p)
        except Exception as exc:  # noqa: BLE001
            dcp_errors.append({"file": rel,
                               "error": "%s: %s" % (type(exc).__name__, exc)})
            continue
        if "error" in d:
            dcp_errors.append({"file": rel, "error": d["error"]})
            continue
        for name in d["tags"]:
            tag_presence[name] += 1
        for u in d["unknown_tags"]:
            unknown_tags[u["tag"]] += 1
        t = d["tags"]
        if "UniqueCameraModel" in t and "value" in t["UniqueCameraModel"]:
            unique_models[t["UniqueCameraModel"]["value"]] += 1
        if "ProfileName" in t and "value" in t["ProfileName"]:
            profile_names[t["ProfileName"]["value"]] += 1
        i1 = t.get("CalibrationIlluminant1", {}).get("illuminant")
        i2 = t.get("CalibrationIlluminant2", {}).get("illuminant")
        illum_pairs[(i1, i2)] += 1
        if "ProfileHueSatMapDims" in t:
            hsm_dims[tuple(t["ProfileHueSatMapDims"].get("value") or [])] += 1
        if "ProfileLookTableDims" in t:
            look_dims[tuple(t["ProfileLookTableDims"].get("value") or [])] += 1
        if "ProfileEmbedPolicy" in t:
            embed_policy[str(t["ProfileEmbedPolicy"].get("value"))] += 1
        if "ProfileCopyright" in t:
            copyrights[str(t["ProfileCopyright"].get("value"))[:80]] += 1
        if "ProfileDynamicRange" in t:
            dyn_range[str(t["ProfileDynamicRange"].get("value"))] += 1
        if len(dcp_examples) < 3:
            dcp_examples.append({"file": rel, "parsed": d})

    # ---------------- LCP -------------------------------------------------
    lcp_files = []
    for dp, _d, fn in os.walk(lens_root):
        for f in fn:
            if f.lower().endswith(".lcp"):
                lcp_files.append(os.path.join(dp, f))
    lcp_files.sort()

    vocab = collections.Counter()
    ranges: dict[str, tuple] = {}
    lens_models = collections.Counter()
    lens_cameras = collections.Counter()
    lens_makes = collections.Counter()
    model_presence = collections.Counter()
    sample_counts = []
    lcp_errors = []
    lcp_examples = []
    lcp_bytes = 0
    deep_every = max(1, len(lcp_files) // max(1, args.lcp_deep))

    for i, p in enumerate(lcp_files):
        rel = os.path.relpath(p, args.install).replace("\\", "/")
        try:
            lcp_bytes += os.path.getsize(p)
            rec = read_lcp(p, (i % deep_every == 0), vocab, ranges)
        except Exception as exc:  # noqa: BLE001
            lcp_errors.append({"file": rel,
                               "error": "%s: %s" % (type(exc).__name__, exc)})
            continue
        key = rec["lens_pretty"] or rec["lens"] or os.path.basename(p)
        lens_models[key] += 1
        if rec["camera_pretty"] or rec["camera_model"]:
            lens_cameras[rec["camera_pretty"] or rec["camera_model"]] += 1
        lens_makes[rec["make"] or "?"] += 1
        for m in rec["models"]:
            model_presence[m] += 1
        sample_counts.append(rec["samples"])
        if len(lcp_examples) < 2:
            lcp_examples.append({"file": rel, "parsed": rec})

    doc = {
        "schema_id": SCHEMA_ID,
        "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
        "method": {
            "mode": "offline_static_parse",
            "app_launched": False,
            "profile_payloads_copied": False,
            "sources": [
                {"id": "dcp", "classification": "parsed", "root": cam_root,
                 "format": "DNG camera profile: little-endian TIFF-like IFD, "
                           "magic 'IIRC' (0x4352)",
                 "parser": "struct-based IFD walk in this tool; DNG profile "
                           "tags named from the DNG spec numbering"},
                {"id": "lcp", "classification": "parsed", "root": lens_root,
                 "format": "XMP/RDF text, stCamera namespace "
                           "(http://ns.adobe.com/photoshop/1.0/camera-profile)",
                 "parser": "regex attribute/element harvest; full attribute "
                           "vocabulary harvested on a strided subsample"},
            ],
            "classification_legend": {
                "parsed": "read directly out of a shipped file",
                "derived": "computed from parsed data",
                "heuristic": "this tool's judgement",
            },
        },
        "counts": {
            "dcp_files_parsed": len(dcp_files) - len(dcp_errors),
            "dcp_files_found": len(dcp_files),
            "dcp_files_failed": len(dcp_errors),
            "dcp_total_bytes": total_bytes,
            "dcp_distinct_unique_camera_models": len(unique_models),
            "dcp_distinct_profile_names": len(profile_names),
            "lcp_files_parsed": len(lcp_files) - len(lcp_errors),
            "lcp_files_found": len(lcp_files),
            "lcp_files_failed": len(lcp_errors),
            "lcp_total_bytes": lcp_bytes,
            "lcp_distinct_lens_names": len(lens_models),
            "lcp_distinct_calibration_cameras": len(lens_cameras),
            "lcp_calibration_samples_total": sum(sample_counts),
            "lcp_deep_harvest_files": len(lcp_files) // deep_every + 1,
        },
        "camera_profiles_dcp": {
            "classification": "parsed",
            "correction_model": {
                "description": "A .dcp carries a colour-rendering model, not a "
                               "geometric one. The model is: two illuminant-"
                               "referenced 3x3 ColorMatrix (XYZ->camera) plus "
                               "optional ForwardMatrix (camera->XYZ D50) and "
                               "CameraCalibration matrices, an optional 3D "
                               "HueSatMap deformation lattice over "
                               "(hue, saturation, value), an optional "
                               "ProfileLookTable lattice of the same shape "
                               "applied after the HueSatMap, an optional 1D "
                               "ProfileToneCurve, plus BaselineExposureOffset "
                               "and DefaultBlackRender rendering hints.",
                "classification": "derived:from tag presence across the corpus",
            },
            "tag_presence": [
                {"tag_name": k, "files": v,
                 "fraction": round(v / max(1, len(dcp_files)), 4)}
                for k, v in tag_presence.most_common()],
            "unknown_tags_seen": [
                {"tag": k, "files": v} for k, v in unknown_tags.most_common()],
            "files_by_family": dict(by_family.most_common()),
            "calibration_illuminant_pairs": [
                {"illuminant1": k[0], "illuminant2": k[1], "files": v}
                for k, v in illum_pairs.most_common()],
            "hue_sat_map_dims": [
                {"dims_hue_sat_val": list(k), "files": v}
                for k, v in hsm_dims.most_common(20)],
            "look_table_dims": [
                {"dims_hue_sat_val": list(k), "files": v}
                for k, v in look_dims.most_common(20)],
            "profile_embed_policy": dict(embed_policy.most_common()),
            "profile_dynamic_range": dict(dyn_range.most_common()),
            "copyright_strings": dict(copyrights.most_common(10)),
            "profile_names_all": [
                {"name": k, "files": v} for k, v in profile_names.most_common()],
            "unique_camera_models_all": [
                {"model": k, "files": v}
                for k, v in unique_models.most_common()],
            "worked_examples": dcp_examples,
        },
        "lens_profiles_lcp": {
            "classification": "parsed",
            "correction_model": {
                "description": "A .lcp carries geometric and radiometric "
                               "models sampled over a (FocalLength, "
                               "FocusDistance, ApertureValue) grid. Each "
                               "rdf:li sample may carry: PerspectiveModel "
                               "(FocalLengthX/Y, ImageXCenter/YCenter, "
                               "RadialDistortParam1..3, optional "
                               "TangentialDistortParam1..2, ScaleFactor, "
                               "ResidualMeanError/StandardDeviation); "
                               "FisheyeModel (same role, fisheye projection); "
                               "VignetteModel (VignetteModelParam1..3); and "
                               "THREE separate chromatic models - "
                               "ChromaticRedGreenModel, ChromaticGreenModel "
                               "and ChromaticBlueGreenModel - i.e. lateral CA "
                               "is corrected per colour channel pair, not by "
                               "one combined model. A consumer interpolates "
                               "between samples for the shot's focal length, "
                               "focus distance and aperture.",
                "classification": "derived:element names and attribute "
                                  "vocabulary observed across the corpus",
            },
            "model_presence_files": dict(model_presence.most_common()),
            "samples_per_file": {
                "min": min(sample_counts) if sample_counts else None,
                "max": max(sample_counts) if sample_counts else None,
                "mean": round(sum(sample_counts) / len(sample_counts), 2)
                if sample_counts else None,
                "classification": "derived",
            },
            "attribute_vocabulary": [
                {"attribute": "stCamera:" + k, "occurrences": v,
                 "numeric_range": ({"min": ranges[k][0], "max": ranges[k][1]}
                                   if k in ranges else None)}
                for k, v in vocab.most_common()],
            "attribute_vocabulary_note":
                "harvested from a strided subsample of the .lcp corpus; "
                "occurrence counts are subsample counts, not corpus counts",
            "files_by_make": dict(lens_makes.most_common()),
            "lens_names_all": [
                {"lens": k, "files": v} for k, v in lens_models.most_common()],
            "calibration_cameras_all": [
                {"camera": k, "files": v}
                for k, v in lens_cameras.most_common()],
            "worked_examples": lcp_examples,
        },
        "errors": {"dcp": dcp_errors[:50], "lcp": lcp_errors[:50]},
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print(json.dumps(doc["counts"], indent=1))
    if dcp_errors or lcp_errors:
        print("errors dcp=%d lcp=%d" % (len(dcp_errors), len(lcp_errors)),
              file=sys.stderr)


if __name__ == "__main__":
    main()
