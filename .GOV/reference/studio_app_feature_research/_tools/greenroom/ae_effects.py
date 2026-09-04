"""After Effects 2026 -> aftereffects_effects_catalogue.json

Offline. Reads only. Never launches After Effects.

Evidence chain
--------------
A. Support Files/aelib.dll carries an embedded JSON registry
   {"mPlugins":[{"mEffects":[{mMatchName,mName,mCategory,mEntryPointName,
   mOutFlags,mOutFlags2,mReservedInfo,mSupportURL}],"mFullPath","mGPUEntry"}]}
   -> authoritative registration record for the Adobe-authored effects.
B. Bundled third-party plug-ins (CycoreFX HD, Keylight, mocha) do not appear in
   (A). They carry a classic Windows 'PIPL' resource (type "PIPL", id 16000)
   whose properties ('kind','name','catg','8664','eMNA','eURL', version and
   out-flag words) are parsed directly.
C. Each .aex embeds its localizable strings as $$$/AE/<Plugin>/LStr/NNNN=text
   literals: index 0000 is the About string, pipe-delimited entries are popup
   option lists, the rest are parameter / group / checkbox labels.
D. Support Files/Presets/**/*.ffx presets embed 'pard' parameter definitions
   keyed by "<matchname>-NNNN". These give the real parameter type, valid and
   slider ranges, default, precision and display units.
E. Support Files/PresetEffects.xml declares the pseudo-effects used by presets
   with fully typed parameters.
"""

from __future__ import annotations

import collections
import os
import re
import struct
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402

try:
    import pefile
except ImportError:
    pefile = None


# --------------------------------------------------------------------------
# A. aelib.dll JSON registry
# --------------------------------------------------------------------------

def registry_from_aelib():
    path = os.path.join(C.support_files(), "aelib.dll")
    data = C.read_bytes(path)
    blobs = [b for b in C.extract_json_blobs(data, b'"mPlugins"') if "mPlugins" in b]
    if not blobs:
        return [], None
    reg = max(blobs, key=lambda b: len(b["mPlugins"]))
    out = []
    for pl in reg["mPlugins"]:
        for e in pl.get("mEffects", []):
            out.append({
                "match_name": e.get("mMatchName"),
                "display_name": C.strip_zstring_key(e.get("mName", "")),
                "display_name_key": C.zstring_key_of(e.get("mName", "")),
                "category": C.strip_zstring_key(e.get("mCategory", "")),
                "category_key": C.zstring_key_of(e.get("mCategory", "")),
                "entry_point": e.get("mEntryPointName"),
                "out_flags_raw": e.get("mOutFlags"),
                "out_flags2_raw": e.get("mOutFlags2"),
                "reserved_info": e.get("mReservedInfo"),
                "support_url": e.get("mSupportURL"),
                "plugin_path": pl.get("mFullPath"),
                "gpu_entry_point": pl.get("mGPUEntry") or None,
                "source": "aelib.dll embedded plug-in registry (parsed)",
            })
    return out, C.rel(path)


# --------------------------------------------------------------------------
# A2. MediaCore GPU filter registry (PluginSupport.dll)
# --------------------------------------------------------------------------

def gpu_registry():
    """Support Files/PluginSupport.dll -> {"AEVideoPlugins":{"AEModules":[...]}}

    Each entry carries a PiPLString whose JSON adds fields the aelib registry
    does not have: a plain-English mEffectDescription and mSearchKeywords. The
    match names are host-prefixed, e.g. "AE.ADBE AEASCCDL"; the "AE." prefix is
    the host tag, so the After Effects match name is the remainder.
    """
    import json as _j
    p = os.path.join(C.support_files(), "PluginSupport.dll")
    data = C.read_bytes(p)
    blobs = C.extract_json_nul_delimited(data, b'"AEVideoPlugins"')
    if not blobs:
        return [], C.rel(p)
    reg = max(blobs, key=lambda b: len(str(b)))
    rows = []
    for mod in (reg.get("AEVideoPlugins") or {}).get("AEModules", []) or []:
        for e in mod.get("Effects", []) or []:
            try:
                pipl = _j.loads(e.get("PiPLString") or "{}")
            except Exception:
                pipl = {}
            raw_mn = pipl.get("mMatchName") or e.get("GPUVideoFilter.MatchName") or ""
            host, _, mn = raw_mn.partition(".")
            kw = []
            if pipl.get("mSearchKeywords"):
                try:
                    kwd = _j.loads(pipl["mSearchKeywords"])
                    for lst in kwd.values():
                        kw += [C.strip_zstring_key(x) for x in lst]
                except Exception:
                    pass
            rows.append({
                "host_tag": host,
                "match_name": mn or raw_mn,
                "registered_match_name": raw_mn,
                "display_name": C.strip_zstring_key(pipl.get("mName", "")),
                "display_name_key": C.zstring_key_of(pipl.get("mName", "")),
                "category": C.strip_zstring_key(pipl.get("mCategory", "")),
                "description": pipl.get("mEffectDescription"),
                "search_keywords": kw or None,
                "entry_point": pipl.get("mEntryPointName"),
                "support_url": pipl.get("mSupportURL"),
                "kind_raw": pipl.get("mKind"),
                "version_raw": pipl.get("mVersion"),
                "spec_version_raw": pipl.get("mSpecVersion"),
                "reserved_info": pipl.get("mReservedInfo"),
                "module": mod.get("ModuleName"),
                "gpu_accelerated": True,
                "source": "PluginSupport.dll AEVideoPlugins registry (parsed)",
            })
    return rows, C.rel(p)


# --------------------------------------------------------------------------
# B. PIPL resources
# --------------------------------------------------------------------------

PIPL_KEYS = {
    "kind": "plugin_kind", "name": "display_name", "catg": "category",
    "8664": "entry_point_x64", "8666": "entry_point_arm64",
    "eMNA": "match_name", "eURL": "support_url", "eVER": "version_word",
    "eSVR": "spec_version_word", "ePVR": "plugin_version_word",
    "eINF": "info_word", "eGLO": "global_out_flags",
    "eGL2": "global_out_flags2", "aeFL": "ae_flags",
    "eFRM": "frame_flags", "eGPU": "gpu_word",
}


def parse_pipl(data: bytes):
    """Parse a Windows PIPL resource blob into a dict of properties."""
    if len(data) < 10:
        return None
    # header: u32 reserved, u16 pad, u32 property count
    try:
        count = struct.unpack_from("<I", data, 6)[0]
    except struct.error:
        return None
    if count > 128:
        return None
    props = {}
    off = 10
    for _ in range(count):
        if off + 16 > len(data):
            break
        vendor = data[off:off + 4][::-1].decode("latin-1", "replace")
        key = data[off + 4:off + 8][::-1].decode("latin-1", "replace")
        length = struct.unpack_from("<I", data, off + 12)[0]
        body = data[off + 16:off + 16 + length]
        off += 16 + length + ((4 - length % 4) % 4)
        name = PIPL_KEYS.get(key)
        if name in ("display_name", "match_name", "category", "support_url",
                    "entry_point_x64", "entry_point_arm64"):
            # Pascal string: leading byte is the length. Verified on
            # CycoreFXHD/BallAction.aex where 'name' = 0x0E + "CC Ball Action"
            # and 'catg' = 0x3E + the 62-byte ZString category literal.
            txt = body
            if txt:
                n = txt[0]
                if 0 < n <= len(txt) - 1:
                    txt = txt[1:1 + n]
                else:
                    txt = txt.split(b"\x00", 1)[0]
            props[name] = txt.split(b"\x00", 1)[0].decode("utf-8", "replace")
        elif name == "plugin_kind":
            props[name] = body[:4][::-1].decode("latin-1", "replace")
        elif name:
            props[name] = struct.unpack_from("<I", body, 0)[0] if len(body) >= 4 else None
        else:
            props.setdefault("unmapped", {})[vendor + "/" + key] = body[:32].hex()
    return props


def pipl_scan(paths):
    out = []
    if pefile is None:
        return out, "pefile unavailable: PIPL resources not scanned"
    for p in paths:
        try:
            pe = pefile.PE(p, fast_load=True)
            pe.parse_data_directories(
                directories=[pefile.DIRECTORY_ENTRY["IMAGE_DIRECTORY_ENTRY_RESOURCE"]])
        except Exception:
            continue
        if not hasattr(pe, "DIRECTORY_ENTRY_RESOURCE"):
            pe.close()
            continue
        for e in pe.DIRECTORY_ENTRY_RESOURCE.entries:
            nm = e.name.decode() if e.name else str(e.struct.Id)
            if nm.upper() != "PIPL":
                continue
            for e2 in e.directory.entries:
                for e3 in e2.directory.entries:
                    try:
                        blob = pe.get_data(e3.data.struct.OffsetToData,
                                           e3.data.struct.Size)
                    except Exception:
                        continue
                    props = parse_pipl(blob)
                    if props and props.get("match_name"):
                        props["plugin_file"] = C.rel(p)
                        props["source"] = "PIPL resource (parsed)"
                        out.append(props)
        pe.close()
    return out, None


# --------------------------------------------------------------------------
# C. LStr blocks per plug-in binary
# --------------------------------------------------------------------------

LSTR_RE = re.compile(
    rb"\$\$\$/(?P<ns>AE/[!-<>-~]{1,120})/LStr/(?P<idx>\d{1,5})=(?P<val>[^\x00]{0,1200})")


def lstr_blocks(data: bytes):
    """Return {namespace: {index: text}} for $$$/AE/<ns>/LStr/NNNN= literals."""
    out = collections.defaultdict(dict)
    for m in LSTR_RE.finditer(data):
        ns = m.group("ns").decode("latin-1")
        idx = int(m.group("idx"))
        try:
            val = m.group("val").decode("utf-8")
        except UnicodeDecodeError:
            val = m.group("val").decode("latin-1")
        out[ns][idx] = C.clean_zvalue(val)
    return out


def classify_lstr(block: dict):
    """Split an LStr block into about-text, enum lists and label strings."""
    about = None
    enums = []
    labels = []
    errors = []
    for idx in sorted(block):
        v = block[idx].strip()
        if not v:
            continue
        if idx == 0 and ("#{copy}" in block[idx] or "©" in v or "v%ld" in v):
            about = v
            continue
        if "|" in v and v.count("|") >= 1 and len(v) < 1500:
            opts = v.split("|")
            enums.append({
                "lstr_index": idx,
                "option_count": len(opts),
                "options": [{"index": i + 1,
                             "label": o,
                             "separator": o.strip() in ("(-", "-", "(-)")}
                            for i, o in enumerate(opts)],
            })
            continue
        low = v.lower()
        if (low.startswith(("failed to", "could not", "couldn't", "unable to",
                            "not able to", "error", "invalid", "out of memory"))
                or "cannot allocate" in low or "%s" in v and "error" in low):
            errors.append({"lstr_index": idx, "text": v})
            continue
        labels.append({"lstr_index": idx, "text": v})
    return about, enums, labels, errors


# --------------------------------------------------------------------------
# D. pard harvest from shipped .ffx presets
# --------------------------------------------------------------------------

PARAM_MN_RE = re.compile(r"^(?P<eff>.+?)-(?P<idx>\d{4})$")


def harvest_pard():
    """{match_name: {param_index: record}} from shipped .ffx presets plus the
    shipped Required/secret.aep project (10 MB, 1210 pard records)."""
    by_effect = {}
    stats = {"ffx_files_read": 0, "pard_records": 0, "pdnm_records": 0,
             "aep_files_read": 0}
    presets = os.path.join(C.support_files(), "Presets")
    for p in C.iter_files(presets, (".ffx",)):
        try:
            data = C.read_bytes(p)
        except OSError:
            continue
        stats["ffx_files_read"] += 1
        _harvest_one(data, by_effect, stats, C.rel(p))
    for p in C.iter_files(C.support_files(), (".aep", ".aet")):
        try:
            data = C.read_bytes(p)
        except OSError:
            continue
        stats["aep_files_read"] += 1
        _harvest_one(data, by_effect, stats, C.rel(p))
    return by_effect, stats


# --------------------------------------------------------------------------
# C2. corpus-filtered plain C-string pool (for plug-ins with no ZStrings)
# --------------------------------------------------------------------------

# NUL-preceded as well as NUL-terminated: without the lookbehind the scan
# also yields the printable tail of a binary blob, which produced truncated
# labels such as "xis|Center X|..." instead of "Axis|Center X|...".
CSTR_RE = re.compile(rb"(?<=\x00)[\x20-\x7e]{2,240}\x00")

NOISE_SUBSTR = (
    "Suite", "suite", "://", ".cpp", ".h\x00", "D:\\releases", "%s", "%d",
    "%ld", "%f", "operator", "std::", "dvacore", "::", "Could not", "could not",
    "Failed", "failed", "Unable", "cannot", "Cannot", "error", "Error",
    "ERROR", "assert", "exception", "Exception", "allocat", "nullptr",
    "bad ", "vector", "string too long", "invalid", "Invalid", "@",
)


def build_cstring_corpus(paths):
    """Document frequency of every printable C string across all plug-ins.

    Strings that occur in many different plug-in binaries are SDK/CRT
    boilerplate (AEGP suite names, error text, RTTI). Strings that occur in
    only one or two binaries are that plug-in's own vocabulary, which is where
    its parameter labels live.
    """
    df = collections.Counter()
    per_file = {}
    for p in paths:
        try:
            data = C.read_bytes(p)
        except OSError:
            continue
        seen = []
        seen_set = set()
        for m in CSTR_RE.finditer(data):
            s = m.group()[:-1].decode("latin-1")
            if s in seen_set:
                continue
            seen_set.add(s)
            seen.append((m.start(), s))
        per_file[p] = seen
        df.update(seen_set)
    return df, per_file


# A UI label starts with a capital (or a leading digit, as in "3D Position"),
# is mostly letters, and uses only punctuation that appears in Adobe UI text.
LABEL_SHAPE = re.compile(r"^(?:[A-Z]|[0-9](?=[Dd]\b)|[0-9]+(?= ))"
                         r"[A-Za-z0-9 ()\-&/.,'%+#:*]{2,47}$")


def looks_like_label(s: str) -> bool:
    """Reject binary noise that happens to be printable.

    The document-frequency filter removes SDK/CRT boilerplate; this shape
    filter removes .rdata fragments such as 'FW', '$pl' or '^5Yy' that are
    printable by accident. Both filters are conservative: a rejected string is
    simply not reported as a label.
    """
    if len(s) < 3 or len(s) > 50:
        return False
    if any(n in s for n in NOISE_SUBSTR):
        return False
    if not LABEL_SHAPE.match(s):
        return False
    letters = sum(ch.isalpha() for ch in s)
    if letters < 3 or letters / float(len(s)) < 0.55:
        return False
    if re.match(r"^[A-Z0-9_]{4,}$", s):    # SCREAMING_CASE constants
        return False
    if s.count(" ") > 6:
        return False
    return True


def string_pool_for(path, per_file, df, df_cutoff):
    entries = per_file.get(path, [])
    labels, enums = [], []
    for off, s in entries:
        if "|" in s and s.count("|") >= 1 and len(s) <= 400 and df[s] <= df_cutoff:
            opts = s.split("|")
            plausible = [o for o in opts
                         if looks_like_label(o) or o.strip() in ("(-", "-", "")]
            if (all(len(o) <= 48 for o in opts)
                    and len(plausible) >= max(2, int(0.8 * len(opts)))):
                enums.append({"file_offset": hex(off), "option_count": len(opts),
                              "options": [{"index": i + 1, "label": o,
                                           "separator": o.strip() in ("(-", "-")}
                                          for i, o in enumerate(opts)]})
                continue
        if df[s] <= df_cutoff and looks_like_label(s):
            labels.append({"file_offset": hex(off), "text": s})
    return labels, enums


def _harvest_one(data, by_effect, stats, src):
    last_mn = None
    pending = None

    def walk(chunks):
        nonlocal last_mn, pending
        for c in chunks:
            if c.children:
                walk(c.children)
                continue
            if c.cid == b"tdmn":
                last_mn = C.cstr(c.data)
            elif c.cid == b"pard":
                stats["pard_records"] += 1
                pending = (last_mn, C.decode_pard(c.data))
                _store(pending, by_effect, src)
            elif c.cid == b"pdnm" and pending:
                stats["pdnm_records"] += 1
                txt = C.utf8_chunk(c.data) if c.data[:4] == b"Utf8" else C.cstr(c.data)
                _attach_pdnm(pending, txt, by_effect)
    walk(C.rifx_parse(data))


def _store(pending, by_effect, src):
    mn, rec = pending
    if not mn:
        return
    m = PARAM_MN_RE.match(mn)
    if not m:
        return
    eff = m.group("eff")
    idx = int(m.group("idx"))
    slot = by_effect.setdefault(eff, {})
    if idx in slot:
        # keep the record that carries the richest decode
        old = slot[idx]["record"]
        if len(str(rec)) <= len(str(old)):
            return
    rec = dict(rec)
    # the concrete value found in this particular preset is preset data,
    # not part of the effect definition; keep it out of the catalogue
    rec.pop("value_in_preset", None)
    rec.pop("value_in_preset_argb", None)
    slot[idx] = {"param_index": idx, "record": rec, "first_seen_in": src}


def _attach_pdnm(pending, txt, by_effect):
    mn, _ = pending
    if not mn or not txt:
        return
    m = PARAM_MN_RE.match(mn)
    if not m:
        return
    slot = by_effect.get(m.group("eff"), {}).get(int(m.group("idx")))
    if slot is None:
        return
    rec = slot["record"]
    if rec.get("param_type") == "POPUP":
        opts = txt.split("|")
        rec["options"] = [{"index": i + 1, "label": o,
                           "separator": o.strip() in ("(-", "-", "(-)")}
                          for i, o in enumerate(opts)]
        rec["options_source"] = "pdnm chunk (parsed)"
    elif rec.get("param_type") == "CHECKBOX":
        rec["checkbox_label"] = txt
    elif rec.get("param_type") == "BUTTON":
        rec["button_label"] = txt
    elif not rec.get("name"):
        rec["name"] = txt


# --------------------------------------------------------------------------
# E. PresetEffects.xml
# --------------------------------------------------------------------------

def parse_preset_effects_xml():
    path = os.path.join(C.support_files(), "PresetEffects.xml")
    raw = C.read_bytes(path).decode("utf-8", "replace")
    # strip the inline DTD: ElementTree cannot handle the ATTLIST block
    body = raw[raw.index("<Effects>"):]
    root = ET.fromstring(body)
    out = []
    for eff in root.findall("Effect"):
        out.append({
            "match_name": eff.get("matchname"),
            "display_name": C.strip_zstring_key(eff.get("name", "")),
            "display_name_key": C.zstring_key_of(eff.get("name", "")),
            "external_id": eff.get("external_id"),
            "parameters": _xml_params(eff),
            "source": "PresetEffects.xml (parsed)",
        })
    return out, C.rel(path)


def _xml_params(node, depth=0):
    params = []
    for ch in node:
        tag = ch.tag
        nm = C.strip_zstring_key(ch.get("name", ""))
        base = {"name": nm, "name_key": C.zstring_key_of(ch.get("name", "")),
                "param_type": tag.upper(), "depth": depth}
        if ch.get("external_id"):
            base["external_id"] = ch.get("external_id")
        if ch.get("CANNOT_TIME_VARY") == "true":
            base["cannot_time_vary"] = True
        if ch.get("INVISIBLE") == "true":
            base["invisible"] = True
        if tag == "Group":
            base["children"] = _xml_params(ch, depth + 1)
        elif tag in ("Slider",):
            for k, o in (("default", "default"), ("valid_min", "valid_min"),
                         ("valid_max", "valid_max"), ("slider_min", "slider_min"),
                         ("slider_max", "slider_max"), ("precision", "precision")):
                if ch.get(k) is not None:
                    base[o] = _num(ch.get(k))
            units = []
            if ch.get("DISPLAY_PERCENT") == "true":
                units.append("percent")
            if ch.get("DISPLAY_PIXEL") == "true":
                units.append("pixel")
            if units:
                base["display_units"] = units
        elif tag == "Angle":
            base["default"] = _num(ch.get("default"))
            base["units"] = "degrees"
        elif tag == "Checkbox":
            base["default"] = ch.get("default") == "true"
        elif tag in ("Popup", "Popup_UTF8"):
            ps = ch.get("popup_string", "")
            opts = ps.split("|")
            base["option_count"] = len(opts)
            base["options"] = [{"index": i + 1, "label": C.strip_zstring_key(o)}
                               for i, o in enumerate(opts)]
            base["default_index"] = _num(ch.get("default"))
        elif tag == "Color":
            base["default_rgb_0_255"] = [_num(ch.get("default_red")),
                                         _num(ch.get("default_green")),
                                         _num(ch.get("default_blue"))]
        elif tag == "Layer":
            base["default_self"] = ch.get("default_self", "true") == "true"
        elif tag in ("Point", "Point3D"):
            base["default_x_fraction_of_layer"] = _num(ch.get("default_x", "0.5"))
            base["default_y_fraction_of_layer"] = _num(ch.get("default_y", "0.5"))
            if tag == "Point3D":
                base["default_z_fraction_of_layer"] = _num(ch.get("default_z", "0.5"))
        params.append(base)
    return params


def _num(v):
    if v is None:
        return None
    try:
        f = float(v)
        return int(f) if f == int(f) else f
    except ValueError:
        return v


# --------------------------------------------------------------------------
# excluded_ai
# --------------------------------------------------------------------------

AI_MATCH_TOKENS = [
    ("roto brush", "Roto Brush / Roto Brush 2 (ML segmentation)"),
    ("refine matte", "Refine Matte / Refine Edge (ML matte refinement)"),
    ("refine soft matte", "Refine Soft Matte"),
    ("refine hard matte", "Refine Hard Matte"),
    ("content aware", "Content-Aware Fill"),
    ("contentaware", "Content-Aware Fill"),
    ("scene edit", "Scene Edit Detection"),
    ("body track", "Body Tracker"),
    ("face track", "Face Tracking"),
    ("sensei", "Adobe Sensei surface"),
    ("firefly", "Adobe Firefly surface"),
    ("rotobrush", "Roto Brush"),
    ("autoreframe", "Auto Reframe (Sensei)"),
    ("auto reframe", "Auto Reframe (Sensei)"),
    ("depth from", "ML depth estimation"),
    ("upscale", "ML/Sensei upscale candidate - verify"),
]


def build_excluded_ai(all_effects):
    hits = []
    for e in all_effects:
        blob = " ".join(str(e.get(k, "")) for k in
                        ("match_name", "display_name", "plugin_path", "plugin_file")).lower()
        for tok, why in AI_MATCH_TOKENS:
            if tok in blob:
                hits.append({
                    "match_name": e.get("match_name"),
                    "display_name": e.get("display_name"),
                    "category": e.get("category"),
                    "reason": why,
                    "evidence_path": e.get("plugin_path") or e.get("plugin_file"),
                })
                break
    sf = C.support_files()
    assets = []
    for relpath, why in [
        ("Support Files/MLModels", "On-disk ML model store"),
        ("Support Files/MLModels/model_metadata.json",
         "ML model manifest: model ids, ONNX/CoreML variants, tensor shapes"),
        ("Support Files/MLModels/FastMask", "FastMask segmentation model payload"),
        ("Support Files/MLModels/ShotCutDetection",
         "Shot-cut/Scene Edit Detection model payload"),
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
    ]:
        full = os.path.join(C.install_root(), relpath.replace("/", os.sep))
        if os.path.exists(full):
            assets.append({"path": relpath, "exists": True, "role": why})
    models = []
    meta = os.path.join(sf, "MLModels", "model_metadata.json")
    if os.path.exists(meta):
        import json as _j
        try:
            md = _j.loads(C.read_bytes(meta).decode("utf-8", "replace"))
            for k, v in md.items():
                models.append({
                    "model_id": k,
                    "file_name": v.get("file_name"),
                    "module": C.strip_zstring_key(v.get("module_display_name", "")),
                    "formats": [f.get("format") for f in v.get("available_formats", [])],
                })
        except Exception as exc:
            models.append({"parse_error": str(exc)})
    return {
        "policy": C.EXCLUDED_AI_NOTE,
        "excluded_effect_surfaces": hits,
        "excluded_effect_surface_count": len(hits),
        "on_disk_ai_assets": assets,
        "ml_models_declared": models,
        "ml_model_count": len(models),
        "note_upscale": ("'ADBE Upscale' is listed above only because its name "
                         "matched an AI token; its on-disk registration carries "
                         "no ML model reference. It is reported for audit, and "
                         "is NOT treated as an AI feature elsewhere in this "
                         "export."),
    }


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------


def main():
    sf = C.support_files()
    reg, reg_path = registry_from_aelib()

    aex_paths = sorted(C.iter_files(sf, (".aex",)))
    aex_by_norm = {}
    for p in aex_paths:
        aex_by_norm.setdefault(
            _norm(os.path.splitext(os.path.basename(p))[0]), p)

    pipls, pipl_err = pipl_scan(aex_paths)
    gpu_rows, gpu_src = gpu_registry()
    gpu_by_mn = {}
    for g in gpu_rows:
        gpu_by_mn.setdefault(g["match_name"], g)
    pard, pard_stats = harvest_pard()
    xml_effects, xml_path = parse_preset_effects_xml()

    # ---- string evidence per binary -------------------------------------
    lstr_by_path = {}
    for p in aex_paths:
        blocks = lstr_blocks(C.read_bytes(p))
        if blocks:
            lstr_by_path[p] = blocks
    df, per_file = build_cstring_corpus(aex_paths)
    df_cutoff = max(2, int(len(aex_paths) * 0.12))

    # ---- assemble effect records ----------------------------------------
    effects = {}
    for e in reg:
        rec = dict(e)
        binp = aex_by_norm.get(_norm(os.path.basename(e["plugin_path"] or "")))
        rec["plugin_file_on_disk"] = C.rel(binp) if binp else None
        rec["shipped_in_this_install"] = bool(binp)
        effects[e["match_name"]] = rec

    pipl_only = 0
    for p in pipls:
        mn = p["match_name"]
        if mn in effects:
            effects[mn]["pipl"] = {k: v for k, v in p.items() if k != "match_name"}
            if not effects[mn].get("plugin_file_on_disk"):
                effects[mn]["plugin_file_on_disk"] = p["plugin_file"]
                effects[mn]["shipped_in_this_install"] = True
            continue
        pipl_only += 1
        effects[mn] = {
            "match_name": mn,
            "display_name": p.get("display_name"),
            "category": C.strip_zstring_key(p.get("category", "")),
            "category_key": C.zstring_key_of(p.get("category", "")),
            "entry_point": p.get("entry_point_x64"),
            "support_url": p.get("support_url"),
            "plugin_path": os.path.dirname(p["plugin_file"]),
            "plugin_file_on_disk": p["plugin_file"],
            "shipped_in_this_install": True,
            "out_flags_raw": p.get("global_out_flags"),
            "out_flags2_raw": p.get("global_out_flags2"),
            "plugin_kind": p.get("plugin_kind"),
            "pipl": {k: v for k, v in p.items() if k != "match_name"},
            "source": "PIPL resource (parsed)",
        }

    # MediaCore GPU registrations: attach to the matching effect, and keep the
    # ones that name no aelib-registered effect as their own entries.
    gpu_attached = 0
    gpu_only = 0
    for mn, g in gpu_by_mn.items():
        if mn in effects:
            gpu_attached += 1
            effects[mn]["gpu_accelerated"] = True
            effects[mn]["mediacore_gpu_registration"] = g
            if g.get("description") and not effects[mn].get("description"):
                effects[mn]["description"] = g["description"]
            if g.get("search_keywords"):
                effects[mn]["search_keywords"] = g["search_keywords"]
        else:
            gpu_only += 1
            # This registry is a build-time cache and lists MediaCore filters
            # that may not be installed here. Only claim it ships if the match
            # name actually occurs as a string in a shipped .aex.
            on_disk = mn in df
            effects[mn] = {
                "match_name": mn,
                "display_name": g.get("display_name"),
                "display_name_key": g.get("display_name_key"),
                "category": g.get("category"),
                "description": g.get("description"),
                "search_keywords": g.get("search_keywords"),
                "entry_point": g.get("entry_point"),
                "support_url": g.get("support_url"),
                "plugin_path": g.get("module"),
                "shipped_in_this_install": on_disk,
                "registration_only": not on_disk,
                "registration_only_note": (None if on_disk else
                    "Present in the PluginSupport.dll MediaCore filter registry "
                    "but its match name does not occur in any shipped .aex, so "
                    "the plug-in is NOT installed here. Reported for "
                    "completeness, not claimed as shipped."),
                "gpu_accelerated": True,
                "mediacore_gpu_registration": g,
                "source": "PluginSupport.dll AEVideoPlugins registry (parsed)",
            }

    per_binary = collections.Counter(
        v.get("plugin_file_on_disk") for v in effects.values()
        if v.get("plugin_file_on_disk"))

    param_from_records = 0
    with_lstr = 0
    with_pool = 0
    validation = {"effects_checked": 0, "params_checked": 0,
                  "params_found_in_strings": 0}

    for mn, ent in effects.items():
        slots = pard.get(mn)
        if slots:
            ordered = [slots[i] for i in sorted(slots)]
            ent["parameters"] = [dict(param_index=s["param_index"], **s["record"])
                                 for s in ordered]
            ent["parameters_source"] = (
                "parsed: pard parameter-definition records (+pdnm option "
                "strings) recovered from shipped .ffx presets and secret.aep, "
                "keyed by the effect's own parameter indices")
            ent["parameter_count_recovered"] = len(ordered)
            ent["parameter_indices_recovered"] = sorted(slots)
            ent["parameter_record_completeness"] = (
                "contiguous_from_zero"
                if sorted(slots) == list(range(min(slots), max(slots) + 1))
                and min(slots) == 0 else "partial")
            param_from_records += 1

        binp = None
        if ent.get("plugin_file_on_disk"):
            binp = os.path.join(C.install_root(),
                                ent["plugin_file_on_disk"].replace("/", os.sep))
        strings = {}
        if binp and binp in lstr_by_path:
            blocks = lstr_by_path[binp]
            ns = _pick_namespace(blocks, ent)
            if ns:
                about, enums, labels, errors = classify_lstr(blocks[ns])
                strings = {
                    "kind": "zstring_lstr_table",
                    "evidence_file": C.rel(binp),
                    "zstring_namespace": "$$$/%s/LStr/" % ns,
                    "about_text": about,
                    "enumerations": enums,
                    "label_pool": labels,
                    "error_strings": errors,
                }
                with_lstr += 1
        if not strings and binp and binp in per_file:
            labels, enums = string_pool_for(binp, per_file, df, df_cutoff)
            if labels or enums:
                strings = {
                    "kind": "plain_cstring_pool",
                    "evidence_file": C.rel(binp),
                    "enumerations": enums,
                    "label_pool": labels,
                }
                with_pool += 1
        if strings:
            strings["effects_sharing_this_binary"] = per_binary.get(
                ent.get("plugin_file_on_disk"), 1)
            strings["ordering_label"] = "HEURISTIC"
            strings["note"] = (
                "label_pool is the plug-in's own vocabulary in on-disk order. "
                "For a ZString table that order is the plug-in's declaration "
                "order, so it tracks parameter order closely; for a plain "
                "C-string pool it is .rdata layout order. Neither proves the "
                "label-to-parameter binding. Where 'parameters' is present, "
                "that parsed record is authoritative and the pool is only "
                "corroboration.")
            ent["strings"] = strings

        if strings and ent.get("parameters"):
            pool = set(l["text"] for l in strings.get("label_pool", []))
            for en in strings.get("enumerations", []):
                for o in en["options"]:
                    pool.add(o["label"])
            named = [p for p in ent["parameters"]
                     if p.get("name") and p["param_type"] not in
                     ("GROUP_END", "LAYER", "NO_DATA")]
            if named:
                validation["effects_checked"] += 1
                validation["params_checked"] += len(named)
                validation["params_found_in_strings"] += sum(
                    1 for p in named if p["name"] in pool)

    xml_added = 0
    for x in xml_effects:
        mn = x["match_name"]
        if mn in effects:
            effects[mn]["preset_effects_xml_parameters"] = x["parameters"]
        else:
            xml_added += 1
            effects[mn] = {
                "match_name": mn,
                "display_name": x["display_name"],
                "display_name_key": x.get("display_name_key"),
                "category": "Pseudo-effect (preset-only)",
                "plugin_path": xml_path,
                "parameters": x["parameters"],
                "parameters_source": "parsed: PresetEffects.xml, fully typed",
                "parameter_count_recovered": len(x["parameters"]),
                "source": "PresetEffects.xml (parsed)",
                "pseudo_effect": True,
            }

    excluded = build_excluded_ai(list(effects.values()))
    excluded_names = set(
        h["match_name"] for h in excluded["excluded_effect_surfaces"]
        if h["match_name"] and "Upscale" not in (h["match_name"] or ""))
    kept = {k: v for k, v in effects.items() if k not in excluded_names}

    by_cat = collections.Counter(v.get("category") or "(uncategorised)"
                                 for v in kept.values())
    with_params = sum(1 for v in kept.values() if v.get("parameters"))
    total_params = sum(len(v.get("parameters", [])) for v in kept.values())
    with_any_strings = sum(1 for v in kept.values() if v.get("strings"))
    enum_total = sum(len(v.get("strings", {}).get("enumerations", []))
                     for v in kept.values())
    shipped = sum(1 for v in kept.values() if v.get("shipped_in_this_install"))

    rate = (100.0 * validation["params_found_in_strings"] /
            validation["params_checked"]) if validation["params_checked"] else None
    validation["string_pool_recall_percent"] = round(rate, 1) if rate else None
    validation["interpretation"] = (
        "Of parameters whose type/range/default were parsed from pard records, "
        "this percentage also appear verbatim in the same plug-in's string "
        "evidence. It measures how much of an effect's parameter vocabulary the "
        "string pool captures for the effects where no pard record exists.")

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_effects.py",
        "evidence": [
            {"id": "A", "path": reg_path, "label": "parsed",
             "what": "embedded JSON plug-in registry with mPlugins/mEffects "
                     "giving mMatchName, mName, mCategory, mEntryPointName, "
                     "mOutFlags, mOutFlags2, mReservedInfo, mSupportURL",
             "extraction": "brace-matched JSON located by the mPlugins anchor "
                           "in the PE image, then json.loads"},
            {"id": "A2", "path": gpu_src, "label": "parsed",
             "what": "AEVideoPlugins registry: one PiPLString per GPU-capable "
                     "MediaCore filter, adding a plain-English "
                     "mEffectDescription and mSearchKeywords that the aelib "
                     "registry does not carry. Match names are host-prefixed "
                     "(\"AE.<match name>\"); the AE. tag is the host, so the "
                     "After Effects match name is the remainder.",
             "extraction": "NUL-delimited JSON widen + json.loads, then "
                           "json.loads of each nested PiPLString"},
            {"id": "B", "path": "Support Files/Plug-ins/**/*.aex", "label": "parsed",
             "what": "classic Windows PIPL resource (type PIPL, id 16000) on "
                     "bundled third-party plug-ins (CycoreFX HD, Keylight, mocha)",
             "extraction": "pefile resource walk; property records are "
                           "vendor(4)+key(4)+id(4)+len(4)+Pascal-string data, "
                           "4-byte padded, 4CC keys byte-reversed on Windows"},
            {"id": "C", "path": "Support Files/**/*.aex",
             "label": "parsed strings, HEURISTIC label-to-parameter binding",
             "what": "$$$/AE/<Plugin>/LStr/NNNN= tables; index 0000 is the About "
                     "string, pipe-delimited values are PF_ADD_POPUP option lists",
             "extraction": "regex over the raw image, grouped by namespace"},
            {"id": "C2", "path": "Support Files/**/*.aex",
             "label": "parsed strings, HEURISTIC label-to-parameter binding",
             "what": "plain NUL-terminated C-string pool for plug-ins that ship "
                     "no ZStrings (all of CycoreFX HD)",
             "extraction": ("document-frequency filter across all %d .aex files: "
                            "a string present in more than %d binaries is SDK/CRT "
                            "boilerplate and is dropped; the remainder is the "
                            "plug-in's own vocabulary, kept in .rdata order"
                            % (len(aex_paths), df_cutoff))},
            {"id": "D", "path": "Support Files/Presets/**/*.ffx + "
                                "Support Files/Required/secret.aep",
             "label": "parsed",
             "what": "pard parameter definitions and pdnm popup strings",
             "extraction": "big-endian RIFX walk. pard field offsets were "
                           "recovered by differencing and validated against "
                           "independently checkable on-disk facts: popup option "
                           "counts must equal the option count of the adjacent "
                           "pdnm string (Fractal Noise Fractal Type = 20 options "
                           "default 1, Noise Type = 4 default 3, Blending Mode = "
                           "21 default 2), and the slider blocks reproduce Levels "
                           "(Gamma 0..5 default 1, Input White default 1) and "
                           "Gaussian Blur (Blurriness valid 0..30000, slider "
                           "0..50, default 0)"},
            {"id": "E", "path": xml_path, "label": "parsed",
             "what": "pseudo-effect declarations with fully typed parameters",
             "extraction": "inline DTD stripped, then ElementTree"},
        ],
        "counts": {
            "aex_files_on_disk": len(aex_paths),
            "effects_in_aelib_registry": len(reg),
            "effects_whose_plugin_binary_is_present_in_this_install": shipped,
            "pipl_resources_parsed": len(pipls),
            "effects_only_found_via_pipl": pipl_only,
            "pseudo_effects_from_preset_effects_xml": xml_added,
            "ffx_files_read": pard_stats["ffx_files_read"],
            "aep_files_read": pard_stats["aep_files_read"],
            "pard_records_decoded": pard_stats["pard_records"],
            "pdnm_option_strings_decoded": pard_stats["pdnm_records"],
            "gpu_registrations_read": len(gpu_rows),
            "gpu_registrations_attached_to_a_registered_effect": gpu_attached,
            "effects_added_only_by_the_gpu_registry": gpu_only,
            "gpu_registry_entries_not_installed_here": sum(
                1 for v in effects.values() if v.get("registration_only")),
            "effects_with_zstring_table": with_lstr,
            "effects_with_plain_cstring_pool": with_pool,
        },
        "validation": validation,
        "failures_and_limits": [
            "PF_OutFlags / PF_OutFlags2 bit semantics are NOT decoded: no SDK "
            "header ships on disk, so out_flags_raw / out_flags2_raw are emitted "
            "as raw integers rather than guessed bit names.",
            "Typed parameter records (type, valid/slider range, default, "
            "precision, display units) exist only for effects that appear in a "
            "shipped .ffx preset, in secret.aep, or in PresetEffects.xml. For "
            "the rest, the recoverable evidence is the label pool and popup "
            "option lists, and ordering is labelled HEURISTIC.",
            ("%d registry entries name a plug-in module that is not present in "
             "this install (SDK_Tester, GPUTest, BodyTracker and similar). They "
             "are kept with shipped_in_this_install=false rather than silently "
             "dropped." % (len(reg) - shipped)),
            "Where one binary hosts several effects the string pool is shared "
            "and cannot be split per effect; strings.effects_sharing_this_binary "
            "reports that ambiguity.",
        ],
    }
    if pipl_err:
        method["failures_and_limits"].append(pipl_err)

    payload = {
        "summary": {
            "effects_total": len(kept),
            "effects_that_exist_in_this_install": len(kept) - sum(
                1 for v in kept.values() if v.get("registration_only")),
            "entries_that_are_registration_only_not_installed": sum(
                1 for v in kept.values() if v.get("registration_only")),
            "preset_only_pseudo_effects": sum(
                1 for v in kept.values() if v.get("pseudo_effect")),
            "effects_shipped_as_plugin_binary": shipped,
            "effects_with_typed_parameter_records": with_params,
            "typed_parameter_records_total": total_params,
            "effects_with_string_evidence": with_any_strings,
            "effects_with_a_plain_english_description": sum(
                1 for v in kept.values() if v.get("description")),
            "gpu_accelerated_effects": sum(
                1 for v in kept.values() if v.get("gpu_accelerated")),
            "enumerated_option_lists_recovered": enum_total,
            "effects_by_category": dict(sorted(by_cat.items())),
            "ai_surfaces_excluded": len(excluded_names),
        },
        "param_type_enum": C.PARAM_TYPES,
        "param_flag_bits": dict((hex(m), n) for m, n in C.PARAM_FLAG_BITS),
        "display_flag_bits": dict((hex(m), n) for m, n in C.DISPLAY_FLAG_BITS),
        "value_kind_notes": {
            "fixed_16_16": "on-disk 32-bit fixed point, divided by 65536 here",
            "enum_index_1_based": "popup indices are 1-based; '(-' is a separator",
            "point_percent_of_layer": "point defaults are percentages of layer size",
            "argb8": "colour defaults are 8-bit ARGB",
        },
        "effects": [kept[k] for k in sorted(kept)],
    }
    C.write_json("aftereffects_effects_catalogue.json",
                 "handshake.studio.teardown.aftereffects.effects_catalogue",
                 method, payload, excluded_ai=excluded)
    print("effects=%d shipped=%d typed_param_effects=%d typed_params=%d "
          "string_evidence=%d enums=%d recall=%s"
          % (len(kept), shipped, with_params, total_params, with_any_strings,
             enum_total, validation["string_pool_recall_percent"]),
          file=sys.stderr)


def _pick_namespace(blocks, ent):
    """Choose the LStr namespace belonging to this effect."""
    if len(blocks) == 1:
        return next(iter(blocks))
    want = _norm(ent.get("display_name") or "")
    best = None
    for ns in blocks:
        leaf = _norm(ns.split("/")[-1])
        if leaf == want:
            return ns
        if best is None or len(blocks[ns]) > len(blocks[best]):
            best = ns
    return best


def _norm(s):
    return re.sub(r"[^a-z0-9]", "", (s or "").lower())


if __name__ == "__main__":
    main()
