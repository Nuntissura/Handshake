"""Shared offline-teardown helpers for Adobe After Effects 2026.

Read-only. Never launches After Effects or any other application.

Covers:
  * install-root discovery (env-overridable, no hardcoded drive assumptions
    beyond a documented default)
  * Adobe ZString ($$$/key=value) literal extraction from binaries
  * Adobe localizable dictionary (.dat) parsing
  * embedded JSON blob extraction from PE binaries (aelib.dll effect manifest)
  * RIFX ("Egg!" / "FaFX") chunk walking for .aep / .ffx files
  * pard / tdb4 / cdat parameter-record decoding
  * uniform JSON envelope writing (schema_id, generated_at, method,
    app_launched, parsed-vs-heuristic labelling, excluded_ai)
"""

from __future__ import annotations

import datetime as _dt
import json
import os
import re
import struct
import sys

# --------------------------------------------------------------------------
# Roots
# --------------------------------------------------------------------------

DEFAULT_INSTALL_ROOT = r"C:\Program Files\Adobe\Adobe After Effects 2026"
DEFAULT_OUT_ROOT = os.path.join(
    os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))),
    "_greenroom_20260903", "installed_exports", "aftereffects", "offline")


def install_root() -> str:
    return os.environ.get("AE_INSTALL_ROOT", DEFAULT_INSTALL_ROOT)


def support_files() -> str:
    return os.path.join(install_root(), "Support Files")


def out_root() -> str:
    p = os.environ.get("AE_OUT_ROOT", DEFAULT_OUT_ROOT)
    os.makedirs(p, exist_ok=True)
    return p


def user_data_root() -> str:
    return os.path.join(os.environ.get("APPDATA", ""), "Adobe", "After Effects")


def rel(path: str) -> str:
    """Path relative to the install root, for auditable evidence references."""
    try:
        return os.path.relpath(path, install_root()).replace("\\", "/")
    except ValueError:
        return path.replace("\\", "/")


# --------------------------------------------------------------------------
# File walking
# --------------------------------------------------------------------------

def iter_files(root: str, exts=None, skip_dirs=()):
    exts = tuple(e.lower() for e in exts) if exts else None
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in skip_dirs]
        for fn in filenames:
            if exts and not fn.lower().endswith(exts):
                continue
            yield os.path.join(dirpath, fn)


def read_bytes(path: str) -> bytes:
    with open(path, "rb") as fh:
        return fh.read()


# --------------------------------------------------------------------------
# ZStrings
# --------------------------------------------------------------------------

# A ZString literal embedded in an Adobe binary looks like
#   $$$/AE/Fractal_Noise/LStr/0010=Fractal Type
# terminated by NUL. Keys are printable-ASCII without '=' or whitespace.
ZSTR_RE = re.compile(rb"\$\$\$/(?P<key>[!-<>-~]{2,240})=(?P<val>[^\x00]{0,2000})")


def zstrings(data: bytes):
    """Yield (key, value) for every $$$/key=value literal in `data`.

    Values are decoded UTF-8 with latin-1 fallback. Trailing junk is possible
    when the literal is not NUL-terminated within the scanned window; callers
    that need strict values should cross-check against a dictionary file.
    """
    for m in ZSTR_RE.finditer(data):
        key = m.group("key").decode("latin-1")
        raw = m.group("val")
        try:
            val = raw.decode("utf-8")
        except UnicodeDecodeError:
            val = raw.decode("latin-1")
        yield key, val


def read_dictionary(path: str) -> dict:
    """Parse an Adobe localizabledictionary .dat: lines of "$$$/key=value"."""
    out = {}
    with open(path, "rb") as fh:
        raw = fh.read()
    if raw.startswith(b"\xef\xbb\xbf"):
        raw = raw[3:]
    text = raw.decode("utf-8", "replace")
    for line in text.splitlines():
        line = line.strip()
        if not (line.startswith('"$$$/') and line.endswith('"')):
            continue
        body = line[1:-1]
        k, _, v = body.partition("=")
        out[k[4:] if k.startswith("$$$/") else k] = v
    return out


def strip_zstring_key(s: str) -> str:
    """'$$$/AE/X=Display' -> 'Display'; plain text passes through."""
    if s.startswith("$$$/"):
        _, _, v = s.partition("=")
        return v
    return s


def zstring_key_of(s: str) -> str:
    if s.startswith("$$$/"):
        return s.partition("=")[0][4:]
    return ""


# Adobe inline markup used inside ZString values.
ZS_MARKUP = {
    "#{cr}": "\n", "#{nbsp}": "\u00a0", "#{tab}": "\t",
    "#{copy}": "\u00a9", "#{reg}": "\u00ae", "#{tm}": "\u2122",
    "^}": "'", "^{": "'", "^[": "[", "^]": "]",
}


def clean_zvalue(v: str) -> str:
    for k, r in ZS_MARKUP.items():
        v = v.replace(k, r)
    return v


BINARY_EXTS = (".dll", ".exe", ".aex", ".dat", ".prm", ".aegp", ".8ba", ".bundle")

_EN_CACHE = None


def build_english_index(force=False):
    """{zstring key -> {"text":.., "source":..}} for every $$$/key=English
    literal embedded in the shipped binaries.

    After Effects ships localized dictionaries (Dictionaries/<locale>/*.dat) but
    NO en_US dictionary for the application itself: the English source strings
    live inside the binaries. This mirrors what the Premiere teardown found.
    """
    global _EN_CACHE
    if _EN_CACHE is not None and not force:
        return _EN_CACHE
    out = {}
    for p in iter_files(support_files(), BINARY_EXTS,
                        skip_dirs=("Dictionaries", "node_modules", "CEPHtmlEngine")):
        try:
            data = read_bytes(p)
        except OSError:
            continue
        if b"$$$/" not in data:
            continue
        r = rel(p)
        for k, v in zstrings(data):
            if k not in out:
                out[k] = {"text": clean_zvalue(v), "source": r}
    _EN_CACHE = out
    return out


_DICT_CACHE = None


def build_key_inventory(locale="de_DE"):
    """Full ZString key inventory from a shipped localized dictionary."""
    global _DICT_CACHE
    if _DICT_CACHE is not None:
        return _DICT_CACHE
    base = os.path.join(support_files(), "Dictionaries", locale)
    if not os.path.isdir(base):
        _DICT_CACHE = {}
        return _DICT_CACHE
    for fn in os.listdir(base):
        if fn.lower().endswith(".dat"):
            _DICT_CACHE = read_dictionary(os.path.join(base, fn))
            return _DICT_CACHE
    _DICT_CACHE = {}
    return _DICT_CACHE


def en(key, idx=None):
    idx = idx if idx is not None else build_english_index()
    e = idx.get(key)
    return e["text"] if e else None


def keys_under(prefix, idx=None):
    idx = idx if idx is not None else build_english_index()
    return {k: v for k, v in idx.items() if k.startswith(prefix)}


# --------------------------------------------------------------------------
# Embedded JSON blobs inside PE binaries
# --------------------------------------------------------------------------

def extract_json_blobs(data: bytes, anchor: bytes):
    """Find JSON objects in `data` containing `anchor`, by brace matching
    backwards to the opening '{' and forwards to its match. Returns list of
    parsed objects."""
    out = []
    seen = set()
    for m in re.finditer(re.escape(anchor), data):
        # walk back to a '{' at depth 0 that starts a plausible object
        start = data.rfind(b"{", 0, m.start())
        # widen: keep walking back while the candidate does not balance
        best = None
        probe = start
        for _ in range(64):
            if probe < 0:
                break
            obj, end = _try_json(data, probe)
            if obj is not None and end > m.start():
                best = (probe, end, obj)
            probe = data.rfind(b"{", 0, probe)
        if best and best[0] not in seen:
            seen.add(best[0])
            out.append(best[2])
    return out


def extract_json_nul_delimited(data: bytes, anchor: bytes):
    """Adobe stores large JSON manifests as single NUL-terminated C strings.
    Find `anchor`, widen to the surrounding NUL boundaries, json.loads."""
    out = []
    seen = set()
    for m in re.finditer(re.escape(anchor), data):
        s = data.rfind(b"\x00", 0, m.start()) + 1
        e = data.find(b"\x00", m.end())
        if e < 0 or (s, e) in seen:
            continue
        seen.add((s, e))
        chunk = data[s:e].strip()
        if not chunk.startswith(b"{") and not chunk.startswith(b"["):
            continue
        try:
            out.append(json.loads(chunk.decode("utf-8")))
        except Exception:
            continue
    return out


def _try_json(data: bytes, start: int):
    depth = 0
    instr = False
    esc = False
    i = start
    n = len(data)
    limit = min(n, start + 8_000_000)
    while i < limit:
        c = data[i]
        if instr:
            if esc:
                esc = False
            elif c == 0x5C:
                esc = True
            elif c == 0x22:
                instr = False
        else:
            if c == 0x22:
                instr = True
            elif c == 0x7B:
                depth += 1
            elif c == 0x7D:
                depth -= 1
                if depth == 0:
                    chunk = data[start:i + 1]
                    try:
                        return json.loads(chunk.decode("utf-8")), i + 1
                    except Exception:
                        return None, i + 1
            elif c == 0x00:
                return None, i
        i += 1
    return None, limit


# --------------------------------------------------------------------------
# RIFX (.aep / .ffx) chunk walking
# --------------------------------------------------------------------------

CONTAINER_IDS = (b"LIST", b"RIFX", b"RIFF")


class Chunk:
    __slots__ = ("cid", "ltype", "data", "children", "depth")

    def __init__(self, cid, ltype=None, data=b"", depth=0):
        self.cid = cid
        self.ltype = ltype
        self.data = data
        self.children = []
        self.depth = depth

    def __repr__(self):
        t = self.ltype.decode("latin-1") if self.ltype else ""
        return "<%s%s n=%d len=%d>" % (
            self.cid.decode("latin-1"), "(%s)" % t if t else "",
            len(self.children), len(self.data))


def rifx_parse(data: bytes, off=0, end=None, depth=0, maxdepth=64):
    """Parse big-endian RIFX into a list of Chunk."""
    if end is None:
        end = len(data)
    out = []
    while off + 8 <= end:
        cid = data[off:off + 4]
        (size,) = struct.unpack_from(">I", data, off + 4)
        body = off + 8
        bend = body + size
        if bend > end:
            bend = end
        if cid in CONTAINER_IDS and size >= 4 and depth < maxdepth:
            ltype = data[body:body + 4]
            ch = Chunk(cid, ltype, b"", depth)
            ch.children = rifx_parse(data, body + 4, bend, depth + 1, maxdepth)
            out.append(ch)
        else:
            out.append(Chunk(cid, None, data[body:bend], depth))
        off = bend + (size & 1)
    return out


def rifx_iter(chunks):
    for c in chunks:
        yield c
        if c.children:
            yield from rifx_iter(c.children)


def cstr(b: bytes) -> str:
    return b.split(b"\x00", 1)[0].decode("utf-8", "replace")


def utf8_chunk(b: bytes) -> str:
    """AE stores names as: 'Utf8' + u32 length + bytes (tdsn / fnam)."""
    if len(b) >= 8 and b[:4] == b"Utf8":
        (n,) = struct.unpack_from(">I", b, 4)
        return b[8:8 + n].decode("utf-8", "replace")
    return cstr(b)


# --------------------------------------------------------------------------
# pard / tdb4 / cdat
# --------------------------------------------------------------------------

# PF_Param_Type, from the After Effects SDK header AE_Effect.h. Confirmed
# empirically against pard records in shipped .ffx presets (see
# ae_ffx_probe evidence in aftereffects_presets.json.method).
PARAM_TYPES = {
    0: "LAYER", 1: "SLIDER", 2: "FIX_SLIDER", 3: "ANGLE", 4: "CHECKBOX",
    5: "COLOR", 6: "POINT", 7: "POPUP", 8: "CUSTOM", 9: "NO_DATA",
    10: "FLOAT_SLIDER", 11: "ARBITRARY_DATA", 12: "PATH", 13: "GROUP_START",
    14: "GROUP_END", 15: "BUTTON", 16: "RESERVED2", 17: "RESERVED3",
    18: "POINT_3D",
}


# PF_ParamFlags bit names (AE_Effect.h). Corroborated on disk: shipped popup
# params carry 0x42 == SUPERVISE|CANNOT_TIME_VARY, group headers carry 0x20.
PARAM_FLAG_BITS = [
    (1 << 1, "CANNOT_TIME_VARY"),
    (1 << 2, "CANNOT_INTERP"),
    (1 << 5, "COLLAPSE_TWIRLY"),
    (1 << 6, "SUPERVISE"),
    (1 << 7, "USE_VALUE_FOR_OLD_PROJECTS"),
    (1 << 8, "LAYER_PARAM_IS_TRACKMATTE"),
    (1 << 9, "EXCLUDE_FROM_HAVE_INPUTS_CHANGED"),
    (1 << 10, "SKIP_REVEAL_WHEN_UNHIDDEN"),
]

# PF_ValueDisplayFlags (AE_Effect.h). Corroborated: Levels Input/Output
# Black/White carry 0x02 and are shown in 0-255 pixel units; "Blend With
# Original" carries 0x01 and is shown as a percentage.
DISPLAY_FLAG_BITS = [(1, "PERCENT"), (2, "PIXEL"), (4, "RESERVED1"),
                     (8, "REVERSE")]

FIXED = 65536.0


def _bits(v, table):
    return [n for m, n in table if v & m]


def _fixed(b, off):
    return struct.unpack_from(">i", b, off)[0] / FIXED


def decode_pard(b: bytes) -> dict:
    """Decode a 'pard' parameter-definition record (148 bytes in AE 2026).

    Offsets were recovered by differencing records across shipped .ffx presets
    and cross-checking against parameters whose on-disk popup strings ('pdnm')
    make the option count and default index independently verifiable, e.g.
    Fractal Noise "Noise Type" -> 4 options / default 3 ("Soft Linear") and
    "Fractal Type" -> 20 options / default 1 ("Basic"). Slider blocks were
    confirmed against Levels (Gamma 0..5 default 1; Input White default 1) and
    Gaussian Blur (Blurriness valid 0..30000, slider 0..50, default 0).

    Offsets whose meaning is not proven are emitted as `raw_*`.
    """
    d = {"record_bytes": len(b)}
    if len(b) < 0x30:
        d["decode_status"] = "short_record"
        return d
    (ptype,) = struct.unpack_from(">I", b, 0x0C)
    d["param_type_code"] = ptype
    d["param_type"] = PARAM_TYPES.get(ptype, "UNKNOWN_%d" % ptype)
    d["name"] = b[0x10:0x30].split(b"\x00", 1)[0].decode("utf-8", "replace")
    if len(b) < 0x94:
        d["decode_status"] = "truncated"
        return d
    (flags,) = struct.unpack_from(">I", b, 0x30)
    d["param_flags_raw"] = flags
    d["param_flags"] = _bits(flags, PARAM_FLAG_BITS)

    t = d["param_type"]
    if t in ("FLOAT_SLIDER",):
        d["value_kind"] = "float"
        d["valid_min"] = _fin(struct.unpack_from(">f", b, 0x68)[0])
        d["valid_max"] = _fin(struct.unpack_from(">f", b, 0x6C)[0])
        d["slider_min"] = _fin(struct.unpack_from(">f", b, 0x70)[0])
        d["slider_max"] = _fin(struct.unpack_from(">f", b, 0x74)[0])
        d["default"] = _fin(struct.unpack_from(">f", b, 0x78)[0])
        prec = struct.unpack_from(">H", b, 0x7C)[0]
        disp = struct.unpack_from(">H", b, 0x7E)[0]
        d["precision"] = prec if prec <= 12 else None
        d["display_flags"] = _bits(disp, DISPLAY_FLAG_BITS)
        d["value_in_preset"] = _fin(struct.unpack_from(">d", b, 0x38)[0])
    elif t in ("FIX_SLIDER", "ANGLE"):
        d["value_kind"] = "fixed_16_16"
        d["valid_min"] = _fin(_fixed(b, 0x7C))
        d["valid_max"] = _fin(_fixed(b, 0x80))
        d["slider_min"] = _fin(_fixed(b, 0x84))
        d["slider_max"] = _fin(_fixed(b, 0x88))
        d["default"] = _fin(_fixed(b, 0x8C))
        prec = struct.unpack_from(">H", b, 0x90)[0]
        disp = struct.unpack_from(">H", b, 0x92)[0]
        d["precision"] = prec if prec <= 12 else None
        d["display_flags"] = _bits(disp, DISPLAY_FLAG_BITS) if disp < 16 else []
        d["value_in_preset"] = _fin(_fixed(b, 0x38))
        if t == "ANGLE":
            d["units"] = "degrees"
    elif t == "SLIDER":
        d["value_kind"] = "int32_legacy"
        d["valid_min"] = struct.unpack_from(">i", b, 0x7C)[0]
        d["valid_max"] = struct.unpack_from(">i", b, 0x80)[0]
        d["slider_min"] = struct.unpack_from(">i", b, 0x84)[0]
        d["slider_max"] = struct.unpack_from(">i", b, 0x88)[0]
        d["default"] = struct.unpack_from(">i", b, 0x8C)[0]
        d["value_in_preset"] = struct.unpack_from(">i", b, 0x38)[0]
    elif t == "POPUP":
        d["value_kind"] = "enum_index_1_based"
        d["option_count"] = struct.unpack_from(">H", b, 0x3C)[0]
        d["default_index"] = struct.unpack_from(">H", b, 0x3E)[0]
        d["value_in_preset"] = struct.unpack_from(">I", b, 0x38)[0]
    elif t == "CHECKBOX":
        d["value_kind"] = "boolean"
        d["default"] = bool(b[0x3C])
        d["value_in_preset"] = bool(struct.unpack_from(">I", b, 0x38)[0])
    elif t == "COLOR":
        d["value_kind"] = "argb8"
        d["default_argb"] = "#%08X" % struct.unpack_from(">I", b, 0x3C)[0]
        d["value_in_preset_argb"] = "#%08X" % struct.unpack_from(">I", b, 0x38)[0]
    elif t in ("POINT", "POINT_3D"):
        d["value_kind"] = "point_percent_of_layer"
        d["default_x_percent"] = _fin(_fixed(b, 0x44))
        d["default_y_percent"] = _fin(_fixed(b, 0x48))
        if t == "POINT_3D":
            d["default_z_percent"] = _fin(_fixed(b, 0x4C))
    elif t == "LAYER":
        d["value_kind"] = "layer_reference"
    elif t in ("GROUP_START", "GROUP_END", "NO_DATA"):
        d["value_kind"] = "structural"
    elif t == "BUTTON":
        d["value_kind"] = "action"
    elif t == "PATH":
        d["value_kind"] = "mask_path_reference"
    elif t == "ARBITRARY_DATA":
        d["value_kind"] = "arbitrary_blob"
    return d


def _fin(v):
    try:
        if v != v or v in (float("inf"), float("-inf")):
            return None
        if abs(v) > 1e300:
            return None
        return round(v, 10)
    except Exception:
        return None


def decode_cdat(b: bytes):
    """'cdat' holds the stream's static value as big-endian doubles."""
    n = len(b) // 8
    vals = list(struct.unpack_from(">%dd" % n, b, 0)) if n else []
    return [_fin(v) for v in vals]


def decode_tdb4(b: bytes) -> dict:
    """'tdb4' stream descriptor. Only fields proven by differencing are named."""
    d = {"len": len(b)}
    if len(b) >= 8:
        (d["components"],) = struct.unpack_from(">H", b, 2)
    if len(b) >= 0x20:
        (d["flag_word_0x04"],) = struct.unpack_from(">H", b, 0x04)
    if len(b) >= 0x50:
        for off in (0x28, 0x30, 0x38, 0x40, 0x48):
            (v,) = struct.unpack_from(">d", b, off)
            d["raw_%02x_f64" % off] = _fin(v)
    return d


# --------------------------------------------------------------------------
# Output envelope
# --------------------------------------------------------------------------

SCHEMA_VERSION = "1.0.0"

EXCLUDED_AI_NOTE = (
    "Adobe AI / generative surfaces are out of scope for the Handshake Studio "
    "rebuild. They are enumerated with on-disk evidence paths so the exclusion "
    "is auditable, then omitted from every catalogue in this export.")


# Every on-disk artefact that evidences an Adobe AI / generative surface in
# this install. Listed so the exclusion is auditable from any output file on
# its own, then excluded from every catalogue in this export.
AI_EVIDENCE_PATHS = [
    ("Support Files/MLModels", "ML model store"),
    ("Support Files/MLModels/model_metadata.json",
     "ML model manifest: model ids, ONNX/CoreML variants, tensor shapes"),
    ("Support Files/MLModels/FastMask", "FastMask segmentation model payload"),
    ("Support Files/MLModels/ShotCutDetection",
     "Shot-cut / Scene Edit Detection model payload"),
    ("Support Files/FaceTracker", "Face tracking model payload"),
    ("Support Files/FaceTracker/model", "Face tracking model payload"),
    ("Support Files/onnxruntime.dll", "ONNX Runtime inference engine"),
    ("Support Files/onnxruntime_providers_openvino.dll", "ONNX OpenVINO provider"),
    ("Support Files/onnxruntime_providers_shared.dll", "ONNX shared provider"),
    ("Support Files/openvino_onnx_frontend.dll", "OpenVINO ONNX frontend"),
    ("Support Files/DirectML.dll", "DirectML inference backend"),
    ("Support Files/MLFoundation.dll", "Adobe ML foundation runtime"),
    ("Support Files/MLFeatureWrappers.dll", "Adobe ML feature wrappers"),
    ("Support Files/dvamlprocessing.dll", "dva ML processing"),
    ("Support Files/Required/Roto Brush.aex", "Roto Brush plug-in"),
    ("Support Files/Required/RefineMatte.aex", "Refine Matte plug-in"),
    ("Support Files/Required/RefineMatte2.aex", "Refine Matte 2 plug-in"),
]


def default_excluded_ai():
    assets = []
    for relpath, role in AI_EVIDENCE_PATHS:
        full = os.path.join(install_root(), relpath.replace("/", os.sep))
        if os.path.exists(full):
            assets.append({"path": relpath, "exists": True, "role": role})
    return {
        "policy": EXCLUDED_AI_NOTE,
        "on_disk_ai_assets": assets,
        "excluded_surface_list":
            "The per-effect and per-scripting-class exclusion list, with the "
            "reason and evidence path for each, is in "
            "aftereffects_effects_catalogue.json#excluded_ai and "
            "aftereffects_scripting_expressions.json#excluded_ai.",
        "applies_to_this_file":
            "No AI or generative surface is catalogued in this file. Any AI "
            "surface encountered while parsing was dropped after being recorded "
            "in the lists above.",
    }


def write_json(filename: str, schema_id: str, method, payload: dict,
               excluded_ai=None, extra_top=None):
    doc = {
        "schema_id": schema_id,
        "schema_version": SCHEMA_VERSION,
        "generated_at": _dt.datetime.now(_dt.timezone.utc)
                             .strftime("%Y-%m-%dT%H:%M:%SZ"),
        "app_launched": False,
        "source_app": "Adobe After Effects 2026",
        "source_install_root": install_root(),
        "method": method,
    }
    if extra_top:
        doc.update(extra_top)
    doc["excluded_ai"] = (excluded_ai if excluded_ai is not None
                          else default_excluded_ai())
    doc.update(payload)
    path = os.path.join(out_root(), filename)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False, sort_keys=False)
    size = os.path.getsize(path)
    print("WROTE %s  (%.1f KB)" % (path, size / 1024.0), file=sys.stderr)
    return path
