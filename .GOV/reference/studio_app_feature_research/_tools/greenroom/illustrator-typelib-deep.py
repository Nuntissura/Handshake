#!/usr/bin/env python
"""illustrator-typelib-deep.py

Offline deep walk of the Adobe Illustrator 30 COM type library embedded in
ScriptingSupport.aip.  Reads the typelib file ONLY -- never creates a
Dispatch object, never launches Illustrator.

Produces:
  illustrator_enums.json             -- 138 enum groups, member names + int values
  illustrator_parameter_surface.json -- per-class property/method specification
                                        with VT types, enum bindings, and the
                                        range/default text mined from typelib docs.

Everything here is PARSED (typelib is authoritative).  The only derived fields
are `range_hint` / `default_hint` / `units_hint`, which are regex extractions
from the Adobe-authored doc strings; they are labelled as such.
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import sys

import pythoncom

TLB_DEFAULT = r"C:\Program Files\Adobe\Adobe Illustrator 2026\Plug-ins\Extensions\ScriptingSupport.aip"

TKIND = {0: "ENUM", 1: "RECORD", 2: "MODULE", 3: "INTERFACE",
         4: "DISPATCH", 5: "COCLASS", 6: "ALIAS", 7: "UNION"}

# VARENUM -> readable scalar name
VT = {
    0: "void", 1: "null", 2: "int16", 3: "int32", 4: "float32", 5: "float64",
    6: "currency", 7: "date", 8: "string", 9: "object", 10: "hresult",
    11: "bool", 12: "variant", 13: "unknown", 14: "decimal", 16: "int8",
    17: "uint8", 18: "uint16", 19: "uint32", 20: "int64", 21: "uint64",
    22: "int", 23: "uint", 24: "void", 25: "hresult", 26: "ptr",
    27: "safearray", 28: "carray", 29: "userdefined", 30: "lpstr",
    31: "lpwstr", 36: "record", 37: "intptr", 38: "uintptr",
}
VT_PTR, VT_SAFEARRAY, VT_CARRAY, VT_USERDEFINED = 26, 27, 28, 29

INVKIND = {1: "method", 2: "propget", 4: "propput", 8: "propputref"}


def fourcc(value):
    """Illustrator enum members are mostly packed 4-char codes (e.g. 'Sele').

    Returns the ASCII rendering when all four bytes are printable, else None.
    DERIVED (mechanical decode of the parsed integer).
    """
    if not isinstance(value, int) or value < 0 or value > 0xFFFFFFFF:
        return None
    b = value.to_bytes(4, "big")
    if all(32 <= c < 127 for c in b):
        s = b.decode("ascii")
        if s.strip():
            return s
    return None


# Option/parameter classes grouped by the product feature they configure.
FEATURE_INDEX = {
    "export_for_screens": ["ExportForScreensOptionsPNG24", "ExportForScreensOptionsPNG8",
                           "ExportForScreensOptionsJPEG", "ExportForScreensOptionsAVIF",
                           "ExportForScreensOptionsWebP",
                           "ExportForScreensOptionsWebOptimizedSVG",
                           "ExportForScreensPDFOptions", "ExportForScreensItemToExport"],
    "export": ["ExportOptionsPNG24", "ExportOptionsPNG8", "ExportOptionsJPEG",
               "ExportOptionsGIF", "ExportOptionsPhotoshop", "ExportOptionsSVG",
               "ExportOptionsTIFF", "ExportOptionsAutoCAD", "ExportOptionsAVIF",
               "ExportOptionsWebP", "ExportOptionsWebOptimizedSVG",
               "ImageCaptureOptions"],
    "save": ["IllustratorSaveOptions", "EPSSaveOptions", "PDFSaveOptions",
             "FXGSaveOptions"],
    "open_place": ["OpenOptions", "PDFFileOptions", "PhotoshopFileOptions",
                   "AutoCADFileOptions"],
    "print": ["PrintOptions", "PrintColorManagementOptions",
              "PrintColorSeparationOptions", "PrintCoordinateOptions",
              "PrintFlattenerOptions", "PrintFontOptions", "PrintJobOptions",
              "PrintPageMarksOptions", "PrintPaperOptions",
              "PrintPostScriptOptions", "Printer", "PrinterInfo", "Paper",
              "PaperInfo", "PPDFile", "PPDFileInfo", "Ink", "InkInfo",
              "Screen", "ScreenInfo", "ScreenSpotFunction"],
    "rasterize": ["RasterizeOptions", "RasterEffectOptions", "RasterItem"],
    "document": ["DocumentPreset", "Document", "Artboard", "View", "Preferences"],
    "text": ["CharacterAttributes", "ParagraphAttributes", "CharacterStyle",
             "ParagraphStyle", "ListStyle", "TabStopInfo", "TextFrame",
             "TextPath", "TextRange", "TextFont", "Story", "InsertionPoint"],
    "path": ["PathItem", "PathPoint", "CompoundPathItem", "GraphItem",
             "MeshItem", "PluginItem", "NonNativeItem"],
    "color": ["CMYKColor", "RGBColor", "GrayColor", "LabColor", "SpotColor",
              "NoColor", "PatternColor", "GradientColor", "Gradient",
              "GradientStop", "Spot", "Swatch", "SwatchGroup"],
    "repeat_objects": ["GridRepeatConfig", "GridRepeatItem", "RadialRepeatConfig",
                       "RadialRepeatItem", "SymmetryRepeatConfig",
                       "SymmetryRepeatItem"],
    "image_trace": ["TracingOptions", "TracingObject"],
    "styles_libraries": ["Brush", "Symbol", "SymbolItem", "GraphicStyle",
                         "Pattern", "Layer", "Variable", "DataSet", "Tag",
                         "Asset", "EmbedItem"],
}

# ---------------------------------------------------------------- doc mining
# Adobe doc strings look like:
#   "number of colors in exported color table ( 2 - 256; default: 128 )"
#   "the resolution in dots per inch (default: 72.0)"
#   "should the resulting image be antialiased (default: true)"
RE_DEFAULT = re.compile(r"default\s*[:=]\s*([^;)\]]+)", re.I)
RE_RANGE_DASH = re.compile(r"(-?\d+(?:\.\d+)?)\s*(?:-|to|\.\.)\s*(-?\d+(?:\.\d+)?)")
RE_RANGE_WORD = re.compile(
    r"(?:range|between|from)\s*[:\s]*\[?\s*(-?\d+(?:\.\d+)?)\s*(?:-|to|,|\.\.)\s*(-?\d+(?:\.\d+)?)",
    re.I)
UNIT_WORDS = [
    ("dots per inch", "dpi"), ("dpi", "dpi"), ("ppi", "ppi"),
    ("percentage", "percent"), ("percent", "percent"), ("%", "percent"),
    ("points", "points"), ("point", "points"),
    ("pixels", "pixels"), ("pixel", "pixels"),
    ("degrees", "degrees"), ("degree", "degrees"),
    ("inches", "inches"), ("millimeters", "millimeters"),
    ("seconds", "seconds"), ("bytes", "bytes"),
]


def mine_doc(doc: str) -> dict:
    """Regex-derive range/default/unit hints from an Adobe doc string.

    HEURISTIC.  The doc text itself is parsed (authoritative Adobe prose);
    the structured extraction is best-effort.
    """
    out = {}
    if not doc:
        return out
    low = doc.lower()

    m = RE_DEFAULT.search(doc)
    if m:
        out["default_hint"] = m.group(1).strip().strip('."\'')

    # prefer an explicit range keyword, else any "a - b" pair that is not the default
    m = RE_RANGE_WORD.search(doc)
    if not m:
        # search inside parenthesised segments first: "( 2 - 256; default: 128 )"
        for seg in re.findall(r"\(([^)]*)\)", doc):
            mm = RE_RANGE_DASH.search(seg.split("default")[0])
            if mm:
                m = mm
                break
    if not m:
        m = RE_RANGE_DASH.search(low.split("default")[0])
    if m:
        try:
            lo, hi = float(m.group(1)), float(m.group(2))
            if lo <= hi:
                out["range_hint"] = {"min": lo, "max": hi}
        except ValueError:
            pass

    for word, unit in UNIT_WORDS:
        if word in low:
            out["units_hint"] = unit
            break
    return out


# ------------------------------------------------------------ typedesc walk
class Resolver:
    def __init__(self, tl):
        self.tl = tl
        self.href_cache = {}
        self.enum_by_name = {}
        self.iface_names = set()

    def name_for_href(self, ti, href):
        key = (id(ti), href)
        if key in self.href_cache:
            return self.href_cache[key]
        try:
            rti = ti.GetRefTypeInfo(href)
            nm = rti.GetDocumentation(-1)[0]
            ta = rti.GetTypeAttr()
            kind = TKIND.get(ta.typekind, str(ta.typekind))
        except Exception:
            nm, kind = None, None
        self.href_cache[key] = (nm, kind)
        return (nm, kind)

    def typedesc(self, ti, td):
        """Return {'type': str, 'enum': name|None, 'ref': name|None}."""
        if td is None:
            return {"type": "void"}
        # td is (vt, ?, ?) or ((vt, href), ?, ?) or nested
        if isinstance(td, tuple) and len(td) == 3 and not isinstance(td[0], tuple):
            head = td[0]
        else:
            head = td
        return self._walk(ti, head)

    def _walk(self, ti, node):
        if isinstance(node, int):
            return {"type": VT.get(node, f"vt{node}")}
        if isinstance(node, tuple):
            vt = node[0]
            if isinstance(vt, tuple):  # nested descriptor
                return self._walk(ti, vt)
            if vt == VT_USERDEFINED:
                nm, kind = self.name_for_href(ti, node[1])
                if kind == "ENUM":
                    return {"type": "enum", "enum": nm}
                if kind == "ALIAS":
                    return {"type": "alias", "ref": nm}
                return {"type": "object", "ref": nm}
            if vt in (VT_PTR, VT_SAFEARRAY, VT_CARRAY):
                inner = self._walk(ti, node[1]) if len(node) > 1 else {"type": "void"}
                if vt == VT_PTR:
                    inner = dict(inner)
                    inner["by_ref"] = True
                    return inner
                return {"type": "array", "of": inner}
            return {"type": VT.get(vt, f"vt{vt}")}
        return {"type": "unknown"}


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tlb", default=TLB_DEFAULT)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    if not os.path.exists(args.tlb):
        print(f"FATAL: typelib not found: {args.tlb}", file=sys.stderr)
        return 2

    tl = pythoncom.LoadTypeLib(args.tlb)
    la = tl.GetLibAttr()
    lib_doc = tl.GetDocumentation(-1)
    guid, lcid, syskind, major, minor, flags = la[0], la[1], la[2], la[3], la[4], la[5]
    res = Resolver(tl)

    n = tl.GetTypeInfoCount()
    enums, dispatches, coclasses = {}, {}, {}

    for i in range(n):
        ti = tl.GetTypeInfo(i)
        ta = ti.GetTypeAttr()
        name, doc = tl.GetDocumentation(i)[0], tl.GetDocumentation(i)[1]
        kind = TKIND.get(ta.typekind, str(ta.typekind))

        if kind == "ENUM":
            members = []
            for v in range(ta.cVars):
                vd = ti.GetVarDesc(v)
                try:
                    mnames = ti.GetNames(vd.memid)
                    mname = mnames[0] if mnames else f"member{v}"
                except Exception:
                    mname = f"member{v}"
                mdoc = ""
                try:
                    mdoc = ti.GetDocumentation(vd.memid)[1] or ""
                except Exception:
                    pass
                entry = {"name": mname, "value": vd.value, "doc": mdoc}
                fc = fourcc(vd.value)
                if fc:
                    entry["fourcc"] = fc
                members.append(entry)
            enums[name] = {"doc": doc or "", "member_count": len(members),
                           "members": members}

        elif kind == "DISPATCH":
            props, methods = {}, []
            for f in range(ta.cFuncs):
                fd = ti.GetFuncDesc(f)
                try:
                    fnames = ti.GetNames(fd.memid)
                except Exception:
                    fnames = ()
                fname = fnames[0] if fnames else f"member{fd.memid}"
                fdoc = ""
                try:
                    fdoc = ti.GetDocumentation(fd.memid)[1] or ""
                except Exception:
                    pass
                ik = INVKIND.get(fd.invkind, str(fd.invkind))
                rt = res.typedesc(ti, fd.rettype)

                if ik in ("propget", "propput", "propputref"):
                    p = props.setdefault(fname, {
                        "name": fname, "readable": False, "writable": False,
                        "type": None, "enum": None, "object_type": None,
                        "doc": "",
                    })
                    if fdoc and not p["doc"]:
                        p["doc"] = fdoc
                    if ik == "propget":
                        p["readable"] = True
                        p["type"] = rt.get("type")
                        p["enum"] = rt.get("enum")
                        p["object_type"] = rt.get("ref")
                    else:
                        p["writable"] = True
                        if fd.args:
                            at = res.typedesc(ti, fd.args[0])
                            if p["type"] in (None, "void"):
                                p["type"] = at.get("type")
                                p["enum"] = at.get("enum")
                                p["object_type"] = at.get("ref")
                else:
                    params = []
                    argnames = list(fnames[1:]) if len(fnames) > 1 else []
                    for ai, a in enumerate(fd.args):
                        at = res.typedesc(ti, a)
                        pn = argnames[ai] if ai < len(argnames) else f"arg{ai}"
                        # paramflags live in a[3] when present
                        optional = False
                        try:
                            pf = a[3] if len(a) > 3 else None
                            if isinstance(pf, int) and (pf & 0x10):
                                optional = True
                        except Exception:
                            pass
                        params.append({"name": pn, "type": at.get("type"),
                                       "enum": at.get("enum"),
                                       "object_type": at.get("ref"),
                                       "optional": optional})
                    methods.append({
                        "name": fname, "doc": fdoc, "returns": rt,
                        "params": params, "param_count": len(params),
                    })
            # attach mined hints
            for p in props.values():
                p.update(mine_doc(p["doc"]))
            for m in methods:
                h = mine_doc(m["doc"])
                if h:
                    m["doc_hints"] = h
            dispatches[name] = {
                "doc": doc or "", "kind": "dispatch",
                "property_count": len(props), "method_count": len(methods),
                "properties": dict(sorted(props.items())),
                "methods": sorted(methods, key=lambda x: x["name"]),
            }

        elif kind == "COCLASS":
            ifaces = []
            for j in range(ta.cImplTypes):
                try:
                    href = ti.GetRefTypeOfImplType(j)
                    rti = ti.GetRefTypeInfo(href)
                    ifaces.append(rti.GetDocumentation(-1)[0])
                except Exception:
                    pass
            coclasses[name] = {"doc": doc or "", "interfaces": ifaces,
                               "default_interface": ifaces[0] if ifaces else None}

    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    common = {
        "generated_at": now,
        "method": {
            "channel": "com_typelib_direct_walk",
            "tool": "illustrator-typelib-deep.py",
            "source_file": args.tlb,
            "typelib_guid": str(guid),
            "typelib_name": lib_doc[0],
            "typelib_doc": lib_doc[1],
            "typelib_version": f"{major}.{minor}",
            "app_launched": False,
            "reads": "ITypeLib/ITypeInfo metadata only; no COM object instantiated",
        },
    }

    # ---------------------------------------------------------------- enums
    enum_out = json.loads(json.dumps(common))
    enum_out["schema_id"] = "handshake.studio.illustrator.enums.v1"
    enum_out["method"]["labelling"] = {
        "enum_names": "parsed",
        "enum_member_names": "parsed",
        "enum_member_values": "parsed",
        "enum_member_docs": "parsed (Adobe-authored, may be empty)",
        "fourcc": "DERIVED (mechanical ASCII decode of the parsed integer; "
                  "Illustrator enum values are packed 4-character codes)",
    }
    enum_out["enum_group_count"] = len(enums)
    enum_out["enum_member_total"] = sum(e["member_count"] for e in enums.values())
    enum_out["enums"] = dict(sorted(enums.items()))

    # ------------------------------------------------------ parameter surface
    # map: which enum is referenced by which class.property
    enum_usage = {}
    for cname, c in dispatches.items():
        for pname, p in c["properties"].items():
            if p.get("enum"):
                enum_usage.setdefault(p["enum"], []).append(f"{cname}.{pname}")
        for m in c["methods"]:
            for pa in m["params"]:
                if pa.get("enum"):
                    enum_usage.setdefault(pa["enum"], []).append(
                        f"{cname}.{m['name']}({pa['name']})")
    enum_out["enum_usage_index"] = {k: sorted(set(v))
                                    for k, v in sorted(enum_usage.items())}
    enum_out["enums_unreferenced"] = sorted(
        [k for k in enums if k not in enum_usage])

    ps = json.loads(json.dumps(common))
    ps["schema_id"] = "handshake.studio.illustrator.parameter_surface.v1"
    ps["method"]["labelling"] = {
        "property_names": "parsed",
        "property_types": "parsed (VARENUM from typelib)",
        "property_enum_binding": "parsed (VT_USERDEFINED href resolved to enum)",
        "property_docs": "parsed (Adobe-authored doc strings)",
        "range_hint/default_hint/units_hint": "HEURISTIC (regex over the parsed doc string)",
        "feature_index": "DERIVED (hand-authored grouping of parsed class names)",
    }
    # Resolve the feature index against real class names (typelib exposes both
    # the coclass `Foo` and its default interface `_Foo`).
    fidx, fmissing = {}, []
    for feat, names in FEATURE_INDEX.items():
        resolved = []
        for nm in names:
            if nm in dispatches:
                resolved.append(nm)
            elif f"_{nm}" in dispatches:
                resolved.append(f"_{nm}")
            else:
                fmissing.append(nm)
        fidx[feat] = {
            "classes": resolved,
            "property_total": sum(dispatches[c]["property_count"] for c in resolved),
            "method_total": sum(dispatches[c]["method_count"] for c in resolved),
        }
    ps["feature_index"] = fidx
    ps["feature_index_unresolved"] = sorted(set(fmissing))
    ps["classes_not_in_feature_index"] = sorted(
        set(dispatches) - {c for v in fidx.values() for c in v["classes"]})
    ps["class_count"] = len(dispatches)
    ps["coclass_count"] = len(coclasses)
    ps["property_total"] = sum(c["property_count"] for c in dispatches.values())
    ps["method_total"] = sum(c["method_count"] for c in dispatches.values())
    ps["coclasses"] = dict(sorted(coclasses.items()))
    ps["classes"] = dict(sorted(dispatches.items()))

    os.makedirs(args.out, exist_ok=True)
    for fname, payload in (("illustrator_enums.json", enum_out),
                           ("illustrator_parameter_surface.json", ps)):
        fp = os.path.join(args.out, fname)
        with open(fp, "w", encoding="utf-8") as fh:
            json.dump(payload, fh, indent=1, ensure_ascii=False)
        print(f"WROTE {fp}  ({os.path.getsize(fp):,} bytes)")

    print(f"enums={len(enums)} members={enum_out['enum_member_total']} "
          f"classes={len(dispatches)} props={ps['property_total']} "
          f"methods={ps['method_total']} coclasses={len(coclasses)}")
    hinted = sum(1 for c in dispatches.values() for p in c["properties"].values()
                 if "range_hint" in p or "default_hint" in p)
    print(f"properties with mined range/default hint: {hinted}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
