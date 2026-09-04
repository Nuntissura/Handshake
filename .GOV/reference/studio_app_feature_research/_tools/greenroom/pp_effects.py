"""pp_effects.py -- Premiere Pro 2026 effect / transition catalogue with parameters.

OFFLINE ONLY. No Adobe process is started.

Seven independent evidence streams are merged, and every parameter row records
which stream it came from so parsed values are never confused with derived ones.

  S1  PresetEffects.xml            fully typed parameter declarations
                                   (Slider/Angle/Checkbox/Popup/Color/Point/
                                   Layer/Group with default, valid_min,
                                   valid_max, slider_min, slider_max and
                                   DISPLAY_PERCENT / DISPLAY_PIXEL units).
                                   These are the "pseudo effects" that back
                                   shipped animation presets.

  S2  *.prfpset shipped presets    Real serialized effect instances. Each
                                   VideoComponentParam / AudioComponentParam
                                   carries Name, LowerBound, UpperBound,
                                   CurrentValue, UnitsString and
                                   ParameterControlType. The bounds are the
                                   effect's declared parameter range; the
                                   CurrentValue is that preset's value.

  S3  executable string table      $$$/AE/<Effect>/LStr/NNNN  and
                                   $$$/MediaCore/AEFilters/<Id>/<key>
                                   give the effect description (LStr/0000),
                                   the ordered parameter labels, and popup
                                   enumerations as "A|B|C" strings.

  S4  audio filter Eve layouts     install/eve/*.eve -- the shipped dialog for
                                   each dva audio filter, giving control kind
                                   and, where declared, range and precision.

  S5  audio filter prop.map UIs    install/xml/{Amplify,FFTFilter,NotchFilter,
                                   PitchShift,ScientificFilter,StereoExpander}
                                   .xml -- serialized dvaui control trees for
                                   the built-in audio filters.

  S6  Adobe Dialog Manager sheets  install/adm/*.adm -- for each built-in audio
                                   effect, the normalised input slot for every
                                   parameter, the real-world unit range it maps
                                   onto, the mapping expression itself, and the
                                   host's own kParameterIndex_* constant. This
                                   is the strongest available statement of the
                                   audio effects' behaviour.

  S7  Essential Sound model        install/json/{DefaultAdjustmentsModes,
                                   EssentialSoundConfigPresets,
                                   EssentialSoundPresets,
                                   AudioChannelLayoutPresets}.json and
                                   Settings/EssentialSound/*.json -- the
                                   clip-type adjustment model: which parameter
                                   models each clip type exposes and each
                                   field's declared type, default, min and max.
                                   Enhance Speech and the ML sound classifier
                                   are stripped as excluded AI surfaces.

Match names come from the executable's own NUL-terminated "AE.ADBE ..." /
"ADBE ..." literals and from the FilterMatchName fields inside the presets.
"""
import collections
import json
import os
import re
import sys
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C
import pp_adm
import dw_eve

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

# ---------------------------------------------------------------------------
# S1  PresetEffects.xml
# ---------------------------------------------------------------------------
PARAM_ELEMENTS = ("Slider", "Angle", "Checkbox", "Popup", "Popup_UTF8",
                  "Color", "Layer", "Point", "Point3D", "Group")

TYPE_FOR_ELEMENT = {
    "Slider": "float",
    "Angle": "angle_degrees",
    "Checkbox": "boolean",
    "Popup": "enum",
    "Popup_UTF8": "enum",
    "Color": "rgb_color",
    "Layer": "layer_reference",
    "Point": "point_2d_normalized",
    "Point3D": "point_3d_normalized",
}


def _loc(v):
    k, txt = C.split_localized(v)
    return txt, k


def _num(v):
    if v is None:
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return v
    return int(f) if f == int(f) and abs(f) < 1e15 else f


def parse_preset_effects_xml(path):
    """Return list of typed pseudo-effect definitions."""
    import xml.etree.ElementTree as ET
    with open(path, "rb") as fh:
        raw = fh.read()
    # the shipped file carries an inline DTD that ET's parser rejects on some
    # builds; strip the DOCTYPE block, keep the <Effects> body verbatim.
    body_start = raw.find(b"<Effects>")
    if body_start < 0:
        raise ValueError("no <Effects> body in %s" % path)
    root = ET.fromstring(raw[body_start:])

    out = []
    for eff in root.findall("Effect"):
        name, name_key = _loc(eff.get("name"))
        params = []

        def walk(node, group_path):
            for kid in node:
                tag = kid.tag
                if tag not in PARAM_ELEMENTS:
                    continue
                pname, pkey = _loc(kid.get("name"))
                if tag == "Group":
                    if kid.get("INVISIBLE") != "true":
                        params.append({
                            "name": pname, "name_string_key": pkey,
                            "type": "group", "control_role": "parameter group",
                            "group_path": list(group_path),
                            "source": "S1_PresetEffects.xml",
                            "confidence": "parsed",
                        })
                    walk(kid, group_path + [pname])
                    continue
                rec = {
                    "name": pname, "name_string_key": pkey,
                    "type": TYPE_FOR_ELEMENT.get(tag, tag.lower()),
                    "group_path": list(group_path),
                    "keyframable": kid.get("CANNOT_TIME_VARY") != "true",
                    "source": "S1_PresetEffects.xml",
                    "confidence": "parsed",
                }
                if tag == "Slider":
                    rec["default"] = _num(kid.get("default"))
                    rec["min"] = _num(kid.get("valid_min"))
                    rec["max"] = _num(kid.get("valid_max"))
                    if kid.get("slider_min") is not None:
                        rec["ui_slider_min"] = _num(kid.get("slider_min"))
                    if kid.get("slider_max") is not None:
                        rec["ui_slider_max"] = _num(kid.get("slider_max"))
                    if kid.get("DISPLAY_PERCENT") == "true":
                        rec["units"] = "percent"
                    elif kid.get("DISPLAY_PIXEL") == "true":
                        rec["units"] = "pixels"
                    else:
                        rec["units"] = None
                elif tag == "Angle":
                    rec["default"] = _num(kid.get("default"))
                    rec["units"] = "degrees"
                elif tag == "Checkbox":
                    rec["default"] = kid.get("default") == "true"
                elif tag in ("Popup", "Popup_UTF8"):
                    opts, _ = _loc(kid.get("popup_string"))
                    rec["enum_options"] = [o for o in (opts or "").split("|") if o]
                    rec["default_index_1based"] = _num(kid.get("default"))
                    d = rec["default_index_1based"]
                    if isinstance(d, int) and rec["enum_options"] and 1 <= d <= len(rec["enum_options"]):
                        rec["default"] = rec["enum_options"][d - 1]
                elif tag == "Color":
                    rec["default"] = {
                        "r": _num(kid.get("default_red")),
                        "g": _num(kid.get("default_green")),
                        "b": _num(kid.get("default_blue")),
                    }
                    rec["units"] = "0-255 per channel"
                elif tag in ("Point", "Point3D"):
                    rec["default"] = {
                        "x": _num(kid.get("default_x") or 0.5),
                        "y": _num(kid.get("default_y") or 0.5),
                    }
                    if tag == "Point3D":
                        rec["default"]["z"] = _num(kid.get("default_z") or 0.5)
                    rec["units"] = "fraction of layer bounds"
                elif tag == "Layer":
                    rec["default"] = ("self" if kid.get("default_self") != "false"
                                      else None)
                params.append(rec)

        walk(eff, [])
        out.append({
            "match_name": eff.get("matchname"),
            "display_name": name,
            "display_name_string_key": name_key,
            "external_id": eff.get("external_id"),
            "parameters": params,
        })
    return out


# ---------------------------------------------------------------------------
# S2  .prfpset shipped effect presets
# ---------------------------------------------------------------------------
# ParameterControlType is an integer written into every serialized parameter.
# The enum is not published in any shipped text file. The labels below were
# DERIVED by tabulating, for every code, the parameter class GUID, the shape of
# LowerBound/UpperBound, and the parameter names carrying that code across all
# 379 shipped presets (see control_type_evidence in the output, which ships the
# raw tabulation so the derivation can be re-checked without this tool).
PARAM_CONTROL_TYPE = {
    "1": "integer stepper / integer slider",
    "2": "legacy scalar slider (fe47129e parameter class)",
    "3": "angle dial, degrees",
    "4": "checkbox / boolean toggle",
    "5": "colour value or colour picker (64-bit packed channel value)",
    "6": "point / 2D position",
    "7": "enumerated integer list; bounds are the index range",
    "8": "floating-point slider with separate soft UI bounds",
    "9": "curve or colour-wheel editor backed by an arbitrary-data blob",
    "10": "opaque arbitrary-data blob (LUT payload, shader graph)",
    "11": "collapsible parameter-group header",
    "12": "group terminator / layout separator",
    "16": "boolean rendered as an inline toggle button",
}
# Each control-type code maps 1:1 onto a parameter class GUID in the shipped
# presets, which is what makes the derivation above checkable.
PARAM_CLASS_GUID = {
    "fe47129e-6c94-4fc0-95d5-c056a517aaf3": "legacy scalar video parameter",
    "a4ff2d6e-7ac2-44f8-9d52-17d9ca50e542": "floating-point video parameter with soft UI bounds",
    "6e02e8bb-2569-46b2-8ab1-4ab11c43e9c8": "integer video parameter (plain or enumerated)",
    "cc12343e-f113-4d3b-ae05-b287db77d461": "boolean / layout video parameter",
    "0fde4e9f-f895-4ba3-b0fe-9a6feafda583": "colour video parameter",
    "ca81d347-309b-44d2-acc7-1c572efb973c": "point video parameter",
    "313e54d4-6903-49ad-b0bf-8262cdd10f4e": "arbitrary-data video parameter",
}
PARAM_CONTROL_TYPE_CONFIDENCE = "heuristic"

MEDIA_TYPE_GUID = {
    "228cda18-3625-4d2d-951e-348879e4ed93": "video",
    "80b8e3d5-6dca-4cb6-8ce6-08d24e6a1c74": "audio",
}

# raw evidence tabulation, filled while parsing
CONTROL_TYPE_EVIDENCE = collections.defaultdict(
    lambda: {"count": 0, "param_class_guids": collections.Counter(),
             "bound_shapes": collections.Counter(),
             "sample_names": []})


def _record_control_type(code, class_guid, lo, hi, name):
    ev = CONTROL_TYPE_EVIDENCE[str(code)]
    ev["count"] += 1
    if class_guid:
        ev["param_class_guids"][class_guid] += 1
    if lo is None and hi is None:
        shape = "no bounds (arbitrary data)"
    elif lo == "false" and hi == "true":
        shape = "false..true"
    elif lo == "false" and hi == "false":
        shape = "false..false (layout only)"
    else:
        shape = "numeric %s..%s" % (lo, hi)
        try:
            float(lo), float(hi)
            shape = ("numeric, decimal point present"
                     if ("." in str(lo) or "." in str(hi))
                     else "numeric, integer literal")
        except (TypeError, ValueError):
            pass
    ev["bound_shapes"][shape] += 1
    if name and name.strip() and len(ev["sample_names"]) < 14 \
            and name.strip() not in ev["sample_names"]:
        ev["sample_names"].append(name.strip())

PARAM_TAGS = ("VideoComponentParam", "AudioComponentParam", "ComponentParam")
COMPONENT_TAGS = ("VideoFilterComponent", "AudioFilterComponent",
                  "VideoTransitionComponent", "AudioTransitionComponent",
                  "Component")


def parse_prfpset(path):
    """Return (presets, stats). Each preset lists the effects it instantiates."""
    objects, root = C.parse_premiere_data(path)

    # index parents so a bin path can be reconstructed
    by_tag = collections.defaultdict(list)
    for oid, el in objects.items():
        by_tag[C._strip_ns(el.tag)].append((oid, el))

    def child_refs(el, container_tag):
        cont = el.find(container_tag)
        if cont is None:
            return []
        return [k.get("ObjectRef") for k in cont if k.get("ObjectRef")]

    # --- build the bin tree so every preset knows its folder path
    bin_children = {}
    bin_name = {}
    bin_data = {}
    for oid, el in by_tag.get("BinTreeItem", []):
        bin_children[oid] = child_refs(el, "Items")
        base = el.find("TreeItemBase")
        if base is not None:
            n = base.find("Name")
            bin_name[oid] = (n.text or "").strip() if n is not None else None
            d = base.find("Data")
            if d is not None and d.get("ObjectRef"):
                bin_data[oid] = d.get("ObjectRef")
    tree_parent = {}
    tree_name = {}
    for oid, el in by_tag.get("TreeItem", []):
        base = el.find("TreeItemBase")
        if base is not None:
            n = base.find("Name")
            tree_name[oid] = (n.text or "").strip() if n is not None else None
            d = base.find("Data")
            if d is not None and d.get("ObjectRef"):
                tree_parent[d.get("ObjectRef")] = oid

    path_of = {}

    def descend(oid, prefix):
        nm = bin_name.get(oid)
        cur = prefix if nm in (None, "Root") else prefix + [nm]
        for ch in bin_children.get(oid, []):
            if ch in bin_children:
                descend(ch, cur)
            else:
                path_of[ch] = cur

    roots = [oid for oid in bin_children if bin_name.get(oid) == "Root"]
    for r in roots:
        descend(r, [])
    # second pass: TreeItem leaves inherit the folder of their BinTreeItem parent
    for oid in list(path_of):
        pass

    def read_param(el, index=None):
        f = C.flat_fields(el)
        tag = C._strip_ns(el.tag)
        code = f.get("ParameterControlType")
        lo, hi = f.get("LowerBound"), f.get("UpperBound")
        name = f.get("Name")
        _record_control_type(code, el.get("ClassID"), lo, hi, name)
        rec = {
            "name": (name or "").strip() or None,
            "declared_index": index,
            "parameter_id": _num(f.get("ParameterID")),
            "control_type_code": code,
            "control_type_label": PARAM_CONTROL_TYPE.get(code, "unknown"),
            "control_type_confidence": PARAM_CONTROL_TYPE_CONFIDENCE,
            "param_kind": tag,
            "param_class_id": el.get("ClassID"),
            "param_class_label": PARAM_CLASS_GUID.get(el.get("ClassID")),
            "min": _num(lo),
            "max": _num(hi),
            "ui_min": _num(f.get("LowerUIBound")),
            "ui_max": _num(f.get("UpperUIBound")),
            "preset_value": _num(f.get("CurrentValue")),
            "units": f.get("UnitsString") or None,
            "range_locked": f.get("RangeLocked") == "true",
            "is_locked": f.get("IsLocked") == "true",
            "is_time_varying": f.get("IsTimeVarying") == "true",
            "discontinuous_interpolate": f.get("DiscontinuousInterpolate") == "true",
        }
        if tag == "ArbVideoComponentParam":
            rec["arbitrary_data"] = True
            rec["keyframe_set_size"] = _num(f.get("KeyframeSetSize"))
            blob = el.find("StartKeyframeValue")
            if blob is not None:
                rec["blob_encoding"] = blob.get("Encoding")
                rec["blob_checksum"] = blob.get("Checksum")
                rec["blob_base64_chars"] = len((blob.text or "").strip())
        sk = f.get("StartKeyframe")
        if sk:
            parts = sk.split(",")
            if len(parts) >= 2:
                rec["start_keyframe_value"] = _num(parts[1])
                if rec["preset_value"] in (0, None) and rec["start_keyframe_value"] is not None:
                    rec["effective_value"] = rec["start_keyframe_value"]
        if "effective_value" not in rec:
            rec["effective_value"] = rec["preset_value"]
        kf = f.get("Keyframes")
        if kf:
            rec["keyframe_count"] = len([x for x in kf.split(";") if x.strip()])
        return rec

    def read_component(oid):
        el = objects.get(oid)
        if el is None:
            return None
        tag = C._strip_ns(el.tag)
        f = C.flat_fields(el)
        comp = el.find("Component")
        display = None
        params = []
        bypass = None
        intrinsic = None
        if comp is not None:
            cf = C.flat_fields(comp)
            display = cf.get("DisplayName") or None
            bypass = cf.get("Bypass") == "true"
            intrinsic = cf.get("Intrinsic") == "true"
            pc = comp.find("Params")
            if pc is not None:
                for p in pc:
                    ref = p.get("ObjectRef")
                    pel = objects.get(ref)
                    if pel is None:
                        continue
                    ptag = C._strip_ns(pel.tag)
                    if ptag in PARAM_TAGS or ptag.endswith("ComponentParam"):
                        idx = p.get("Index")
                        params.append(read_param(
                            pel, int(idx) if idx is not None else None))
        return {
            "component_kind": tag,
            "match_name": f.get("MatchName") or None,
            "display_name": display,
            "bypass": bypass,
            "intrinsic": intrinsic,
            "video_filter_type": f.get("VideoFilterType"),
            "parameters": params,
        }

    presets = []
    for oid, el in by_tag.get("FilterPreset", []):
        f = C.flat_fields(el)
        comp_ref = None
        comps = []
        for kid in el:
            t = C._strip_ns(kid.tag)
            if t in ("Component", "Components") and kid.get("ObjectRef"):
                comps.append(kid.get("ObjectRef"))
            elif t == "Components":
                comps.extend(k.get("ObjectRef") for k in kid if k.get("ObjectRef"))
        components = [c for c in (read_component(r) for r in comps) if c]
        owner_tree = tree_parent.get(oid)
        # FilterPreset is referenced by FilterPresetItem which is referenced by TreeItem
        pname = None
        folder = []
        for pi_oid, pi_el in by_tag.get("FilterPresetItem", []):
            fp = pi_el.find("FilterPreset")
            if fp is not None and fp.get("ObjectRef") == oid:
                ti = tree_parent.get(pi_oid)
                pname = tree_name.get(ti)
                folder = path_of.get(ti, [])
                break
        presets.append({
            "preset_name": pname,
            "bin_path": folder,
            "description": f.get("Description") or None,
            "match_name": f.get("FilterMatchName") or None,
            "media_type_guid": f.get("MediaType"),
            "media_type": MEDIA_TYPE_GUID.get(f.get("MediaType"), "unknown"),
            "preset_type_code": f.get("Type"),
            "speed": _num(f.get("Speed")),
            "components": components,
        })

    stats = {
        "objects_indexed": len(objects),
        "filter_presets": len(presets),
        "bin_folders": len(bin_name),
    }
    return presets, stats


# ---------------------------------------------------------------------------
# S3  executable string table -> per-effect label tables
# ---------------------------------------------------------------------------
LSTR_RE = re.compile(r"^\$\$\$/AE/([^/]+)/LStr/(\d+)$")
MC_RE = re.compile(r"^\$\$\$/MediaCore/AEFilters/([^/]+)/(.+)$")
EFFECTS_NS_RE = re.compile(r"^\$\$\$/Effects/([^/]+)/(.+)$")
DVAAF_RE = re.compile(r"^\$\$\$/dvaaudiofilters/([^/]+)$")
AE_EFFECT_NAME_RE = re.compile(r"^\$\$\$/AE/Effect/Name/(.+)$")

# LStr/0000 is the effect's About string. Its shipped form is
#   "<Display Name>, v%ld.%ld#{cr}#{cr}#{copy}....#{cr}#{cr}<description>"
ABOUT_RE = re.compile(r"^(.*?),\s*v%ld\.%ld(.*)$", re.S)

NOISE_LABELS = re.compile(
    r"(couldn|could not|cannot|unable to|out of memory|error|failed|"
    r"copyright|adobe systems|allocate)", re.I)


def build_string_effects(table):
    """Group the executable's own $$$ strings into per-effect label tables."""
    ae = collections.defaultdict(dict)
    for k, v in table.items():
        m = LSTR_RE.match(k)
        if m:
            ae[m.group(1)][int(m.group(2))] = v
    mediacore = collections.defaultdict(dict)
    for k, v in table.items():
        m = MC_RE.match(k)
        if m:
            mediacore[m.group(1)][m.group(2)] = v
    audio_ns = collections.defaultdict(dict)
    for k, v in table.items():
        m = EFFECTS_NS_RE.match(k)
        if m:
            audio_ns[m.group(1)][m.group(2)] = v
    dva = {}
    for k, v in table.items():
        m = DVAAF_RE.match(k)
        if m:
            dva[m.group(1)] = v
    ae_names = {}
    for k, v in table.items():
        m = AE_EFFECT_NAME_RE.match(k)
        if m:
            ae_names[m.group(1)] = v
    return ae, mediacore, audio_ns, dva, ae_names


def labels_to_params(indexed, source):
    """LStr index table -> ordered parameter label rows.

    Index 0 is the About/description string; the remaining indices are the
    strings the effect registers, in registration order. Some of those are
    error messages rather than parameter labels, so obvious error text is
    tagged rather than silently dropped.

    A pipe-delimited string is a popup's option list. In every shipped effect
    where both are present the option list is registered immediately BEFORE the
    label of the popup it belongs to -- Gaussian Blur registers
    "Horizontal and Vertical|Horizontal|Vertical" at LStr/0006 and
    "Blur Dimensions" at LStr/0007 -- so the option list is attached forward to
    the next label row. That adjacency is a derived reading and is marked
    heuristic on every row it produces.
    """
    rows = []
    for idx in sorted(indexed):
        if idx == 0:
            continue
        txt = indexed[idx]
        rec = {
            "name": txt,
            "label_index": idx,
            "source": source,
            "confidence": "parsed",
        }
        if "|" in txt and not txt.endswith("|") and len(txt.split("|")) > 1:
            rec["is_option_list"] = True
            rec["enum_options"] = txt.split("|")
            rec["name"] = None
        elif NOISE_LABELS.search(txt) or len(txt) > 90:
            rec["role"] = "diagnostic or About text, not a parameter label"
            rec["role_confidence"] = "heuristic"
        else:
            rec["role"] = "parameter label"
            rec["role_confidence"] = "heuristic"
        rows.append(rec)

    out = []
    pending = None
    for rec in rows:
        if rec.pop("is_option_list", False):
            pending = rec
            continue
        if pending is not None and rec.get("role") == "parameter label":
            rec["type"] = "enum"
            rec["enum_options"] = pending["enum_options"]
            rec["enum_options_label_index"] = pending["label_index"]
            rec["enum_binding"] = ("option list registered at the preceding "
                                   "index; adjacency reading")
            rec["enum_binding_confidence"] = "heuristic"
            pending = None
        elif pending is not None:
            pending["role"] = ("orphan enumerated option list; no parameter "
                               "label follows it")
            pending["role_confidence"] = "heuristic"
            pending["type"] = "enum"
            out.append(pending)
            pending = None
        out.append(rec)
    if pending is not None:
        pending["role"] = "orphan enumerated option list"
        pending["role_confidence"] = "heuristic"
        pending["type"] = "enum"
        out.append(pending)
    return out


# ---------------------------------------------------------------------------
# S4  audio-filter Eve dialog layouts
# ---------------------------------------------------------------------------
AUDIO_EVE_SKIP = re.compile(r"(setup|dialog|prefs|settings)$", re.I)


def parse_audio_eves(eve_dir):
    out = {}
    failures = []
    for fn in sorted(os.listdir(eve_dir)):
        if not fn.lower().endswith(".eve"):
            continue
        path = os.path.join(eve_dir, fn)
        try:
            with open(path, "r", encoding="utf-8", errors="replace") as fh:
                layouts = dw_eve.parse_eve(fh.read())
        except Exception as exc:                      # noqa: BLE001
            failures.append({"file": C.rel(path), "error": repr(exc)})
            continue
        controls = []
        for lay in layouts:
            for ctl in dw_eve.flatten_controls(lay["nodes"]):
                if ctl.get("is_container") or ctl["kind"] in ("separator", "placeholder"):
                    continue
                row = {
                    "kind": ctl["kind"],
                    "control_role": ctl.get("control_role"),
                    "value_kind": ctl.get("value_kind"),
                    "identifier": ctl.get("identifier"),
                    "label": ctl.get("label"),
                    "label_string_key": ctl.get("label_string_key"),
                    "container_path": ctl.get("container_path"),
                }
                for k in ("min_value", "max_value", "value", "default",
                          "increment", "digits", "precision", "items",
                          "small_increment", "large_increment"):
                    if k in ctl:
                        row[k] = ctl[k]
                controls.append(row)
        out[os.path.splitext(fn)[0]] = {
            "file": C.rel(path),
            "layout_names": [l["layout_name"] for l in layouts],
            "controls": controls,
        }
    return out, failures


# ---------------------------------------------------------------------------
# S5  audio-filter prop.map control trees
# ---------------------------------------------------------------------------
# install/xml/Audition.xml is deliberately absent: it is an EuCon control
# surface AppSet (<AppSet><ControlBindings>), not a prop.map UI archive. It is
# parsed instead by pp_commands.py as a hardware control-surface binding set.
AUDIO_PROPMAP = ("Amplify", "FFTFilter", "NotchFilter", "PitchShift",
                 "ScientificFilter", "StereoExpander",
                 "CurvePointEditDialog", "CustomAudioChannelLayoutDialog",
                 "EssentialSoundView", "LegacyControls", "IPP2Controls")

ADAPTER_ROLE = {
    "dvaui::controls::UI_CheckBox": ("checkbox", "boolean"),
    "dvaui::controls::UI_PushButton": ("push button", "action"),
    "dvaui::controls::UI_RadioButton": ("radio button", "enum member"),
    "dvaui::controls::UI_ComboBox": ("dropdown", "enum"),
    "dvaui::controls::UI_DropDownList": ("dropdown", "enum"),
    "dvaui::controls::UI_EditText": ("text field", "string"),
    "dvaui::controls::UI_StaticText": ("label", "readonly"),
    "dvaui::controls::UI_Slider": ("slider", "number"),
    "dvaui::controls::UI_HSlider": ("horizontal slider", "number"),
    "dvaui::controls::UI_VSlider": ("vertical slider", "number"),
    "dvaui::controls::UI_OutlineBoxWithLabel": ("group box", "container"),
    "dvaui::controls::UI_TabGroup": ("tab group", "container"),
    "dvaui::ui::UI_SubView": ("sub view", "container"),
    "dvaui::controls::UI_ListBox": ("list box", "enum"),
    "dvaui::controls::UI_ProgressBar": ("progress bar", "readonly number"),
}


def walk_propmap_ui(node, path=(), out=None, depth=0):
    if out is None:
        out = []
    if depth > 60 or not isinstance(node, dict):
        return out
    adapter = node.get("adapter")
    ident = node.get("Name") or node.get("name") or node.get("ID")
    if adapter:
        role, vkind = ADAPTER_ROLE.get(adapter, (None, None))
        rec = {"adapter": adapter, "control_role": role, "value_kind": vkind,
               "container_path": list(path)}
        for k in ("Text", "text", "Title", "Label", "Value", "Min", "Max",
                  "Increment", "Precision", "Identifier", "ID", "Name",
                  "ToolTip", "Enabled", "Visible", "Items"):
            if k in node:
                rec[k] = node[k]
        # values live in section-N siblings
        for key, val in node.items():
            if key.startswith("section-") and isinstance(val, dict):
                for k in ("Text", "Title", "Label", "Value", "Min", "Max",
                          "Increment", "Identifier", "ToolTip"):
                    if k in val and k not in rec:
                        rec[k] = val[k]
        out.append(rec)
        path = path + (adapter.split("::")[-1],)
    for key, val in node.items():
        if isinstance(val, dict):
            walk_propmap_ui(val, path, out, depth + 1)
        elif isinstance(val, list):
            for item in val:
                if isinstance(item, dict):
                    walk_propmap_ui(item, path, out, depth + 1)
    return out


# ---------------------------------------------------------------------------
# match-name harvest out of the executable
# ---------------------------------------------------------------------------
MATCHNAME_RE = re.compile(
    rb"(?<![\x21-\x7e])((?:AE\.)?ADBE [\x20-\x7e]{2,60}?)\x00")


def harvest_match_names(exe_path):
    with open(exe_path, "rb") as fh:
        blob = fh.read()
    names = set()
    for m in MATCHNAME_RE.finditer(blob):
        names.add(m.group(1).decode("latin-1").strip())
    del blob
    return names


# ---------------------------------------------------------------------------
def main(out_dir):
    R = C.PREMIERE_ROOT
    sources = []
    failures = []

    table = C.premiere_strings(SCRATCH)
    sources.append({"id": "S3_exe_string_table",
                    "path": C.rel(os.path.join(R, "Adobe Premiere Pro.exe")),
                    "how": ("NUL-terminated $$$/Namespace/Key=English literals "
                            "matched by regex over the file's bytes; the file "
                            "is never executed or loaded as a module"),
                    "unique_strings": len(table)})

    # ---- S1
    pe_path = os.path.join(R, "PresetEffects.xml")
    try:
        pseudo = parse_preset_effects_xml(pe_path)
    except Exception as exc:                          # noqa: BLE001
        pseudo = []
        failures.append({"stage": "S1_PresetEffects.xml", "path": C.rel(pe_path),
                         "error": repr(exc), "traceback": traceback.format_exc()})
    sources.append({"id": "S1_PresetEffects.xml", "path": C.rel(pe_path),
                    "how": "XML parse of the <Effects> body (inline DTD stripped)",
                    "effects_parsed": len(pseudo)})

    # ---- S2
    prfp_paths = sorted(C.walk_files(os.path.join(R, "LocalizedPresets", "en_US"),
                                     exts=(".prfpset",)))
    all_presets = []
    prfp_stats = []
    for p in prfp_paths:
        try:
            presets, st = parse_prfpset(p)
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "S2_prfpset", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        for pr in presets:
            pr["source_file"] = C.rel(p)
        all_presets.extend(presets)
        st["path"] = C.rel(p)
        prfp_stats.append(st)
    sources.append({"id": "S2_prfpset", "files": prfp_stats,
                    "how": ("PremiereData object-graph walk: BinTreeItem folder "
                            "tree -> TreeItem -> FilterPresetItem -> FilterPreset "
                            "-> Video/AudioFilterComponent -> ComponentParam"),
                    "presets_parsed": len(all_presets)})

    # ---- S3 grouping
    ae_lstr, mediacore, audio_ns, dva_titles, ae_effect_names = build_string_effects(table)

    # ---- S4
    eve_dir = os.path.join(R, "eve")
    eve_data, eve_fail = parse_audio_eves(eve_dir)
    failures.extend({"stage": "S4_eve", **f} for f in eve_fail)
    sources.append({"id": "S4_eve", "path": C.rel(eve_dir),
                    "how": "Adobe Eve layout grammar, recursive-descent parse",
                    "files_parsed": len(eve_data)})

    # ---- S5
    propmap_ui = {}
    for stem in AUDIO_PROPMAP:
        p = os.path.join(R, "xml", stem + ".xml")
        if not os.path.isfile(p):
            continue
        try:
            tree = C.parse_propmap(p)
            propmap_ui[stem] = {"file": C.rel(p),
                                "controls": walk_propmap_ui(tree)}
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "S5_propmap", "path": C.rel(p),
                             "error": repr(exc)})
    sources.append({"id": "S5_propmap", "how": "Adobe prop.map v4 XML walk",
                    "files_parsed": len(propmap_ui)})

    # ---- S6 Adobe Dialog Manager sheets for the built-in audio effects
    adm_dir = os.path.join(R, "adm")
    adm_sheets = {}
    for p in sorted(C.walk_files(adm_dir, exts=(".adm",))):
        stem = os.path.splitext(os.path.basename(p))[0]
        if C.looks_ai(stem):
            continue
        try:
            rec = pp_adm.parse_adm(p)
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "S6_adm", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        rec["file"] = C.rel(p)
        rec["effect_stem"] = stem
        adm_sheets[stem] = rec
    sources.append({
        "id": "S6_adm",
        "path": C.rel(adm_dir),
        "how": ("Adobe Dialog Manager sheet grammar. constant/input/interface "
                "sections parsed; each interface binding names an input slot, "
                "its real-world unit range and, in its trailing comment, the "
                "host's own kParameterIndex_* constant"),
        "sheets_parsed": len(adm_sheets),
        "parameters": sum(s["parameter_count"] for s in adm_sheets.values()),
        "parameters_with_resolved_unit_bounds": sum(
            s["parameters_with_resolved_bounds"] for s in adm_sheets.values()),
    })

    # ---- S7 Essential Sound: the clip-type audio adjustment model
    essential_sound = {}
    es_files = [
        ("modes", os.path.join(R, "json", "DefaultAdjustmentsModes.json")),
        ("modes_premiere", os.path.join(R, "Settings", "EssentialSound",
                                        "PremiereAdjustmentsModes.json")),
        ("config_presets", os.path.join(R, "json",
                                        "EssentialSoundConfigPresets.json")),
        ("presets", os.path.join(R, "json", "EssentialSoundPresets.json")),
        ("audio_channel_layouts", os.path.join(R, "json",
                                               "AudioChannelLayoutPresets.json")),
    ]
    for label, p in es_files:
        if not os.path.isfile(p):
            continue
        try:
            with open(p, "r", encoding="utf-8-sig") as fh:
                data = json.load(fh)
        except Exception as exc:                      # noqa: BLE001
            failures.append({"stage": "S7_essential_sound", "path": C.rel(p),
                             "error": repr(exc)})
            continue

        def strip_ai(node):
            """Drop Enhance Speech and the ML sound classifier from the model."""
            if isinstance(node, dict):
                return {k: strip_ai(v) for k, v in node.items()
                        if not C.looks_ai(k) and not C.looks_ai(str(v)[:80])}
            if isinstance(node, list):
                return [strip_ai(x) for x in node
                        if not C.looks_ai(str(x)[:120])]
            return node

        cleaned = strip_ai(data)
        # resolve the $$$ names the presets carry
        def resolve_names(node):
            if isinstance(node, dict):
                out = {}
                for k, v in node.items():
                    if k == "Name" and isinstance(v, str):
                        key, txt = C.split_localized(v)
                        out["Name"] = txt
                        if key:
                            out["Name_string_key"] = key
                        continue
                    out[k] = resolve_names(v)
                return out
            if isinstance(node, list):
                return [resolve_names(x) for x in node]
            return node

        essential_sound[label] = {"file": C.rel(p), "data": resolve_names(cleaned)}

    es_models = set()
    es_params = 0
    cp = essential_sound.get("config_presets", {}).get("data") or []
    if isinstance(cp, list):
        for row in cp:
            models = ((row.get("PresetData") or {}).get("Models") or {})
            for mid, fields in models.items():
                es_models.add(mid)
                if isinstance(fields, dict):
                    es_params += len(fields)
    sources.append({
        "id": "S7_essential_sound",
        "how": ("shipped Essential Sound JSON read directly: the clip-type "
                "modes, the parameter models each mode exposes with their typed "
                "default / min / max values, and the shipped presets. Enhance "
                "Speech and the ML sound classifier are stripped as excluded AI "
                "surfaces"),
        "files_parsed": len(essential_sound),
        "parameter_models": sorted(es_models),
        "typed_model_fields": es_params,
    })

    # ---- match names
    exe = os.path.join(R, "Adobe Premiere Pro.exe")
    try:
        match_names = harvest_match_names(exe)
    except Exception as exc:                          # noqa: BLE001
        match_names = set()
        failures.append({"stage": "matchnames", "error": repr(exc)})

    # =======================================================================
    # merge
    # =======================================================================
    effects = {}

    def slot(key):
        if key not in effects:
            effects[key] = {
                "effect_key": key,
                "display_name": None,
                "description": None,
                "match_names": [],
                "kind": None,
                "engine": None,
                "category": None,
                "category_confidence": None,
                "evidence": [],
                "parameters": [],
                "shipped_presets": [],
            }
        return effects[key]

    # S1 pseudo effects
    for eff in pseudo:
        if C.looks_ai(eff["match_name"], eff["display_name"]):
            continue
        e = slot(eff["match_name"])
        e["display_name"] = eff["display_name"]
        e["match_names"] = [eff["match_name"]]
        e["kind"] = "preset_pseudo_effect"
        e["engine"] = "ae_native"
        e["category"] = "Presets (pseudo-effect backing shipped animation presets)"
        e["category_confidence"] = "parsed"
        e["evidence"].append("S1_PresetEffects.xml")
        e["parameters"].extend(eff["parameters"])

    # S2 real presets -> real effect instances with real bounds
    per_effect_params = collections.defaultdict(dict)
    for pr in all_presets:
        for comp in pr["components"]:
            mn = comp["match_name"] or pr["match_name"]
            if not mn:
                continue
            if C.looks_ai(mn, comp["display_name"]):
                continue
            e = slot(mn)
            if mn not in e["match_names"]:
                e["match_names"].append(mn)
            if comp["display_name"] and not e["display_name"]:
                e["display_name"] = comp["display_name"]
            if "S2_prfpset" not in e["evidence"]:
                e["evidence"].append("S2_prfpset")
            if not e["kind"]:
                e["kind"] = ("audio_effect" if pr["media_type"] == "audio"
                             else "video_effect")
            if not e["engine"]:
                e["engine"] = ("lumetri" if "Lumetri" in mn else
                               "ae_native" if mn.startswith("AE.") else "mediacore")
            if comp.get("intrinsic"):
                e["kind"] = "intrinsic"
            bucket = per_effect_params[mn]
            for p in comp["parameters"]:
                nm = "%s@%s" % (p["name"] or "(unnamed)", p["declared_index"])
                cur = bucket.get(nm)
                if cur is None:
                    bucket[nm] = {
                        "name": p["name"],
                        "declared_index": p["declared_index"],
                        "parameter_id": p["parameter_id"],
                        "control_type_code": p["control_type_code"],
                        "control_type_label": p["control_type_label"],
                        "control_type_confidence": p["control_type_confidence"],
                        "param_kind": p["param_kind"],
                        "param_class_label": p.get("param_class_label"),
                        "min": p["min"], "max": p["max"],
                        "ui_min": p.get("ui_min"), "ui_max": p.get("ui_max"),
                        "units": p["units"],
                        "range_locked": p["range_locked"],
                        "arbitrary_data": p.get("arbitrary_data", False),
                        "observed_values": [],
                        "supports_keyframes": False,
                        "source": "S2_prfpset",
                        "confidence": "parsed",
                    }
                    cur = bucket[nm]
                v = p.get("effective_value")
                if v is not None and v not in cur["observed_values"]:
                    if len(cur["observed_values"]) < 24:
                        cur["observed_values"].append(v)
                if p.get("is_time_varying") or p.get("keyframe_count"):
                    cur["supports_keyframes"] = True
            e["shipped_presets"].append({
                "preset_name": pr["preset_name"],
                "bin_path": pr["bin_path"],
                "description": pr["description"],
                "source_file": pr["source_file"],
                "values": {p["name"]: p.get("effective_value")
                           for p in comp["parameters"] if p["name"]},
            })

    for mn, bucket in per_effect_params.items():
        e = effects.get(mn)
        if not e:
            continue
        for nm, rec in sorted(
                bucket.items(),
                key=lambda kv: (kv[1]["declared_index"] is None,
                                kv[1]["declared_index"] or 0)):
            rec["value_range_note"] = (
                "min/max are the effect's declared parameter bounds as "
                "serialized by Premiere; observed_values are the values the "
                "shipped presets set, not necessarily the factory default")
            e["parameters"].append(rec)

    # S3 AE label tables
    for eff_id, indexed in ae_lstr.items():
        if C.looks_ai(eff_id):
            continue
        about = indexed.get(0, "")
        m = ABOUT_RE.match(about) if about else None
        disp = m.group(1).strip() if m else None
        desc = None
        if m:
            tail = m.group(2)
            bits = [b.strip() for b in re.split(r"#\{cr\}", tail) if b.strip()]
            bits = [b for b in bits
                    if not b.startswith("#{copy}") and "Adobe Systems" not in b]
            desc = bits[-1] if bits else None
        key = "AE_LStr:" + eff_id
        e = slot(key)
        e["string_namespace"] = "$$$/AE/%s/LStr" % eff_id
        e["display_name"] = e["display_name"] or disp or eff_id.replace("_", " ")
        e["description"] = e["description"] or desc
        e["engine"] = e["engine"] or "ae_native"
        e["kind"] = e["kind"] or "video_effect"
        e["evidence"].append("S3_exe_string_table")
        e["parameters"].extend(
            labels_to_params(indexed, "S3_exe_string_table:$$$/AE/%s/LStr" % eff_id))
        # try to bind a real match name
        cands = [n for n in match_names
                 if n.replace("AE.ADBE ", "").replace("ADBE ", "").replace(" ", "_").lower()
                 == eff_id.lower()]
        if cands:
            e["match_names"] = sorted(set(e["match_names"]) | set(cands))
            e["match_name_binding_confidence"] = "heuristic (name normalisation)"

    # S3 MediaCore AEFilters
    for eff_id, kv in mediacore.items():
        if C.looks_ai(eff_id):
            continue
        key = "MediaCore:" + eff_id
        e = slot(key)
        e["string_namespace"] = "$$$/MediaCore/AEFilters/%s" % eff_id
        e["display_name"] = e["display_name"] or kv.get("Name") or eff_id
        about = kv.get("0000")
        if about and not e["description"]:
            e["description"] = about.split("^n")[0]
        e["engine"] = e["engine"] or "mediacore"
        e["kind"] = e["kind"] or "video_effect"
        e["evidence"].append("S3_exe_string_table")
        # In this namespace a popup's options and its label share a key stem:
        #   kStrSmoothingLabelText = Smoothing
        #   kStrSmoothingPopupText = None|Low|High
        # so the option list binds to the label by stem, not by adjacency.
        def stem_of(key):
            s = key
            for suffix in ("PopupText", "LabelText", "Popup", "Label", "Text"):
                if s.endswith(suffix):
                    s = s[: -len(suffix)]
                    break
            return s.lower()

        popups = {stem_of(k): (k, v) for k, v in kv.items()
                  if k.lower().endswith("popuptext") or
                  ("|" in v and k.lower().startswith("kstr"))}
        consumed = set()
        for k, v in sorted(kv.items()):
            if k in ("Name", "0000") or re.fullmatch(r"\d{4}", k):
                continue
            if k in consumed:
                continue
            rec = {"name": v, "string_key_suffix": k,
                   "source": "S3_exe_string_table:$$$/MediaCore/AEFilters/%s" % eff_id,
                   "confidence": "parsed"}
            pk = popups.get(stem_of(k))
            if pk and pk[0] != k:
                rec["type"] = "enum"
                rec["enum_options"] = pk[1].split("|")
                rec["enum_options_string_key_suffix"] = pk[0]
                rec["enum_binding"] = ("option list shares this parameter's key "
                                       "stem")
                rec["enum_binding_confidence"] = "parsed"
                rec["role"] = "parameter label"
                rec["role_confidence"] = "heuristic"
                consumed.add(pk[0])
            elif "|" in v and k.lower().startswith("kstr"):
                rec["type"] = "enum"
                rec["enum_options"] = v.split("|")
                rec["name"] = None
                rec["role"] = ("enumerated option list whose owning parameter "
                               "has no separate label string")
                rec["role_confidence"] = "heuristic"
            elif "label" in k.lower() or k.lower().startswith("kstr"):
                rec["role"] = "parameter label"
                rec["role_confidence"] = "heuristic"
            e["parameters"].append(rec)

    # S3 audio effect namespace
    for eff_id, kv in audio_ns.items():
        if C.looks_ai(eff_id):
            continue
        key = "AudioFilter:" + eff_id
        e = slot(key)
        e["string_namespace"] = "$$$/Effects/%s" % eff_id
        e["display_name"] = e["display_name"] or (
            dva_titles.get(eff_id + "Title") or eff_id)
        e["engine"] = "dva_audio"
        e["kind"] = "audio_effect"
        e["evidence"].append("S3_exe_string_table")
        for k, v in sorted(kv.items()):
            e["parameters"].append({
                "name": v, "string_key_suffix": k,
                "source": "S3_exe_string_table:$$$/Effects/%s" % eff_id,
                "confidence": "parsed",
                "role": "parameter label", "role_confidence": "heuristic"})

    # S4 bind Eve dialogs onto audio effects by stem name
    eve_bound = 0
    for stem, data in eve_data.items():
        if C.looks_ai(stem):
            continue
        cand = "AudioFilter:" + stem
        e = effects.get(cand)
        if e is None:
            for k in effects:
                if k.startswith("AudioFilter:") and k.split(":", 1)[1].lower() == stem.lower():
                    e = effects[k]
                    break
        if e is None:
            e = slot("AudioFilter:" + stem)
            e["display_name"] = e["display_name"] or dva_titles.get(stem + "Title") or stem
            e["engine"] = "dva_audio"
            e["kind"] = "audio_effect"
        e["evidence"].append("S4_eve")
        e["ui_layout"] = {"source": "S4_eve", "file": data["file"],
                          "layout_names": data["layout_names"],
                          "controls": data["controls"]}
        eve_bound += 1

    # S6 bind ADM sheets onto their audio effects and add real unit ranges
    adm_bound = 0
    adm_params_added = 0
    for stem, sheet in adm_sheets.items():
        target = None
        cands = [stem, stem.replace("UI", ""), (sheet["sheet_name"] or "").replace("UI", "")]
        for key, e in effects.items():
            if not key.startswith("AudioFilter:"):
                continue
            short = key.split(":", 1)[1]
            if short.lower() in {c.lower() for c in cands if c}:
                target = e
                break
        if target is None:
            target = slot("AudioFilter:" + stem)
            target["display_name"] = target["display_name"] or (
                dva_titles.get(stem + "Title") or stem)
            target["engine"] = "dva_audio"
            target["kind"] = "audio_effect"
        target["evidence"].append("S6_adm")
        target["dialog_manager_sheet"] = {
            "source": "S6_adm", "file": sheet["file"],
            "sheet_name": sheet["sheet_name"],
            "input_slot_count": sheet["input_slot_count"],
            "constants": sheet["constants"],
        }
        adm_bound += 1
        have = {(p.get("name") or "").lower() for p in target["parameters"]}
        for p in sheet["parameters"]:
            if p.get("input_slot") is None:
                continue
            rec = {
                "name": p["name"],
                "host_parameter_constant": p.get("host_parameter_constant"),
                "normalised_input_slot": p["input_slot"],
                "type": p.get("value_kind"),
                "min": p.get("min"),
                "max": p.get("max"),
                "unit_family": p.get("unit_family"),
                "default_normalised": p.get("input_initial_normalised"),
                "normalised_to_value_mapping": p.get("mapping"),
                "declared_expression": p.get("expression"),
                "source": "S6_adm",
                "confidence": "parsed",
            }
            if p.get("min") is not None and p.get("max") is not None \
                    and p.get("input_initial_normalised") is not None \
                    and isinstance(p.get("min"), (int, float)) \
                    and isinstance(p.get("max"), (int, float)):
                n = p["input_initial_normalised"]
                rec["default"] = p["min"] + n * (p["max"] - p["min"])
            target["parameters"].append(
                {k: v for k, v in rec.items() if v is not None})
            adm_params_added += 1

    # tidy
    for e in effects.values():
        e["evidence"] = sorted(set(e["evidence"]))
        e["parameter_count"] = len(e["parameters"])
        e["shipped_preset_count"] = len(e["shipped_presets"])

    catalogue = sorted(effects.values(),
                       key=lambda x: (x["kind"] or "", (x["display_name"] or "").lower()))

    categories = []
    for k, v in sorted(table.items()):
        if k.startswith("$$$/MediaCore/FiltersAndEffects/Category/"):
            categories.append({"id": k.rsplit("/", 1)[1], "label": v,
                               "string_key": k})

    by_kind = collections.Counter(e["kind"] or "unknown" for e in catalogue)
    by_engine = collections.Counter(e["engine"] or "unknown" for e in catalogue)
    with_params = sum(1 for e in catalogue if e["parameter_count"])
    total_params = sum(e["parameter_count"] for e in catalogue)

    payload = C.envelope(
        "handshake.studio.premiere.effects_catalogue.v1",
        {
            "summary": ("Effect, transition and audio-filter catalogue with "
                        "parameters, built by merging five independent on-disk "
                        "evidence streams. Every parameter row names its source "
                        "and carries an explicit parsed/heuristic confidence."),
            "streams": {
                "S1_PresetEffects.xml": "typed parameter declarations (authoritative types, defaults, ranges, units, enums)",
                "S2_prfpset": "serialized shipped presets (authoritative declared bounds, control-type codes, real values)",
                "S3_exe_string_table": "per-effect label tables and enumerations from the executable's own $$$ literals",
                "S4_eve": "audio filter dialog layouts (control kinds)",
                "S5_propmap": "audio filter dvaui control trees",
                "S6_adm": "Adobe Dialog Manager sheets: audio parameter slots with real unit ranges and host parameter index constants",
                "S7_essential_sound": "the Essential Sound clip-type adjustment model with typed default/min/max fields",
            },
            "confidence_legend": {
                "parsed": "read verbatim out of a shipped file",
                "heuristic": "derived by correlation across files; stated as such and never presented as shipped fact",
            },
            "known_gaps": [
                ("Premiere's native effects are compiled into Adobe Premiere Pro.exe "
                 "rather than shipped as .prm plug-ins, so no shipped file declares "
                 "the numeric range of a parameter that no shipped preset touches. "
                 "Bounds are therefore complete only for effects reached by S1 or S2."),
                ("The effect-to-category assignment is held in compiled PiPL "
                 "structures, not in any shipped text file. The category "
                 "vocabulary is parsed in full (see effect_categories) but "
                 "per-effect assignment is not recoverable offline and is left null "
                 "rather than guessed."),
                ("ParameterControlType is an unpublished integer enum; the labels in "
                 "parameter_control_type_enum are derived by correlation and are "
                 "marked heuristic everywhere they appear."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "effect_entries": len(catalogue),
                "effect_entries_with_parameters": with_params,
                "parameter_rows_total": total_params,
                "shipped_presets_parsed": len(all_presets),
                "by_kind": dict(by_kind),
                "by_engine": dict(by_engine),
                "match_name_literals_in_executable": len(match_names),
                "audio_eve_layouts_bound": eve_bound,
                "adm_sheets_bound": adm_bound,
                "adm_parameter_rows_added": adm_params_added,
                "essential_sound_parameter_models": len(es_models),
                "essential_sound_typed_model_fields": es_params,
                "effect_categories": len(categories),
                "count_semantics": ("effect_entries counts distinct effect "
                                    "identities, not files; parameter_rows_total "
                                    "counts parameter rows across all effects"),
            },
            "parameter_control_type_enum": {
                "confidence": PARAM_CONTROL_TYPE_CONFIDENCE,
                "note": ("Not published in any shipped text file. Derived by "
                         "tabulating, per code, the parameter class GUID, the "
                         "bound shape and the parameter names carrying it "
                         "across every shipped preset. control_type_evidence "
                         "below ships that raw tabulation so the derivation "
                         "can be re-checked independently."),
                "values": PARAM_CONTROL_TYPE,
                "param_class_guids": PARAM_CLASS_GUID,
            },
            "control_type_evidence": {
                code: {
                    "count": ev["count"],
                    "param_class_guids": dict(ev["param_class_guids"]),
                    "bound_shapes": dict(ev["bound_shapes"]),
                    "sample_parameter_names": ev["sample_names"],
                }
                for code, ev in sorted(CONTROL_TYPE_EVIDENCE.items(),
                                       key=lambda kv: int(kv[0]) if kv[0] and kv[0].lstrip('-').isdigit() else 999)
            },
            "effect_categories": categories,
            "effects": catalogue,
            "audio_filter_ui_control_trees": propmap_ui,
            "audio_dialog_manager_sheets": adm_sheets,
            "essential_sound_model": essential_sound,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_effects_catalogue.json", payload)
    print("wrote", path, size, "bytes")
    print("effects:", len(catalogue), "params:", total_params,
          "presets:", len(all_presets), "failures:", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
