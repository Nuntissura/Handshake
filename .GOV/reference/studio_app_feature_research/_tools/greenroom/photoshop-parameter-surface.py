#!/usr/bin/env python
"""
photoshop-parameter-surface.py

Builds a per-feature PARAMETER SPECIFICATION for Adobe Photoshop 2026 for a
native Rust reimplementation.

Nothing here launches Photoshop. The type library is loaded statically from the
plug-in file on disk with pythoncom.LoadTypeLib(); no COM object is Dispatched,
so no Photoshop process is created.

Sources, in order of authority:
  A. ScriptingSupport.8li ITypeLib walk  -> every class, every property with its
     real VARTYPE, every method with typed and named parameters, optional flags
     and default values.
  B. photoshop_enums.json               -> enumerator vocabulary + integer
     values, bound onto every property/parameter whose type is an enum.
  C. photoshop_preset_contents.json     -> Action Descriptor parameter keys
     actually observed in the installed presets, WITH THEIR UNITS
     (#Pxl / #Prc / #Ang / #Dst ...). This is the only source in the offline
     install that carries units and real default values.

Output: photoshop_parameter_surface.json
"""

import datetime
import hashlib
import json
import os
import re
from collections import Counter, OrderedDict, defaultdict

import pythoncom

TYPELIB = (
    r"C:\Program Files\Adobe\Adobe Photoshop 2026"
    r"\Required\Plug-Ins\Extensions\ScriptingSupport.8li"
)
HERE = os.path.dirname(os.path.abspath(__file__))
OUT_DIR = os.path.abspath(
    os.path.join(
        HERE, "..", "..", "_greenroom_20260903",
        "installed_exports", "photoshop", "offline",
    )
)
OUT_FILE = os.path.join(OUT_DIR, "photoshop_parameter_surface.json")
ENUMS_FILE = os.path.join(OUT_DIR, "photoshop_enums.json")
PRESETS_FILE = os.path.join(OUT_DIR, "photoshop_preset_contents.json")

SCHEMA_ID = "handshake.adobe.photoshop.parameter_surface.v1"

# --------------------------------------------------------------------------
# VARTYPE names
# --------------------------------------------------------------------------

VT = {
    0: "VT_EMPTY", 1: "VT_NULL", 2: "VT_I2", 3: "VT_I4", 4: "VT_R4",
    5: "VT_R8", 6: "VT_CY", 7: "VT_DATE", 8: "VT_BSTR", 9: "VT_DISPATCH",
    10: "VT_ERROR", 11: "VT_BOOL", 12: "VT_VARIANT", 13: "VT_UNKNOWN",
    14: "VT_DECIMAL", 16: "VT_I1", 17: "VT_UI1", 18: "VT_UI2", 19: "VT_UI4",
    20: "VT_I8", 21: "VT_UI8", 22: "VT_INT", 23: "VT_UINT", 24: "VT_VOID",
    25: "VT_HRESULT", 26: "VT_PTR", 27: "VT_SAFEARRAY", 28: "VT_CARRAY",
    29: "VT_USERDEFINED", 30: "VT_LPSTR", 31: "VT_LPWSTR", 36: "VT_RECORD",
    37: "VT_INT_PTR", 38: "VT_UINT_PTR", 8192: "VT_ARRAY", 16384: "VT_BYREF",
}

# Rust-facing primitive mapping. Explicitly a MAPPING SUGGESTION, not data
# read from the type library.
RUST_HINT = {
    "VT_I2": "i16", "VT_I4": "i32", "VT_I8": "i64", "VT_INT": "i32",
    "VT_UI1": "u8", "VT_UI2": "u16", "VT_UI4": "u32", "VT_UI8": "u64",
    "VT_UINT": "u32", "VT_R4": "f32", "VT_R8": "f64", "VT_BOOL": "bool",
    "VT_BSTR": "String", "VT_DATE": "DateTime", "VT_VARIANT": "Value (dynamic)",
    "VT_DISPATCH": "object handle", "VT_UNKNOWN": "object handle",
    "VT_VOID": "()", "VT_SAFEARRAY": "Vec<T>", "VT_CARRAY": "Vec<T>",
}

TKIND = {
    0: "TKIND_ENUM", 1: "TKIND_RECORD", 2: "TKIND_MODULE",
    3: "TKIND_INTERFACE", 4: "TKIND_DISPATCH", 5: "TKIND_COCLASS",
    6: "TKIND_ALIAS", 7: "TKIND_UNION",
}

INVKIND = {1: "method", 2: "property_get", 4: "property_put", 8: "property_put_ref"}

# FUNCFLAGS / PARAMFLAGS
PARAMFLAG_FIN = 0x1
PARAMFLAG_FOUT = 0x2
PARAMFLAG_FLCID = 0x4
PARAMFLAG_FRETVAL = 0x8
PARAMFLAG_FOPT = 0x10
PARAMFLAG_FHASDEFAULT = 0x20

# --------------------------------------------------------------------------
# feature grouping of the DOM classes
# --------------------------------------------------------------------------

OPTION_GROUPS = [
    ("save_options", re.compile(r"SaveOptions$")),
    ("open_options", re.compile(r"OpenOptions$")),
    ("export_options", re.compile(r"^ExportOptions")),
    ("automation_options", re.compile(
        r"^(ContactSheetOptions|PicturePackageOptions|PresentationOptions|"
        r"BatchOptions|PhotoshopPDFSaveOptions|GalleryOptions|"
        r"GalleryBannerOptions|GalleryColorOptions|GalleryImagesOptions|"
        r"GallerySecurityOptions|GalleryThumbnailOptions|"
        r"PhotoCDOpenOptions|PDFOpenOptions|EPSOpenOptions|"
        r"CameraRAWOpenOptions|RawFormatOpenOptions|DICOMOpenOptions)$")),
    ("document_and_layer", re.compile(
        r"^(Document|Documents|ArtLayer|ArtLayers|LayerSet|LayerSets|Layer|"
        r"Layers|LayerComp|LayerComps|Channel|Channels|Selection|"
        r"HistoryState|HistoryStates|Snapshot)$")),
    ("text_and_type", re.compile(r"^(TextItem|TextFont|TextFonts|Font)")),
    ("path_and_shape", re.compile(r"(Path|SubPath|PathPoint)")),
    ("color", re.compile(
        r"(Color$|Colors$|^SolidColor|^RGBColor|^CMYKColor|^LabColor|"
        r"^HSBColor|^GrayColor|^NoColor)")),
    ("action_manager", re.compile(r"^Action")),
    ("app_and_prefs", re.compile(r"^(Application|Preferences|Measurement)")),
]

# ArtLayer / Document methods that are filters, adjustments or transforms.
FILTER_PREFIXES = ("apply", "auto", "adjust", "desaturate", "equalize",
                   "invert", "posterize", "threshold", "mixChannels",
                   "photoFilter", "selectiveColor", "shadowHighlight")


def classify_method(name):
    n = name[0].lower() + name[1:] if name else name
    if n.startswith("apply"):
        return "filter"
    if n.startswith("adjust") or n in (
        "autoContrast", "autoLevels", "autoTone", "desaturate", "equalize",
        "invert", "posterize", "threshold", "mixChannels", "photoFilter",
        "selectiveColor", "shadowHighlight", "curves", "levels",
    ):
        return "adjustment"
    if n.startswith(("resize", "rotate", "translate", "scale", "skew",
                     "transform", "flip", "crop", "trim")):
        return "transform"
    if n.startswith(("save", "export", "close", "open", "print", "duplicate",
                     "merge", "flatten", "rasterize", "convert")):
        return "document_command"
    return "method"



# --------------------------------------------------------------------------
# constraints carried inside the type library help strings
# --------------------------------------------------------------------------

_NUM = r"[-+]?\d+(?:\.\d+)?"
RANGE_PATTERNS = [
    ("dash_range", re.compile(r"\(\s*(" + _NUM + r")\s*-\s*(" + _NUM + r")")),
    ("between_range", re.compile(
        r"between\s+(" + _NUM + r")\s+and\s+(" + _NUM + r")", re.I)),
]
MIN_PATTERN = re.compile(r"minimum\s+(" + _NUM + r")", re.I)
MAX_PATTERN = re.compile(r"maximum\s+(" + _NUM + r")", re.I)
DEFAULT_PATTERN = re.compile(r"default:\s*([^);]+)", re.I)
UNIT_PATTERNS = [
    (re.compile(r"in pixels per inch", re.I), "pixels_per_inch"),
    (re.compile(r"in pixels", re.I), "pixels"),
    (re.compile(r"in percent", re.I), "percent"),
    (re.compile(r"in points", re.I), "points"),
    (re.compile(r"in degrees", re.I), "degrees"),
    (re.compile(r"in inches", re.I), "inches"),
    (re.compile(r"unit value", re.I), "unit_value_generic"),
]
ENUM_LIST_PATTERN = re.compile(
    r"\(\s*([A-Z][A-Za-z]+(?:\s*,\s*[A-Z][A-Za-z]+)+"
    r"(?:\s+or\s+[A-Z][A-Za-z]+)?)\s*\)")


def _num(v):
    try:
        return int(v) if re.fullmatch(r"[-+]?\d+", v) else float(v)
    except ValueError:
        return v


def constraints_from_doc(doc):
    """Extract range / default / unit constraints stated in the help string.

    The help strings are Adobe's own text shipped inside the type library.
    The EXTRACTION is a regex over that text, so each result carries the
    exact matched substring for audit.
    """
    if not doc:
        return None
    out = OrderedDict()
    for label, rx in RANGE_PATTERNS:
        m = rx.search(doc)
        if m:
            out["minimum"] = _num(m.group(1))
            out["maximum"] = _num(m.group(2))
            out["range_pattern"] = label
            out["range_matched_text"] = m.group(0).strip()
            break
    if "minimum" not in out:
        m = MIN_PATTERN.search(doc)
        if m:
            out["minimum"] = _num(m.group(1))
            out["range_matched_text"] = m.group(0).strip()
        m = MAX_PATTERN.search(doc)
        if m:
            out["maximum"] = _num(m.group(1))
    m = DEFAULT_PATTERN.search(doc)
    if m:
        raw = m.group(1).strip().rstrip(".").strip()
        if raw:
            out["default_raw"] = raw
            out["default"] = _num(raw) if re.fullmatch(
                r"[-+]?\d+(?:\.\d+)?", raw
            ) else (
                True if raw.lower() == "true"
                else False if raw.lower() == "false" else raw
            )
            out["default_matched_text"] = m.group(0).strip()
    for rx, unit in UNIT_PATTERNS:
        if rx.search(doc):
            out["unit"] = unit
            break
    m = ENUM_LIST_PATTERN.search(doc)
    if m:
        out["allowed_values_listed_in_doc"] = [
            t.strip()
            for t in re.split(r",|or", m.group(1))
            if t.strip()
        ]
    if not out:
        return None
    out["source"] = "typelib_help_string"
    out["extraction"] = "regex over the Adobe-authored help string"
    return out


# --------------------------------------------------------------------------
# type resolution
# --------------------------------------------------------------------------


class TypeResolver:
    def __init__(self, tlb):
        self.tlb = tlb
        self.name_by_index = {}
        self.kind_by_index = {}
        n = tlb.GetTypeInfoCount()
        for i in range(n):
            try:
                self.name_by_index[i] = tlb.GetDocumentation(i)[0]
                self.kind_by_index[i] = TKIND.get(
                    tlb.GetTypeInfoType(i), "unknown"
                )
            except pythoncom.com_error:
                pass

    def tdesc(self, ti, td, depth=0):
        """Resolve a TYPEDESC tuple into a readable type record."""
        if depth > 8:
            return {"vartype": "VT_UNKNOWN", "note": "recursion limit"}
        if isinstance(td, tuple):
            head = td[0]
            if not isinstance(head, int):
                # nested ELEMDESC: (TYPEDESC, paramflags[, default])
                return self.tdesc(ti, head, depth + 1)
            if head == pythoncom.VT_PTR:
                inner = self.tdesc(ti, td[1], depth + 1)
                inner = dict(inner)
                inner["by_reference"] = True
                return inner
            if head == pythoncom.VT_SAFEARRAY:
                inner = self.tdesc(ti, td[1], depth + 1)
                return {
                    "vartype": "VT_SAFEARRAY",
                    "element": inner,
                    "rust_hint": "Vec<%s>" % inner.get("rust_hint", "?"),
                }
            if head == pythoncom.VT_CARRAY:
                inner = self.tdesc(ti, td[1], depth + 1)
                return {"vartype": "VT_CARRAY", "element": inner}
            if head == pythoncom.VT_USERDEFINED:
                href = td[1]
                nm = None
                kind = None
                try:
                    rti = ti.GetRefTypeInfo(href)
                    nm = rti.GetDocumentation(-1)[0]
                    kind = TKIND.get(rti.GetTypeAttr().typekind, "unknown")
                except pythoncom.com_error:
                    pass
                return {
                    "vartype": "VT_USERDEFINED",
                    "type_name": nm,
                    "type_kind": kind,
                }
            return {"vartype": VT.get(head, "VT_%s" % (head,))}
        if not isinstance(td, int):
            return {"vartype": "VT_UNKNOWN", "raw": str(td)}
        name = VT.get(td, "VT_%d" % td)
        rec = {"vartype": name}
        if name in RUST_HINT:
            rec["rust_hint"] = RUST_HINT[name]
        return rec


def bind_enum(rec, enum_index):
    """Attach the enumerator vocabulary when a type is one of the DOM enums."""
    if rec.get("vartype") == "VT_USERDEFINED":
        nm = rec.get("type_name")
        if nm and nm in enum_index:
            e = enum_index[nm]
            rec["is_enum"] = True
            rec["enum_name"] = nm
            rec["enum_member_count"] = len(e)
            rec["enum_values"] = e
            rec["rust_hint"] = "enum %s (i32 repr)" % nm
        else:
            rec["is_enum"] = False
            rec["rust_hint"] = "object handle (%s)" % (nm or "unknown")
    elif rec.get("vartype") == "VT_SAFEARRAY" and "element" in rec:
        bind_enum(rec["element"], enum_index)
    return rec


# --------------------------------------------------------------------------
# main walk
# --------------------------------------------------------------------------


def walk_typelib(enum_index):
    tlb = pythoncom.LoadTypeLib(TYPELIB)
    res = TypeResolver(tlb)
    n = tlb.GetTypeInfoCount()
    classes = OrderedDict()
    coclasses = OrderedDict()
    stats = Counter()

    for i in range(n):
        kind = TKIND.get(tlb.GetTypeInfoType(i), "unknown")
        stats[kind] += 1
        name, doc = tlb.GetDocumentation(i)[0], tlb.GetDocumentation(i)[1]
        if kind == "TKIND_COCLASS":
            ti = tlb.GetTypeInfo(i)
            ta = ti.GetTypeAttr()
            impls = []
            for j in range(ta.cImplTypes):
                try:
                    href = ti.GetRefTypeOfImplType(j)
                    rti = ti.GetRefTypeInfo(href)
                    impls.append(rti.GetDocumentation(-1)[0])
                except pythoncom.com_error:
                    pass
            coclasses[name] = {
                "name": name,
                "doc": doc,
                "typeinfo_index": i,
                "implemented_interfaces": impls,
            }
            continue
        if kind not in ("TKIND_DISPATCH", "TKIND_INTERFACE"):
            continue

        ti = tlb.GetTypeInfo(i)
        ta = ti.GetTypeAttr()
        props = OrderedDict()
        methods = []

        for f in range(ta.cFuncs):
            fd = ti.GetFuncDesc(f)
            try:
                names = ti.GetNames(fd.memid)
            except pythoncom.com_error:
                names = []
            fname = names[0] if names else "memid_%d" % fd.memid
            fdoc = ""
            try:
                fdoc = ti.GetDocumentation(fd.memid)[1] or ""
            except pythoncom.com_error:
                pass
            invkind = INVKIND.get(fd.invkind, "invkind_%d" % fd.invkind)

            params = []
            for pi, pd in enumerate(fd.args):
                ptype = bind_enum(res.tdesc(ti, pd[0]), enum_index)
                flags = pd[1]
                prec = OrderedDict()
                prec["name"] = names[pi + 1] if len(names) > pi + 1 else (
                    "arg%d" % pi
                )
                prec["type"] = ptype
                prec["optional"] = bool(flags & PARAMFLAG_FOPT)
                prec["direction"] = (
                    "out"
                    if (flags & PARAMFLAG_FOUT) and not (flags & PARAMFLAG_FIN)
                    else "in"
                )
                if flags & PARAMFLAG_FHASDEFAULT and len(pd) > 2:
                    prec["default"] = _jsonable(pd[2])
                params.append(prec)

            rettd = fd.rettype
            if isinstance(rettd, tuple) and len(rettd) >= 2 and isinstance(
                rettd[1], int
            ) and not isinstance(rettd[0], int):
                rettd = rettd[0]
            elif isinstance(rettd, tuple) and len(rettd) >= 2 and isinstance(
                rettd[0], int
            ) and rettd[0] not in (
                pythoncom.VT_PTR, pythoncom.VT_SAFEARRAY,
                pythoncom.VT_CARRAY, pythoncom.VT_USERDEFINED,
            ):
                rettd = rettd[0]
            ret = bind_enum(res.tdesc(ti, rettd), enum_index)

            if invkind in ("property_get", "property_put", "property_put_ref"):
                p = props.setdefault(
                    fname,
                    OrderedDict(
                        [("name", fname), ("doc", ""), ("readable", False),
                         ("writable", False), ("type", None)],
                    ),
                )
                if fdoc and not p["doc"]:
                    p["doc"] = fdoc
                    c = constraints_from_doc(fdoc)
                    if c:
                        p["constraints"] = c
                if invkind == "property_get":
                    p["readable"] = True
                    p["type"] = ret
                else:
                    p["writable"] = True
                    if p["type"] is None and params:
                        p["type"] = params[-1]["type"]
                if params and invkind == "property_get":
                    p["indexed_by"] = [q["name"] for q in params]
            else:
                mcons = constraints_from_doc(fdoc)
                methods.append(
                    OrderedDict(
                        [
                            ("name", fname),
                            ("doc", fdoc),
                            ("constraints", mcons),
                            ("kind", classify_method(fname)),
                            ("parameters", params),
                            ("parameter_count", len(params)),
                            ("required_parameter_count",
                             sum(1 for q in params if not q["optional"])),
                            ("returns", ret),
                        ]
                    )
                )

        for v in range(ta.cVars):
            vd = ti.GetVarDesc(v)
            try:
                vn = ti.GetNames(vd.memid)[0]
            except pythoncom.com_error:
                vn = "var_%d" % vd.memid
            props.setdefault(
                vn,
                OrderedDict(
                    [("name", vn), ("doc", ""), ("readable", True),
                     ("writable", True),
                     ("type", bind_enum(res.tdesc(ti, vd.elemdescVar[0]),
                                        enum_index))],
                ),
            )

        classes[name] = OrderedDict(
            [
                ("name", name),
                ("doc", doc),
                ("typekind", kind),
                ("typeinfo_index", i),
                ("property_count", len(props)),
                ("method_count", len(methods)),
                ("properties", list(props.values())),
                ("methods", methods),
            ]
        )

    return classes, coclasses, dict(stats), n


def _jsonable(v):
    if isinstance(v, (str, int, float, bool)) or v is None:
        return v
    return str(v)


# --------------------------------------------------------------------------
# descriptor-level evidence from the installed presets (units + real values)
# --------------------------------------------------------------------------

UNIT_NAMES = {
    "#Pxl": "pixels", "#Prc": "percent", "#Ang": "degrees",
    "#Rlt": "distance (relative)", "#Dst": "distance", "#Nne": "none",
    "#Pnt": "points", "#Mlm": "millimetres", "#Inch": "inches",
    "#Rsl": "resolution (pixels per inch)", "#Dnt": "density",
}


def descriptor_evidence():
    """Harvest observed Action Descriptor parameter keys, types and units."""
    if not os.path.isfile(PRESETS_FILE):
        return {"available": False, "reason": "photoshop_preset_contents.json not found"}
    with open(PRESETS_FILE, encoding="utf-8") as fh:
        pc = json.load(fh)

    by_class = defaultdict(lambda: defaultdict(Counter))
    key_units = defaultdict(Counter)
    key_enums = defaultdict(Counter)
    key_sources = defaultdict(set)
    tool_classes = defaultdict(lambda: defaultdict(Counter))
    step_events = Counter()
    event_params = defaultdict(Counter)

    unit_re = re.compile(r"^-?[\d.eE+]+ (#\w+)$")

    def absorb(cls, params, bucket, source):
        if not isinstance(params, dict):
            return
        for k, v in params.items():
            key_sources[k].add(source)
            if isinstance(v, str):
                m = unit_re.match(v)
                if m:
                    key_units[k][m.group(1)] += 1
                    bucket[cls][k]["unitfloat"] += 1
                    continue
                if "." in v and v.split(".")[0].isalnum() and len(
                    v.split(".")[0]
                ) <= 8 and not v.startswith("["):
                    key_enums[k][v] += 1
                    bucket[cls][k]["enum"] += 1
                    continue
                bucket[cls][k]["text"] += 1
            elif isinstance(v, bool):
                bucket[cls][k]["bool"] += 1
            elif isinstance(v, (int, float)):
                bucket[cls][k]["number"] += 1
            else:
                bucket[cls][k]["other"] += 1

    for c in pc.get("containers", []):
        fam = c.get("family")
        for e in c.get("entries", []) or []:
            if not isinstance(e, dict):
                continue
            if fam == "tool_presets":
                absorb(e.get("tool_class") or "unknown_tool",
                       e.get("params"), tool_classes, "tool_preset")
                continue
            cls = (
                e.get("gradient_class")
                or e.get("brush_class")
                or e.get("style_class")
                or fam
            )
            absorb(cls, e.get("params"), by_class, fam)
            absorb(cls, e.get("effects"), by_class, fam)
            for st in e.get("steps", []) or []:
                ev = st.get("event_id")
                if isinstance(ev, str):
                    step_events[ev] += 1
                    if st.get("params"):
                        for k in st["params"]:
                            event_params[ev][k] += 1
                        absorb(st.get("param_class") or ev, st.get("params"),
                               by_class, "action_step")

    def pack(bucket):
        out = OrderedDict()
        for cls in sorted(bucket):
            keys = []
            for k in sorted(bucket[cls]):
                rec = OrderedDict()
                rec["key"] = k
                rec["observed_types"] = dict(bucket[cls][k])
                if key_units.get(k):
                    rec["units_observed"] = {
                        u: {"count": n, "meaning": UNIT_NAMES.get(u, "unknown")}
                        for u, n in key_units[k].most_common()
                    }
                if key_enums.get(k):
                    rec["enum_values_observed"] = [
                        v for v, _ in key_enums[k].most_common(40)
                    ]
                keys.append(rec)
            out[cls] = {"parameter_count": len(keys), "parameters": keys}
        return out

    return {
        "available": True,
        "source": "photoshop_preset_contents.json",
        "descriptor_classes": pack(by_class),
        "tool_preset_classes": pack(tool_classes),
        "action_events_observed": OrderedDict(
            (k, {"occurrences": v, "parameter_keys": sorted(event_params[k])})
            for k, v in step_events.most_common()
        ),
        "unit_vocabulary": OrderedDict(
            (u, {"meaning": UNIT_NAMES.get(u, "unknown"),
                 "total_occurrences": sum(
                     c[u] for c in key_units.values() if u in c)})
            for u in sorted({u for c in key_units.values() for u in c})
        ),
    }


# --------------------------------------------------------------------------


def sha1_of(path):
    h = hashlib.sha1()
    with open(path, "rb") as fh:
        while True:
            b = fh.read(1 << 20)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def main():
    enum_index = {}
    enum_meta = {"available": False}
    if os.path.isfile(ENUMS_FILE):
        with open(ENUMS_FILE, encoding="utf-8") as fh:
            ed = json.load(fh)
        for e in ed.get("enums", []):
            if e.get("source") == "typelib" and e.get("values_recovered"):
                enum_index[e["name"]] = [
                    {"name": m["name"], "value": m["value"]}
                    for m in e.get("members", [])
                ]
        enum_meta = {
            "available": True,
            "source": "photoshop_enums.json",
            "enums_bound": len(enum_index),
        }

    classes, coclasses, kind_stats, ti_count = walk_typelib(enum_index)

    # group classes by feature area
    groups = defaultdict(list)
    for name in classes:
        placed = False
        for gname, rx in OPTION_GROUPS:
            if rx.search(name.lstrip("_")):
                groups[gname].append(name)
                placed = True
                break
        if not placed:
            groups["other"].append(name)

    # filters / adjustments / transforms surfaced across every class
    feature_methods = defaultdict(list)
    for cname, c in classes.items():
        for m in c["methods"]:
            if m["kind"] in ("filter", "adjustment", "transform"):
                feature_methods[m["kind"]].append(
                    {
                        "declaring_class": cname,
                        "name": m["name"],
                        "doc": m["doc"],
                        "parameters": m["parameters"],
                        "parameter_count": m["parameter_count"],
                        "required_parameter_count":
                            m["required_parameter_count"],
                        "returns": m["returns"],
                    }
                )

    # de-duplicate: the typelib declares each member on both the underscore
    # dispinterface and its coclass-facing twin
    def dedupe(rows):
        seen = {}
        for r in rows:
            key = (r["name"], r["parameter_count"])
            if key not in seen:
                seen[key] = r
                r["also_declared_on"] = []
            else:
                seen[key]["also_declared_on"].append(r["declaring_class"])
        return list(seen.values())

    for k in list(feature_methods):
        feature_methods[k] = dedupe(feature_methods[k])

    prop_type_hist = Counter()
    enum_bound_props = 0
    total_props = 0
    total_methods = 0
    total_params = 0
    props_with_range = 0
    props_with_default = 0
    props_with_unit = 0
    methods_with_constraints = 0
    for c in classes.values():
        total_methods += c["method_count"]
        for p in c["properties"]:
            total_props += 1
            t = (p.get("type") or {}).get("vartype")
            prop_type_hist[t] += 1
            if (p.get("type") or {}).get("is_enum"):
                enum_bound_props += 1
            con = p.get("constraints") or {}
            if "minimum" in con or "maximum" in con:
                props_with_range += 1
            if "default" in con:
                props_with_default += 1
            if "unit" in con:
                props_with_unit += 1
        for m in c["methods"]:
            total_params += m["parameter_count"]
            if m.get("constraints"):
                methods_with_constraints += 1

    # explicit resolution of the option classes named in the research brief
    requested = [
        "JPEGSaveOptions", "PNGSaveOptions", "PDFSaveOptions",
        "ExportOptionsSaveForWeb", "TiffSaveOptions", "GIFSaveOptions",
        "EPSSaveOptions", "DCS1_SaveOptions", "DCS2_SaveOptions",
        "TargaSaveOptions", "BMPSaveOptions", "PhotoshopSaveOptions",
        "RawSaveOptions", "SGIRGBSaveOptions", "PixarSaveOptions",
        "PICTFileSaveOptions", "ContactSheetOptions",
        "PicturePackageOptions", "PresentationOptions", "BatchOptions",
        "ExportOptionsIllustrator", "PDFOpenOptions", "EPSOpenOptions",
        "CameraRAWOpenOptions", "DICOMOpenOptions", "PhotoCDOpenOptions",
        "RawFormatOpenOptions", "BitmapConversionOptions",
        "IndexedConversionOptions", "GalleryOptions", "GalleryBannerOptions",
        "GalleryCustomColorOptions", "GalleryImagesOptions",
        "GallerySecurityOptions", "GalleryThumbnailOptions",
    ]
    requested_rows = []
    for want in requested:
        iface = "_" + want
        found = iface if iface in classes else (
            want if want in classes else None
        )
        if found:
            c = classes[found]
            requested_rows.append(
                {
                    "requested": want,
                    "resolved_interface": found,
                    "coclass": want if want in coclasses else None,
                    "property_count": c["property_count"],
                    "method_count": c["method_count"],
                    "enum_typed_properties": sum(
                        1 for p in c["properties"]
                        if (p.get("type") or {}).get("is_enum")
                    ),
                    "status": "found",
                }
            )
        else:
            requested_rows.append(
                {"requested": want, "status": "NOT_PRESENT_IN_TYPELIB"}
            )

    desc = descriptor_evidence()

    doc = OrderedDict()
    doc["schema_id"] = SCHEMA_ID
    doc["generated_at"] = datetime.datetime.now(
        datetime.timezone.utc
    ).isoformat()
    doc["generator"] = "photoshop-parameter-surface.py"
    doc["app"] = "Adobe Photoshop 2026"
    doc["process_launched"] = False
    doc["process_launch_note"] = (
        "pythoncom.LoadTypeLib() reads the type library resource statically "
        "from ScriptingSupport.8li. No COM object was Dispatched and no "
        "Photoshop process was created."
    )
    doc["method"] = (
        "SECTION A (authoritative, fully parsed): a direct ITypeLib walk of "
        "ScriptingSupport.8li. Every TKIND_DISPATCH and TKIND_INTERFACE "
        "typeinfo was enumerated; for each, every FUNCDESC was read and split "
        "by INVOKEKIND into property_get / property_put / method, and every "
        "VARDESC was read. Parameter names come from ITypeInfo::GetNames "
        "(index 0 is the member name, 1..n are the parameter names). "
        "Parameter and return TYPEDESCs were resolved recursively: VT_PTR is "
        "unwrapped and marked by_reference, VT_SAFEARRAY/VT_CARRAY record "
        "their element type, and VT_USERDEFINED is resolved through "
        "GetRefTypeInfo to the referenced type's real name and typekind. "
        "PARAMFLAG_FOPT gives the optional flag, PARAMFLAG_FOUT the "
        "direction, PARAMFLAG_FHASDEFAULT the default value. "
        "SECTION B (authoritative): every resolved VT_USERDEFINED whose "
        "referenced type is one of the 130 TKIND_ENUM typeinfos is bound to "
        "its full enumerator list with integer values, taken from "
        "photoshop_enums.json. A property typed as an enum therefore carries "
        "its complete legal value set inline. "
        "SECTION C (observational, not a declaration): Action Descriptor "
        "parameter keys harvested from photoshop_preset_contents.json - the "
        "presets shipped in the install. This is the ONLY offline source that "
        "carries UNITS (#Pxl pixels, #Prc percent, #Ang degrees ...) and real "
        "shipped values. It is evidence of what the parameters ARE and what "
        "units they use, but it is a sample of shipped presets, NOT a "
        "declaration of the full legal parameter set or of value ranges. "
        "Treated and labelled accordingly. "
        "SECTION D (parsed from Adobe-authored text): the type library's own "
        "help strings frequently state ranges, defaults, units and allowed "
        "values, e.g. JPEGSaveOptions.Quality is documented as "
        "'quality of produced image ( 0 - 12; default: 3 )'. Those strings "
        "are parsed into a `constraints` object on the property or method, "
        "always carrying the exact matched substring so the extraction can be "
        "audited. The TEXT is Adobe's; the EXTRACTION is a regex, so treat a "
        "constraints object as strong but machine-derived evidence, not as a "
        "declared machine-readable contract. Only a minority of members carry "
        "such text - see the summary counters for exactly how many."
    )
    doc["source_files"] = [
        {
            "path": TYPELIB,
            "bytes": os.path.getsize(TYPELIB),
            "sha1": sha1_of(TYPELIB),
            "role": "type library (Section A)",
        },
        {"path": ENUMS_FILE, "role": "enumerator vocabulary (Section B)",
         "present": os.path.isfile(ENUMS_FILE)},
        {"path": PRESETS_FILE, "role": "descriptor evidence (Section C)",
         "present": os.path.isfile(PRESETS_FILE)},
    ]
    doc["corrections"] = {
        "supersedes": "dom_typelib.json",
        "false_claims": [
            {
                "claim": "class_count: 83",
                "reality": (
                    "The type library holds %d typeinfos: %s. The earlier "
                    "figure came from introspecting makepy's generated module "
                    "and dropping every underscore-prefixed dispinterface, "
                    "while counting the generated 'constants' helper class as "
                    "a class."
                    % (ti_count, ", ".join(
                        "%d %s" % (v, k) for k, v in sorted(kind_stats.items())
                    ))
                ),
            },
            {
                "claim": "792 properties / 381 methods with no types",
                "reality": (
                    "This walk records %d properties and %d methods carrying "
                    "%d typed parameters, each with a resolved VARTYPE and, "
                    "where the type is an enum, its full enumerator list. The "
                    "earlier dump recorded only get/put booleans and a "
                    "printed Python signature - no types at all."
                    % (total_props, total_methods, total_params)
                ),
            },
        ],
    }
    doc["summary"] = {
        "typeinfo_count": ti_count,
        "typeinfo_kinds": kind_stats,
        "interfaces_walked": len(classes),
        "coclasses": len(coclasses),
        "properties_total": total_props,
        "properties_typed_as_enum": enum_bound_props,
        "methods_total": total_methods,
        "method_parameters_total": total_params,
        "property_vartype_histogram": dict(prop_type_hist.most_common()),
        "enums_bound": enum_meta.get("enums_bound", 0),
        "filter_methods": len(feature_methods.get("filter", [])),
        "adjustment_methods": len(feature_methods.get("adjustment", [])),
        "transform_methods": len(feature_methods.get("transform", [])),
        "properties_with_range_from_help_string": props_with_range,
        "properties_with_default_from_help_string": props_with_default,
        "properties_with_unit_from_help_string": props_with_unit,
        "methods_with_constraints_from_help_string": methods_with_constraints,
        "requested_option_classes_found": sum(
            1 for r in requested_rows if r["status"] == "found"
        ),
        "requested_option_classes_missing": sum(
            1 for r in requested_rows if r["status"] != "found"
        ),
    }
    doc_requested = requested_rows
    doc["enum_binding"] = enum_meta
    doc["requested_option_classes"] = doc_requested
    doc["class_groups"] = OrderedDict(
        (g, sorted(v)) for g, v in sorted(groups.items())
    )
    doc["feature_methods"] = OrderedDict(
        (k, sorted(v, key=lambda r: r["name"]))
        for k, v in sorted(feature_methods.items())
    )
    doc["coclasses"] = coclasses
    doc["classes"] = classes
    doc["descriptor_evidence"] = desc
    doc["unknowns"] = [
        {
            "id": "UNK-PS-001",
            "what": (
                "numeric min/max ranges and step sizes for the MAJORITY of "
                "parameters (see summary.properties_with_range_from_help_"
                "string for how many were recovered from help strings)"
            ),
            "why": (
                "COM type libraries carry no structured range metadata. Where "
                "Adobe happened to write a range into the help string it was "
                "recovered (Section D); everywhere else the range is enforced "
                "inside Photoshop's compiled code and surfaced only by the "
                "dialog widgets at runtime."
            ),
            "where_to_get_it": (
                "photoshop_dialogs.json (Required/layouts/*.eve|.exv) exposes "
                "the control class per parameter - e.g. TSliderFixedPoint - "
                "and some layouts carry explicit slider bounds; that is the "
                "best offline range source. Otherwise Adobe's scripting "
                "reference or runtime probing."
            ),
        },
        {
            "id": "UNK-PS-002",
            "what": "factory default values per parameter",
            "why": (
                "Not declared in the type library. The values in Section C "
                "are the values of SHIPPED PRESETS, which are not the same "
                "thing as a tool's factory default."
            ),
        },
        {
            "id": "UNK-PS-003",
            "what": "semantic documentation for enum members",
            "why": (
                "The type library carries no help strings for enum members; "
                "meaning must come from Adobe's reference or behaviour."
            ),
        },
        {
            "id": "UNK-PS-004",
            "what": (
                "the complete Action Descriptor parameter set for filters and "
                "adjustments that have no DOM method"
            ),
            "why": (
                "Photoshop exposes only a subset of its filters through typed "
                "DOM methods; the rest are reachable only through generic "
                "ActionDescriptor calls whose key vocabulary is not declared "
                "anywhere in the type library. Section C recovers only the "
                "keys that the shipped presets and actions happen to use."
            ),
        },
    ]
    doc["heuristics"] = [
        {
            "id": "HEU-PS-001",
            "what": "the 'kind' label on each method (filter / adjustment / "
                    "transform / document_command / method)",
            "basis": "name-prefix classification, not a declared attribute",
        },
        {
            "id": "HEU-PS-002",
            "what": "class_groups feature grouping",
            "basis": "regex over class names, not a declared taxonomy",
        },
        {
            "id": "HEU-PS-003",
            "what": "rust_hint on every type record",
            "basis": "a suggested Rust mapping, not data from the type library",
        },
        {
            "id": "HEU-PS-004",
            "what": "Section C 'enum_values_observed' and unit inference",
            "basis": (
                "pattern match over flattened descriptor values: a value "
                "matching '<number> #Unit' is read as a unit float, a value "
                "matching 'Type.Value' is read as an enum. Observational."
            ),
        },
    ]
    return doc


if __name__ == "__main__":
    d = main()
    os.makedirs(OUT_DIR, exist_ok=True)
    with open(OUT_FILE, "w", encoding="utf-8") as fh:
        json.dump(d, fh, indent=1, ensure_ascii=False)
    print("wrote", OUT_FILE)
    print(json.dumps(d["summary"], indent=1))
    print("class groups:", {k: len(v) for k, v in d["class_groups"].items()})
