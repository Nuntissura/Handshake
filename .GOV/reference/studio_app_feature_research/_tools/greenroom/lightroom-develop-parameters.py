#!/usr/bin/env python
"""
lightroom-develop-parameters.py

Offline behavioural teardown of Adobe Lightroom Classic's develop-module
parameter surface. Read-only. Never launches Lightroom.

TWO PARSED SOURCES ARE FUSED
  A. XMP wire format - <INSTALL>/Resources/Settings/**/*.xmp
     Camera Raw Settings (crs:) documents: develop presets, look profiles and
     adaptive/mask presets. These are the *interchange* representation, and
     they carry only the properties each preset actually changes.
  B. Catalog engine format - Adobe_imageDevelopSettings.text in a Lightroom
     catalog (SQLite, opened mode=ro&immutable=1, never written).
     Each row is a Lua table written by the develop engine itself and holds
     the COMPLETE resolved setting set for one image. This is what makes
     defaults and full vocabulary recoverable at all.

The two sources use different numeric scales for the same parameter
(XMP normalises many sliders to -1..1; the catalog stores UI scale, e.g.
-100..100). The tool detects and reports that divergence rather than
averaging the two.

Also mines the shipped PE modules for embedded "$$$/key=English" ZSTR pairs
so develop UI labels can be attached as evidence.

Every field carries an explicit classification: "parsed" (read out of a file),
"derived" (computed from parsed data, stated method), or "heuristic"
(this tool's judgement).
"""
from __future__ import annotations

import argparse
import collections
import datetime as _dt
import json
import os
import re
import sqlite3
import sys
import urllib.parse
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lrlua  # noqa: E402

CRS_NS = "http://ns.adobe.com/camera-raw-settings/1.0/"
RDF_NS = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"
SCHEMA_ID = "handshake.adobe.lightroom_classic.develop_parameters.v2"

# ---------------------------------------------------------------------------
# Panel assignment. CURATED / HEURISTIC (this tool's map, not read from disk).
# ---------------------------------------------------------------------------
PANEL_RULES = [
    ("profile", [
        "CameraProfile", "CameraProfileDigest", "ProfileName", "LookName",
        "LookAmount", "LookGroup", "LookUUID", "LookTable", "RGBTables",
        "RGBTable", "RGBTableAmount", "ProfileGainTableMap",
        "ProfileGainTableMapAmount", "ProfileGainTableMapName",
        "RequiresRGBTables", "ConvertToGrayscale", "Look", "Baseline",
        "ColorVariance", "ProfileToneCurve", "ISODependent",
        "OverrideLookVignette", "Amount", "Parameters", "Cluster",
    ], [r"^Look", r"^Profile"]),
    ("basic_whitebalance", [
        "WhiteBalance", "Temperature", "Tint", "IncrementalTemperature",
        "IncrementalTint", "AsShotWhiteXY", "CustomTemperature", "CustomTint",
        "AutoWhiteVersion",
    ], []),
    ("basic_tone", [
        "Exposure", "Exposure2012", "Contrast", "Contrast2012",
        "Highlights2012", "Shadows2012", "Whites2012", "Blacks2012",
        "HighlightRecovery", "FillLight", "Brightness", "Shadows",
        "AutoTone", "AutoExposure", "AutoContrast", "AutoShadows",
        "AutoBrightness", "AutoGrayscaleMix", "AutoGrayMixer",
        "AutoToneDigest", "AutoToneDigestNoSat",
    ], []),
    ("basic_presence", [
        "Texture", "Clarity", "Clarity2012", "Dehaze", "Vibrance",
        "Saturation",
    ], []),
    ("hdr", [
        "HDREditMode", "HDRMaxValue", "SDRBlend", "SDRBrightness",
        "SDRClarity", "SDRContrast", "SDRHighlights", "SDRShadows",
        "SDRWhites",
    ], [r"^SDR", r"^HDR"]),
    ("tone_curve", [
        "ToneCurveName", "ToneCurveName2012", "ToneCurve", "ToneCurvePV2012",
        "ToneCurvePV2012Red", "ToneCurvePV2012Green", "ToneCurvePV2012Blue",
        "ToneCurveRed", "ToneCurveGreen", "ToneCurveBlue",
        "ParametricShadows", "ParametricDarks", "ParametricLights",
        "ParametricHighlights", "ParametricShadowSplit",
        "ParametricMidtoneSplit", "ParametricHighlightSplit",
        "PointCurveEditMode", "CurveRefineSaturation",
    ], [r"^ToneCurve", r"^Parametric"]),
    ("hsl_color_mixer", ["PointColors"], [
        r"^HueAdjustment", r"^SaturationAdjustment", r"^LuminanceAdjustment",
        r"^GrayMixer",
    ]),
    ("color_grading", ["EnableSplitToning"], [r"^ColorGrade", r"^SplitToning"]),
    ("profile_look_filters", ["FilterList"], []),
    ("detail_sharpening", [
        "Sharpness", "SharpenRadius", "SharpenDetail", "SharpenEdgeMasking",
    ], []),
    ("detail_noise", [
        "LuminanceSmoothing", "LuminanceNoiseReductionDetail",
        "LuminanceNoiseReductionContrast", "ColorNoiseReduction",
        "ColorNoiseReductionDetail", "ColorNoiseReductionSmoothness",
        "EnableDenoise", "DenoiseVersion", "DenoiseAmount",
        "MoireFilter",
    ], [r"^Denoise", r"NoiseReduction"]),
    ("lens_corrections", [
        "AutoLateralCA", "ChromaticAberrationR", "ChromaticAberrationB",
        "VignetteAmount", "VignetteMidpoint", "LensManualDistortionAmount",
    ], [r"^LensProfile", r"^Defringe"]),
    ("lens_blur", ["LensBlur"], [r"^LensBlur"]),
    ("transform_geometry", [
        "DistortionCorrectionAlreadyApplied", "VignetteCorrectionAlreadyApplied",
        "LateralChromaticAberrationCorrectionAlreadyApplied",
    ], [r"^Upright", r"^Perspective"]),
    ("effects", [], [r"^PostCropVignette", r"^Grain"]),
    ("calibration", [
        "ShadowTint", "RedHue", "RedSaturation", "GreenHue", "GreenSaturation",
        "BlueHue", "BlueSaturation",
    ], []),
    ("crop_orientation", [
        "CropTop", "CropLeft", "CropBottom", "CropRight", "CropAngle",
        "CropWidth", "CropHeight", "CropUnit", "CropUnits",
        "CropConstrainToWarp", "HasCrop", "Orientation", "ImageOrientation",
        "CropRotationAngle", "CropConstrainAspectRatio",
    ], [r"^Crop"]),
    ("healing_redeye", ["RetouchInfo", "RetouchAreas", "RedEyeInfo",
                        "SpotType", "SourceState", "EnableDistractionRemoval"],
     [r"^Retouch", r"^RedEye"]),
    ("masking_local", [
        "MaskGroupBasedCorrections", "CircularGradientBasedCorrections",
        "GradientBasedCorrections", "PaintBasedCorrections",
        "MaskGroupBasedCorrectionsV2",
    ], [r"^Local", r"^Mask", r"^Correction", r"^Dabs", r"^Range",
        r"^Depth", r"^ReferencePoint", r"^ErrorReason", r"^Wetness"]),
    ("ai_enhance", [], [r"^Enhance"]),
    ("engine_version", [
        "Version", "ProcessVersion", "CompatibleVersion", "HasSettings",
    ], []),
    ("preset_metadata", [
        "Preset", "ToggleStyleAmount", "ToggleStyleDigest", "x-default",
        "PresetType", "UUID", "SupportsAmount", "SupportsAmount2",
        "SupportsColor", "SupportsMonochrome", "SupportsHighDynamicRange",
        "SupportsNormalDynamicRange", "SupportsSceneReferred",
        "SupportsOutputReferred", "CameraModelRestriction", "Copyright",
        "ContactInfo", "Name", "ShortName", "SortName", "Group",
        "Description", "ShowInPresets", "ShowInQuickActions",
        "IncrementalAmount",
    ], [r"^Supports"]),
]

_PANEL_COMPILED = [(p, set(e), [re.compile(r) for r in x])
                   for p, e, x in PANEL_RULES]

LOCAL_CONTEXTS = {
    "MaskGroupBasedCorrections", "CorrectionMasks",
    "CircularGradientBasedCorrections", "GradientBasedCorrections",
    "PaintBasedCorrections", "Masks", "Correction",
}


def assign_panel(name: str, context: str) -> str:
    if context in LOCAL_CONTEXTS:
        return "masking_local"
    if context in ("RetouchInfo", "RedEyeInfo", "RetouchAreas",
                   "pm_patch_variations"):
        return "healing_redeye"
    if context == "ISODependent":
        return "profile"
    if context in ("Look", "Parameters", "LookParameters"):
        return "profile"
    for panel, exacts, regexes in _PANEL_COMPILED:
        if name in exacts:
            return panel
        for rx in regexes:
            if rx.search(name):
                return panel
    return "unclassified"


# ---------------------------------------------------------------------------
_INT_RE = re.compile(r"^[+-]?\d+$")
_FLOAT_RE = re.compile(r"^[+-]?(\d+\.\d*|\.\d+|\d+)([eE][+-]?\d+)?$")
_POINTPAIR_RE = re.compile(r"^\s*[+-]?[\d.]+\s+[+-]?[\d.]+\s*$")


def classify_scalar(v):
    if isinstance(v, bool):
        return "boolean", 1.0 if v else 0.0
    if isinstance(v, int):
        return "integer", float(v)
    if isinstance(v, float):
        return "real", v
    if v is None:
        return "nil", None
    if isinstance(v, (list, dict)):
        return "structured", None
    s = str(v).strip()
    if s in ("True", "False", "true", "false"):
        return "boolean", 1.0 if s.lower() == "true" else 0.0
    if _INT_RE.match(s):
        return "integer", float(s)
    if _FLOAT_RE.match(s):
        return "real", float(s)
    if _POINTPAIR_RE.match(s):
        return "point_pair", None
    return "string", None


class Stat:
    __slots__ = ("count", "types", "lo", "hi", "values", "overflow",
                 "docs", "example")

    def __init__(self):
        self.count = 0
        self.types = collections.Counter()
        self.lo = None
        self.hi = None
        self.values = collections.Counter()
        self.overflow = False
        self.docs = 0
        self.example = None

    def add(self, value):
        self.count += 1
        t, num = classify_scalar(value)
        self.types[t] += 1
        if self.example is None:
            self.example = _short(value)
        if num is not None:
            self.lo = num if self.lo is None else min(self.lo, num)
            self.hi = num if self.hi is None else max(self.hi, num)
        key = _short(value)
        if len(self.values) < 4096:
            self.values[key] += 1
        else:
            self.overflow = True

    def dump(self, top=20):
        d = {
            "occurrences": self.count,
            "documents_containing": self.docs,
            "observed_types": dict(self.types),
            "dominant_type": max(self.types, key=self.types.get)
            if self.types else None,
            "distinct_values_observed": (">4096" if self.overflow
                                         else len(self.values)),
            "observed_values_top": [{"value": v, "count": c}
                                    for v, c in self.values.most_common(top)],
            "example_value": self.example,
        }
        if self.lo is not None:
            d["observed_numeric_range"] = {"min": self.lo, "max": self.hi}
        return d


def _short(v, n=160):
    if isinstance(v, (dict, list)):
        try:
            s = json.dumps(lrlua.jsonable(v), ensure_ascii=False)
        except Exception:  # noqa: BLE001
            s = repr(v)
    else:
        s = str(v)
    return s if len(s) <= n else s[:n] + "\u2026"


# ---------------------------------------------------------------------------
# SOURCE A: XMP presets
# ---------------------------------------------------------------------------
def local(tag):
    return tag.split("}", 1)[1] if "}" in tag else tag


def ns_of(tag):
    return tag[1:].split("}", 1)[0] if tag.startswith("{") else ""


def harvest_xmp(elem, context, stats, shapes, seen):
    for k, v in elem.attrib.items():
        if ns_of(k) != CRS_NS:
            continue
        name = local(k)
        stats[(context, name)].add(v)
        shapes[context].add(name)
        seen.add((context, name))
    for child in list(elem):
        cns, cname = ns_of(child.tag), local(child.tag)
        if cns != CRS_NS:
            harvest_xmp(child, context, stats, shapes, seen)
            continue
        kids = list(child)
        if not kids:
            stats[(context, cname)].add((child.text or "").strip())
            shapes[context].add(cname)
            seen.add((context, cname))
            continue
        kind = local(kids[0].tag)
        if kind in ("Alt", "Bag", "Seq"):
            items = list(kids[0])
            simple = all(not list(i) and not any(ns_of(a) == CRS_NS
                                                 for a in i.attrib)
                         for i in items)
            st = stats[(context, cname)]
            shapes[context].add(cname)
            seen.add((context, cname))
            if simple:
                st.add(" | ".join((i.text or "").strip() for i in items))
                st.types["container:" + kind.lower()] += 1
            else:
                st.count += 1
                st.types["container:%s_of_struct" % kind.lower()] += 1
                for i in items:
                    harvest_xmp(i, cname, stats, shapes, seen)
                    for gk in list(i):
                        harvest_xmp(gk, cname, stats, shapes, seen)
        else:
            harvest_xmp(child, cname, stats, shapes, seen)


def scan_xmp(install):
    root = os.path.join(install, "Resources", "Settings")
    stats = collections.defaultdict(Stat)
    shapes = collections.defaultdict(set)
    docs = []
    errors = []
    scanned = 0
    for dp, _dn, fn in os.walk(root):
        for f in fn:
            if not f.lower().endswith(".xmp"):
                continue
            scanned += 1
            full = os.path.join(dp, f)
            rel = os.path.relpath(full, install).replace("\\", "/")
            try:
                tree = ET.parse(full)
            except Exception as exc:  # noqa: BLE001
                errors.append({"file": rel,
                               "error": "%s: %s" % (type(exc).__name__, exc)})
                continue
            top = next(tree.getroot().iter("{%s}Description" % RDF_NS), None)
            if top is None:
                errors.append({"file": rel, "error": "no rdf:Description"})
                continue
            seen = set()
            harvest_xmp(top, "root", stats, shapes, seen)
            for key in seen:
                stats[key].docs += 1
            grp = ""
            for g in top.iter("{%s}Group" % CRS_NS):
                li = next(g.iter("{%s}li" % RDF_NS), None)
                grp = (li.text or "").strip() if li is not None else ""
                break
            docs.append({
                "file": rel,
                "preset_type": top.get("{%s}PresetType" % CRS_NS, ""),
                "cluster": top.get("{%s}Cluster" % CRS_NS, ""),
                "process_version": top.get("{%s}ProcessVersion" % CRS_NS, ""),
                "crs_version": top.get("{%s}Version" % CRS_NS, ""),
                "group": grp,
            })
    return stats, shapes, docs, errors, scanned


# ---------------------------------------------------------------------------
# SOURCE B: catalog develop settings
# ---------------------------------------------------------------------------
def walk_lua(node, context, stats, shapes, seen, depth=0):
    if depth > 8:
        return
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "__array__":
                for item in v:
                    walk_lua(item, context, stats, shapes, seen, depth + 1)
                continue
            stats[(context, k)].add(v)
            shapes[context].add(k)
            seen.add((context, k))
            if isinstance(v, (dict, list)) and v:
                walk_lua(v, k, stats, shapes, seen, depth + 1)
    elif isinstance(node, list):
        for item in node:
            if isinstance(item, (dict, list)):
                walk_lua(item, context, stats, shapes, seen, depth + 1)


def scan_catalog(catalog, limit):
    stats = collections.defaultdict(Stat)
    shapes = collections.defaultdict(set)
    errors = []
    uri = ("file:" + urllib.parse.quote(catalog.replace("\\", "/"))
           + "?mode=ro&immutable=1")
    con = sqlite3.connect(uri, uri=True)
    cur = con.cursor()
    total = cur.execute("select count(*) from Adobe_imageDevelopSettings "
                        "where text is not null").fetchone()[0]
    q = "select text from Adobe_imageDevelopSettings where text is not null"
    if limit:
        q += " limit %d" % limit
    n = 0
    empty = 0
    pv = collections.Counter()
    for (txt,) in cur.execute(q):
        if not txt or not txt.strip():
            empty += 1
            continue
        try:
            _name, tbl = lrlua.parse_table(txt)
        except Exception as exc:  # noqa: BLE001
            errors.append("%s: %s" % (type(exc).__name__, str(exc)[:120]))
            continue
        n += 1
        seen = set()
        walk_lua(tbl, "root", stats, shapes, seen)
        for key in seen:
            stats[key].docs += 1
        pv[str(tbl.get("ProcessVersion"))] += 1
    # extra catalog facts
    facts = {}
    for tname, cols in (
            ("Adobe_imageDevelopSettings",
             ["hasDevelopAdjustments", "hasAIMasks", "hasMasks",
              "hasLensBlur", "hasPointColor", "hasRetouch", "isHdrEditMode",
              "grayscale", "profileCorrections", "removeChromaticAberration",
              "whiteBalance", "processVersion"]),):
        for c in cols:
            try:
                rows = cur.execute(
                    "select %s, count(*) from %s group by 1 order by 2 desc "
                    "limit 12" % (c, tname)).fetchall()
                facts[c] = [{"value": r[0], "rows": r[1]} for r in rows]
            except sqlite3.Error as exc:
                errors.append("%s.%s: %s" % (tname, c, exc))
    con.close()
    return stats, shapes, errors, n, total, dict(pv), facts, empty


# ---------------------------------------------------------------------------
ZSTR_RE = re.compile(rb"\$\$\$/([\x20-\x7e]{4,220}?)=([\x20-\x7e]{0,220}?)\x00")


def mine_zstr(path, prefixes):
    out = {}
    try:
        with open(path, "rb") as fh:
            blob = fh.read()
    except OSError:
        return out
    for m in ZSTR_RE.finditer(blob):
        key = m.group(1).decode("ascii", "replace")
        if any(key.startswith(p) for p in prefixes):
            out.setdefault(key, m.group(2).decode("ascii", "replace"))
    return out


# ---------------------------------------------------------------------------
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install",
                    default=r"C:\Program Files\Adobe\Adobe Lightroom Classic")
    ap.add_argument("--catalog", default=None)
    ap.add_argument("--catalog-limit", type=int, default=0)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    xstats, xshapes, xdocs, xerr, xscanned = scan_xmp(args.install)

    cstats = cshapes = {}
    cerr, crows, ctotal, cpv, cfacts, cempty = [], 0, 0, {}, {}, 0
    cat_note = None
    if args.catalog and os.path.isfile(args.catalog):
        (cstats, cshapes, cerr, crows, ctotal, cpv, cfacts,
         cempty) = scan_catalog(args.catalog, args.catalog_limit)
    else:
        cat_note = "no catalog supplied or path missing: %r" % args.catalog

    zstr = {}
    zsrc = []
    for mod in ("Develop.lrmodule", "Library.lrmodule"):
        p = os.path.join(args.install, mod)
        if os.path.isfile(p):
            got = mine_zstr(p, ("AgDevelop", "AgCameraRaw", "AgLocalCorrection",
                                "AgMask", "AgCrop", "AgProfile",
                                "AgColorGrading", "AgDevelopPanel"))
            zsrc.append({"file": mod, "zstr_keys": len(got)})
            zstr.update(got)
    tail_index = collections.defaultdict(list)
    for k, v in zstr.items():
        tail_index[k.rsplit("/", 1)[-1].lower()].append(
            {"key": "$$$/" + k, "label": v})

    # ---- fuse -------------------------------------------------------------
    names = collections.defaultdict(set)  # name -> contexts
    for (ctx, nm) in list(xstats) + list(cstats):
        names[nm].add(ctx)

    lut_payloads = sorted(n for n in names if n.startswith("Table_"))
    params = []
    for nm in sorted(names):
        if nm.startswith("Table_"):
            continue
        ctxs = sorted(names[nm])
        # dominant context = the one with most occurrences across sources
        occ = collections.Counter()
        for c in ctxs:
            occ[c] += xstats.get((c, nm), Stat()).count
            occ[c] += cstats.get((c, nm), Stat()).count
        dom = occ.most_common(1)[0][0]
        # A parameter that exists at the top level of a settings document is a
        # top-level develop parameter even when a nested Look/Parameters copy
        # of it occurs more often. Panel assignment and the headline stats
        # therefore follow the primary (root-preferring) context.
        primary = "root" if "root" in ctxs else dom
        entry = {
            "name": nm,
            "xmp_property": "crs:" + nm,
            "contexts": ctxs,
            "primary_context": primary,
            "dominant_context_by_occurrence": dom,
            "panel": assign_panel(nm, primary),
            "panel_classification": "heuristic:curated_panel_map_in_this_tool",
            "sources": {},
        }
        dom = primary
        a = {c: xstats[(c, nm)] for c in ctxs if (c, nm) in xstats}
        b = {c: cstats[(c, nm)] for c in ctxs if (c, nm) in cstats}
        if a:
            merged = a[dom] if dom in a else list(a.values())[0]
            entry["sources"]["xmp_presets"] = dict(
                merged.dump(), classification="parsed",
                note="observed across shipped .xmp presets; presets store "
                     "deltas only, so absence is not evidence of absence")
        if b:
            merged = b[dom] if dom in b else list(b.values())[0]
            d = dict(merged.dump(), classification="parsed",
                     note="observed across catalog Adobe_imageDevelopSettings "
                          "rows; engine writes the complete resolved set")
            d["coverage_fraction_of_rows"] = (
                round(merged.docs / crows, 4) if crows else None)
            entry["sources"]["catalog_engine"] = d

        # default derivation
        if b:
            merged = b[dom] if dom in b else list(b.values())[0]
            cov = (merged.docs / crows) if crows else 0.0
            if merged.values and cov >= 0.5:
                val, cnt = merged.values.most_common(1)[0]
                entry["default_value"] = val
                entry["default_confidence"] = (round(cnt / merged.count, 4)
                                               if merged.count else None)
                entry["default_classification"] = (
                    "derived:modal value across %d catalog images, where this "
                    "parameter is written on %.1f%% of rows. The engine writes "
                    "this key on nearly every image, so the mode is the value "
                    "an unedited image carries." % (crows, cov * 100))
            elif merged.values:
                val, cnt = merged.values.most_common(1)[0]
                entry["default_value"] = None
                entry["modal_value_when_present"] = {
                    "value": val,
                    "share_of_rows_that_have_the_key": round(cnt / merged.count, 4),
                }
                entry["default_classification"] = (
                    "not_derivable:the engine writes this key on only %.1f%% "
                    "of rows, i.e. it is omitted when at its default. The "
                    "default is therefore the ABSENT value and cannot be read "
                    "from the data; the observed mode is the mode among "
                    "edited images only." % (cov * 100))
            else:
                entry["default_value"] = None
                entry["default_classification"] = "unavailable"
        else:
            entry["default_value"] = None
            entry["default_classification"] = (
                "unavailable:parameter seen only in XMP presets, which store "
                "deltas; no engine-written row observed")

        # scale divergence between the two representations
        ax = a.get(dom)
        bx = b.get(dom)
        if ax is not None and bx is not None and ax.hi is not None \
                and bx.hi is not None:
            amax = max(abs(ax.lo), abs(ax.hi))
            bmax = max(abs(bx.lo), abs(bx.hi))
            if amax > 0 and bmax > 0:
                ratio = bmax / amax
                if ratio >= 20 or ratio <= 0.05:
                    entry["scale_divergence"] = {
                        "xmp_abs_max": amax, "catalog_abs_max": bmax,
                        "catalog_over_xmp": round(ratio, 3),
                        "classification": "derived:observed_range_ratio",
                        "note": "the XMP interchange scale and the catalog "
                                "engine scale differ for this parameter; a "
                                "reimplementation must convert between them",
                    }
        labs = tail_index.get(nm.lower())
        if labs:
            entry["ui_label_candidates"] = labs[:6]
            entry["ui_label_classification"] = (
                "heuristic:ZSTR key tail matched parameter name")
        params.append(entry)

    # ---- derived observations ------------------------------------------
    always_written = []
    sparse = []
    for p in params:
        ce = p["sources"].get("catalog_engine")
        if not ce:
            continue
        cov = ce.get("coverage_fraction_of_rows")
        if cov is None:
            continue
        if cov >= 0.999:
            always_written.append(p["name"])
        elif cov < 0.5:
            sparse.append({"name": p["name"], "coverage": cov})
    sparse.sort(key=lambda x: -x["coverage"])

    present = {p["name"] for p in params}
    probe = ["ColorGradeShadowHue", "ColorGradeShadowSat",
             "ColorGradeHighlightHue", "ColorGradeHighlightSat"]
    alias_note = None
    absent = [n for n in probe if n not in present]
    if absent:
        alias_note = {
            "finding": "Colour Grading shadow/highlight HUE and SATURATION "
                       "have no ColorGrade* key. They are stored under the "
                       "legacy Split Toning keys, while shadow/highlight "
                       "LUMINANCE and the whole midtone/global wheel do have "
                       "ColorGrade* keys.",
            "absent_keys": absent,
            "present_legacy_equivalents": sorted(
                n for n in present if n.startswith("SplitToning")),
            "present_colorgrade_keys": sorted(
                n for n in present if n.startswith("ColorGrade")),
            "classification": "derived:key presence across both parsed sources",
            "consequence_for_reimplementation":
                "a Colour Grading UI with five wheels maps onto two different "
                "key families; a naive ColorGrade*-only model silently drops "
                "shadow and highlight hue/saturation",
        }

    by_panel = collections.Counter(p["panel"] for p in params)
    both = sum(1 for p in params if len(p["sources"]) == 2)
    only_x = sum(1 for p in params
                 if list(p["sources"]) == ["xmp_presets"])
    only_c = sum(1 for p in params
                 if list(p["sources"]) == ["catalog_engine"])

    struct_ctx = {}
    for src, shp in (("xmp_presets", xshapes), ("catalog_engine", cshapes)):
        struct_ctx[src] = {k: sorted(v) for k, v in sorted(shp.items())}

    doc = {
        "schema_id": SCHEMA_ID,
        "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
        "method": {
            "mode": "offline_static_parse",
            "app_launched": False,
            "writes_to_source": "none; catalog opened mode=ro&immutable=1",
            "sources": [
                {"id": "xmp_presets", "classification": "parsed",
                 "root": os.path.join(args.install, "Resources", "Settings"),
                 "files_scanned": xscanned,
                 "files_failed": len(xerr),
                 "format": "XMP/RDF, crs: namespace "
                           "(http://ns.adobe.com/camera-raw-settings/1.0/)",
                 "parser": "xml.etree.ElementTree, recursive crs harvest of "
                           "attributes, element properties, rdf:Alt/Bag/Seq "
                           "containers and nested correction/mask structs"},
                {"id": "catalog_engine", "classification": "parsed",
                 "catalog": args.catalog,
                 "table": "Adobe_imageDevelopSettings.text",
                 "rows_available": ctotal, "rows_parsed": crows,
                 "rows_with_empty_text_payload": cempty,
                 "format": "Lua table source written by the develop engine",
                 "parser": "lrlua.parse_table (this repo)",
                 "note": cat_note},
                {"id": "zstr_labels", "classification": "parsed",
                 "targets": zsrc,
                 "format": "$$$/key=English ZSTR pairs embedded in PE modules"},
            ],
            "classification_legend": {
                "parsed": "read directly out of a shipped or user file",
                "derived": "computed from parsed data; method stated inline",
                "heuristic": "this tool's judgement; reject freely",
            },
        },
        "counts": {
            "xmp_files_scanned": xscanned,
            "xmp_files_failed": len(xerr),
            "catalog_rows_available": ctotal,
            "catalog_rows_parsed": crows,
            "catalog_rows_empty_text": cempty,
            "catalog_rows_failed": len(cerr),
            "develop_parameters_total": len(params),
            "present_in_both_sources": both,
            "xmp_presets_only": only_x,
            "catalog_engine_only": only_c,
            "look_table_lut_payloads_excluded": len(lut_payloads),
            "parameters_by_panel_heuristic": dict(by_panel),
        },
        "known_limitations": [
            "Observed ranges are authoring ranges, not the engine's clamp "
            "range. They are a lower bound on the true range.",
            "Defaults are modal values across one catalog's images. A "
            "parameter that the catalog owner always edits will report a "
            "skewed default; default_confidence exposes that.",
            "Panel assignment is authored by this tool, not read from disk.",
            "Parameters that neither ship in a preset nor appear in this "
            "catalog are invisible to this method.",
        ],
        "derived_observations": {
            "always_written_core_set": {
                "classification": "derived:written on >=99.9% of catalog rows",
                "note": "the engine emits these on effectively every image, so "
                        "they form the mandatory resolved settings core and "
                        "their modal value is a trustworthy default",
                "count": len(always_written),
                "names": sorted(always_written),
            },
            "sparse_parameters": {
                "classification": "derived:written on <50% of catalog rows",
                "note": "omitted when at their default, so their default is "
                        "the absent value and is NOT recoverable from data",
                "count": len(sparse),
                "parameters": sparse[:200],
            },
            "colour_grading_key_aliasing": alias_note,
        },
        "process_versions": {
            "classification": "parsed",
            "in_xmp_presets": dict(collections.Counter(
                d["process_version"] for d in xdocs)),
            "in_catalog_rows": cpv,
        },
        "preset_document_inventory": {
            "classification": "parsed",
            "by_preset_type": dict(collections.Counter(
                d["preset_type"] for d in xdocs)),
            "by_cluster": dict(collections.Counter(
                d["cluster"] for d in xdocs)),
            "groups": sorted({d["group"] for d in xdocs if d["group"]}),
        },
        "catalog_develop_flags": {
            "classification": "parsed",
            "note": "column-level value distributions on "
                    "Adobe_imageDevelopSettings; these are the engine's own "
                    "per-image capability/state flags",
            "columns": cfacts,
        },
        "struct_contexts": {
            "classification": "parsed",
            "description": "property names observed inside each container "
                           "struct; 'root' is the top-level settings object",
            "by_source": struct_ctx,
        },
        "look_table_lut_payloads": {
            "classification": "parsed",
            "count": len(lut_payloads),
            "note": "crs:Table_<md5> properties carry base85-ish encoded LUT "
                    "payloads for look profiles. They are DATA, not editable "
                    "parameters, and are excluded from the parameter list.",
            "sample_names": lut_payloads[:10],
        },
        "parameters": params,
        "errors": {"xmp": xerr, "catalog": cerr[:50]},
    }

    os.makedirs(os.path.dirname(args.out), exist_ok=True)
    with open(args.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print(json.dumps(doc["counts"], indent=1))


if __name__ == "__main__":
    main()
