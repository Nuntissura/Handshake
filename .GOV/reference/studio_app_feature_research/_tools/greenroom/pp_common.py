"""pp_common.py -- shared offline readers for an installed Adobe Premiere Pro 2026.

NO APPLICATION IS EVER LAUNCHED. Every function here reads bytes off disk.
No Adobe DLL is loaded, no COM object is created, no subprocess of any Adobe
binary is started.

Formats handled (all determined by inspecting the shipped bytes):

1. PremiereData XML  (.epr .sqpreset .prfpset .prproj-xml .kys)
   <PremiereData Version="3">
     <Thing ObjectRef="1"/>
     <Thing ObjectID="1" ClassID="<guid>" Version="n"> ...fields... </Thing>
   Objects form a graph: an element carrying ObjectRef="N" points at the
   element carrying ObjectID="N". parse_premiere_data() indexes them and
   resolve_graph() walks the graph into plain nested dicts.

2. Adobe prop.map v4 XML  (install/xml/*.xml)
   <prop.map version='4'><prop.list><prop.pair><key>K</key><VALUE/></prop.pair>
   Value elements: <string>, <ustring>, <int type= size=>, <float>,
   <true/>, <false/>, <array><array.type>..</array.type>ITEM*</array>,
   <prop.list> (nested map), <data> (base64/hex blob).
   These are serialized dvaui UI node archives and workspace layouts.

3. Adobe "$$$/" localizable strings, embedded as NUL-terminated C strings in
   `Adobe Premiere Pro.exe`. Wire form: `$$$/Name/Space/Key=Default English`.
   The part after the FIRST '=' is the shipped English. There is no separate
   English .zbin/.dat table in Premiere 2026 -- zstring/en_US holds only 2 KB
   for the crash reporter -- so the executable's own string blob IS the
   English string table. Extraction is a strict regex over printable bytes
   terminated by NUL; no code from the executable is run.

4. Eve dialog layout (.eve) -- delegated to dw_eve.py (same grammar).
"""
import base64
import json
import os
import re
import struct
import sys
import time
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

PREMIERE_ROOT = r"C:\Program Files\Adobe\Adobe Premiere Pro 2026"
AME_ROOT = r"C:\Program Files\Adobe\Adobe Media Encoder 2026"
APPDATA_ROOT = os.path.join(os.environ.get("APPDATA", ""), "Adobe", "Premiere Pro")

APP_LAUNCHED = False

# ---------------------------------------------------------------------------
# Adobe AI / generative surfaces -- OUT OF SCOPE, recorded so the exclusion is
# auditable. Every entry names the on-disk evidence that the surface exists.
# ---------------------------------------------------------------------------
EXCLUDED_AI = {
    "policy": ("Adobe AI and generative features are out of scope for the "
               "Handshake Studio Rust rebuild. They are enumerated here from "
               "on-disk evidence so the exclusion is auditable, then excluded "
               "from every catalogue in this export."),
    "surfaces": [
        {"id": "auto_transcription",
         "label": "Automatic speech transcription",
         "evidence_paths": ["AutoTranscription/", "WhisperTokenizer/",
                            "MarianTokenizer/", "BertTokenizer/"],
         "excluded_from": ["premiere_effects_catalogue",
                           "premiere_panels_dialogs",
                           "premiere_commands_shortcuts"]},
        {"id": "auto_captioning",
         "label": "Automatic caption generation from transcript",
         "evidence_paths": ["AutoCaptioningAssets/"],
         "note": ("The caption DATA MODEL and caption export formats are NOT "
                  "AI and remain in scope; only automatic generation is "
                  "excluded."),
         "excluded_from": ["premiere_graphics_text", "premiere_export_pipeline"]},
        {"id": "scene_edit_detection",
         "label": "Scene Edit Detection",
         "evidence_paths": ["MLModels/", "Adobe Premiere Pro.exe string "
                            "namespace $$$/ML/"],
         "excluded_from": ["premiere_commands_shortcuts"]},
        {"id": "enhance_speech",
         "label": "Enhance Speech (generative audio restoration)",
         "evidence_paths": ["xml/BasicAdjustmentsEnhanceSpeechSection.xml",
                            "xml/BasicAdjustmentsEnhanceSpeechSection.V7.xml",
                            "MLModels/"],
         "excluded_from": ["premiere_effects_catalogue",
                           "premiere_panels_dialogs"]},
        {"id": "text_based_editing",
         "label": "Text-Based Editing (edit by transcript)",
         "evidence_paths": ["xml/TEXTBASEDEDITINGWORKSPACELAYOUT.xml",
                            "TextProcessing/"],
         "note": ("TextProcessing/ also holds NON-AI text shaping and "
                  "line-breaking data, which stays in scope for the title "
                  "and caption engine."),
         "excluded_from": ["premiere_panels_dialogs"]},
        {"id": "ml_runtime",
         "label": "ONNX / OpenVINO / DirectML inference runtime and models",
         "evidence_paths": ["MLModels/", "model/", "onnxruntime.dll",
                            "openvino*.dll", "DirectML.dll"],
         "excluded_from": ["premiere_media_io"]},
        {"id": "adobe_firefly_generative",
         "label": "Firefly / generative extend and generative media surfaces",
         "evidence_paths": ["Adobe Premiere Pro.exe string namespaces "
                            "$$$/ML/ and $$$/Premiere/GenerativeExtend"],
         "excluded_from": ["premiere_effects_catalogue"]},
    ],
}

# Namespace / filename fragments used to drop AI rows out of catalogues.
AI_DROP_TOKENS = (
    "autotranscription", "auto_transcription", "autocaption", "auto_caption",
    "speechtotext", "speech_to_text", "transcript", "whisper", "marian",
    "berttokenizer", "enhancespeech", "enhance_speech", "sceneeditdetection",
    "scene_edit_detection", "textbasedediting", "text_based_editing",
    "generativeextend", "generative_extend", "firefly", "mlmodel",
)


def looks_ai(*texts):
    """True when any supplied text names one of the excluded AI surfaces."""
    for t in texts:
        if not t:
            continue
        low = str(t).lower().replace(" ", "").replace("-", "")
        for tok in AI_DROP_TOKENS:
            if tok.replace("_", "") in low:
                return True
    return False


# ---------------------------------------------------------------------------
# 1. PremiereData XML object graph
# ---------------------------------------------------------------------------
def _strip_ns(tag):
    return tag.split("}", 1)[-1] if "}" in tag else tag


# A lone '<' that does not begin a tag. At least one shipped preset
# (Settings/EncoderPresets/DVForDAW25.epr) carries an unescaped '<' inside
# element text, which is malformed XML as shipped.
_STRAY_LT = re.compile(rb"<(?![A-Za-z_/?!])")

LAST_PARSE_REPAIRS = {}


def parse_premiere_data(path):
    """Parse a PremiereData XML file into (objects, root_element).

    objects: {object_id(str): element}

    Two repairs are applied and recorded in LAST_PARSE_REPAIRS[path]:
      * stray C0 control bytes are dropped (some presets embed them)
      * on a parse error, unescaped '<' inside element text is escaped and the
        parse is retried, because at least one shipped preset is malformed XML.
    """
    with open(path, "rb") as fh:
        raw = fh.read()
    repairs = []
    cleaned = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", raw)
    if cleaned != raw:
        repairs.append("dropped C0 control bytes")
    try:
        root = ET.fromstring(cleaned)
    except ET.ParseError:
        patched, n = _STRAY_LT.subn(b"&lt;", cleaned)
        if not n:
            LAST_PARSE_REPAIRS[path] = repairs
            raise
        repairs.append("escaped %d unescaped '<' in element text "
                       "(file is malformed XML as shipped)" % n)
        root = ET.fromstring(patched)
    LAST_PARSE_REPAIRS[path] = repairs
    objects = {}
    for el in root.iter():
        oid = el.get("ObjectID")
        if oid is not None:
            objects[oid] = el
    return objects, root


def el_to_dict(el, objects, seen=None, depth=0, max_depth=60):
    """Recursively convert a PremiereData element into plain Python.

    ObjectRef attributes are followed into the referenced object; cycles are
    cut with a {"$ref": id} marker so the walk always terminates.
    """
    if seen is None:
        seen = set()
    if depth > max_depth:
        return {"$truncated": True}

    ref = el.get("ObjectRef")
    if ref is not None and not el.get("ObjectID"):
        if ref in seen:
            return {"$ref": ref}
        target = objects.get(ref)
        if target is None:
            return {"$unresolved_ref": ref}
        return el_to_dict(target, objects, seen | {ref}, depth + 1, max_depth)

    oid = el.get("ObjectID")
    if oid is not None:
        if oid in seen:
            return {"$ref": oid}
        seen = seen | {oid}

    kids = list(el)
    if not kids:
        txt = (el.text or "").strip()
        return txt

    out = {}
    for attr in ("ClassID", "Version", "Index", "ObjectID"):
        if el.get(attr) is not None:
            out["@" + attr] = el.get(attr)
    for kid in kids:
        tag = _strip_ns(kid.tag)
        idx = kid.get("Index")
        val = el_to_dict(kid, objects, seen, depth + 1, max_depth)
        key = tag
        if idx is not None:
            key = "%s[%s]" % (tag, idx)
        if key in out:
            n = 2
            while "%s#%d" % (key, n) in out:
                n += 1
            key = "%s#%d" % (key, n)
        out[key] = val
    return out


def flat_fields(el):
    """Shallow {childtag: text} for the leaf fields of one element."""
    out = {}
    for kid in el:
        if len(kid) == 0:
            out[_strip_ns(kid.tag)] = (kid.text or "").strip()
    return out


def iter_objects(objects, class_id=None, tag=None):
    for oid, el in objects.items():
        if class_id and el.get("ClassID") != class_id:
            continue
        if tag and _strip_ns(el.tag) != tag:
            continue
        yield oid, el


# ---------------------------------------------------------------------------
# 2. Adobe prop.map v4 XML
# ---------------------------------------------------------------------------
def _propmap_value(el):
    tag = _strip_ns(el.tag)
    if tag == "prop.list":
        return _propmap_list(el)
    if tag in ("string", "ustring"):
        return el.text or ""
    if tag == "int":
        t = (el.text or "0").strip()
        try:
            return int(t)
        except ValueError:
            return t
    if tag in ("float", "double"):
        try:
            return float((el.text or "0").strip())
        except ValueError:
            return el.text
    if tag == "true":
        return True
    if tag == "false":
        return False
    if tag == "array":
        items = []
        for kid in el:
            if _strip_ns(kid.tag) == "array.type":
                continue
            items.append(_propmap_value(kid))
        return items
    if tag == "data":
        raw = (el.text or "").strip()
        return {"$data_bytes": len(raw)}
    if len(el):
        return {_strip_ns(k.tag): _propmap_value(k) for k in el}
    return el.text


def _propmap_list(el):
    """A <prop.list> is either a map of <prop.pair> or a positional list."""
    pairs = [k for k in el if _strip_ns(k.tag) == "prop.pair"]
    if pairs:
        out = {}
        for p in pairs:
            key = None
            val = None
            for kid in p:
                if _strip_ns(kid.tag) == "key":
                    key = kid.text
                else:
                    val = _propmap_value(kid)
            if key is None:
                continue
            if key in out:
                n = 2
                while "%s#%d" % (key, n) in out:
                    n += 1
                key = "%s#%d" % (key, n)
            out[key] = val
        return out
    return [_propmap_value(k) for k in el]


def parse_propmap(path):
    """Return the top-level dict of an Adobe prop.map v4 file."""
    with open(path, "rb") as fh:
        raw = fh.read()
    raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", raw)
    root = ET.fromstring(raw)
    if _strip_ns(root.tag) != "prop.map":
        raise ValueError("not a prop.map: %s" % path)
    for kid in root:
        if _strip_ns(kid.tag) == "prop.list":
            return _propmap_list(kid)
    return {}


# ---------------------------------------------------------------------------
# 3. $$$/ localizable string table, read out of the shipped executable
# ---------------------------------------------------------------------------
_DOLLAR_RE = re.compile(rb"\$\$\$/[\x20-\x7e\xc0-\xff]{2,900}?\x00")
_STR_CACHE = {}


def extract_dollar_strings(binary_path, cache_path=None):
    """{'$$$/Name/Space/Key': 'Default English'} out of a shipped binary.

    The binary is read as data only. Nothing in it is executed or loaded.
    """
    key = os.path.abspath(binary_path)
    if key in _STR_CACHE:
        return _STR_CACHE[key]
    if cache_path and os.path.isfile(cache_path):
        with open(cache_path, "r", encoding="utf-8") as fh:
            data = json.load(fh)
        _STR_CACHE[key] = data
        return data

    with open(binary_path, "rb") as fh:
        blob = fh.read()
    out = {}
    for m in _DOLLAR_RE.finditer(blob):
        raw = m.group(0)[:-1]
        try:
            txt = raw.decode("utf-8")
        except UnicodeDecodeError:
            txt = raw.decode("latin-1")
        if "=" not in txt:
            continue
        k, v = txt.split("=", 1)
        if k in out and out[k] != v:
            continue          # first occurrence wins; duplicates are identical
        out.setdefault(k, v)
    del blob
    if cache_path:
        os.makedirs(os.path.dirname(cache_path), exist_ok=True)
        with open(cache_path, "w", encoding="utf-8") as fh:
            json.dump(out, fh, ensure_ascii=False)
    _STR_CACHE[key] = out
    return out


def premiere_strings(cache_dir=None):
    exe = os.path.join(PREMIERE_ROOT, "Adobe Premiere Pro.exe")
    cache = os.path.join(cache_dir, "ppro_dollar_strings.json") if cache_dir else None
    return extract_dollar_strings(exe, cache)


DOLLAR_INLINE = re.compile(r"^\$\$\$/([^=]*)=(.*)$", re.S)


def split_localized(v):
    """'$$$/Key=Text' -> (key, text); anything else -> (None, v)."""
    if isinstance(v, str):
        m = DOLLAR_INLINE.match(v)
        if m:
            return m.group(1), m.group(2)
    return None, v


def resolve_string(key, table):
    """Look a bare '$$$/..' key up in the extracted table."""
    if not key:
        return None
    if not key.startswith("$$$/"):
        key = "$$$/" + key.lstrip("/")
    return table.get(key)


# ---------------------------------------------------------------------------
# 4. small helpers
# ---------------------------------------------------------------------------
def fourcc(value):
    """1095321158 -> 'AIFF'. Premiere stores exporter file types as fourcc."""
    try:
        n = int(value)
    except (TypeError, ValueError):
        return None
    if n < 0:
        n &= 0xFFFFFFFF
    try:
        b = struct.pack(">I", n & 0xFFFFFFFF)
    except struct.error:
        return None
    txt = "".join(chr(c) if 32 <= c < 127 else "." for c in b)
    return txt


def fourcc_from_hex(hexstr):
    try:
        b = bytes.fromhex(hexstr)
    except ValueError:
        return None
    return "".join(chr(c) if 32 <= c < 127 else "." for c in b)


def ticks_to_seconds(ticks):
    """Premiere stores time as 254016000000 ticks per second."""
    TICKS_PER_SECOND = 254016000000
    try:
        return float(ticks) / TICKS_PER_SECOND
    except (TypeError, ValueError):
        return None


def frame_rate_from_ticks(ticks):
    """<VideoFrameRate>10594584000</VideoFrameRate> -> 23.976023976..."""
    TICKS_PER_SECOND = 254016000000
    try:
        t = float(ticks)
    except (TypeError, ValueError):
        return None
    if t <= 0:
        return None
    return TICKS_PER_SECOND / t


def walk_files(root, exts=None, skip_dirs=()):
    exts = tuple(e.lower() for e in exts) if exts else None
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in skip_dirs]
        for fn in filenames:
            if exts and not fn.lower().endswith(exts):
                continue
            yield os.path.join(dirpath, fn)


def rel(path, root=PREMIERE_ROOT):
    try:
        return os.path.relpath(path, root).replace("\\", "/")
    except ValueError:
        return path.replace("\\", "/")


def envelope(schema_id, method, sources, extra=None):
    """Every output file carries this header."""
    env = {
        "schema_id": schema_id,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "app_launched": APP_LAUNCHED,
        "app_launched_note": ("No Adobe process was started. Every value below "
                              "was read from files on disk with Python's own "
                              "readers. No Adobe DLL was loaded and no COM "
                              "object was created."),
        "target": {
            "product": "Adobe Premiere Pro 2026",
            "install_root": PREMIERE_ROOT,
            "sibling_encoder_root": AME_ROOT,
            "user_data_root": APPDATA_ROOT,
        },
        "method": method,
        "sources": sources,
        "excluded_ai": EXCLUDED_AI,
    }
    if extra:
        env.update(extra)
    return env


def write_json(out_dir, filename, payload):
    os.makedirs(out_dir, exist_ok=True)
    path = os.path.join(out_dir, filename)
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(payload, fh, ensure_ascii=False, indent=1)
    return path, os.path.getsize(path)
