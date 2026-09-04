"""pp_iridas.py -- parser for the IRIDAS shader files that implement Lumetri.

Premiere ships the Lumetri colour engine as `.irtp` files under install/Lumetri.
Each one is a SpeedGrade/IRIDAS shader: a declarative control block followed by
an `<IRIDAScript>` body holding the actual per-pixel maths. Shipped `.look`
files are XML documents that stack those shaders with concrete parameter values.

Grammar of the control block, derived from the shipped bytes:

    <shader name="PrColor.BasicCorrection3" noopwhendefault=1 passes=2>
    <lut name="lut1" unit=2>
    <lookup lut="lut1" function="Exposure3" swizzle="0r,0g" variables="A,B">
    <slider name="Temp" label=""$$$/K=Temp"" min=-100 max=100 default=0
            mincolor="0080FF" maxcolor="FF8000" forcemin=1 forcemax=1>
    <checkbox name="HDR" default=0> "$$$/K=HDR"<br>
    <colorselector name="Offset" mode=... elevation=... picker=...>
    <curveeditor name="LuminanceCurve" color=... mode=... shape=...>
    <dropdown name="Blend" items="None|Multiply|..." values="0|1|..." default=0>
    <editbox name="X" min= max= gang= type= default=>
    <rangecontrol name= range= tolerance= invert=>
    <printerlights name= colorselector=>
    <tabbar name= tabs="A|B|C" variables=...>
    <gang name= default= align=>
    <extern name="offset" default="0,0,0">            (no UI, script input)
    <hwslider id= name= min= max= default=>           (hardware surface bind)
    <hwcolor id= name=>  <hwvalue id= name=>  <hwvector id= name=>
    <texture name= unit= type= default=>  <texturelist ...>
    <window width=>  <tab>  <settab>  <resettabs>  <br>  <p>  <b>
    <if condition=...> ... <else> ... </if>

Attribute values appear in three forms: a DOUBLED-quoted localizable string
(label=""$$$/Key=English""), a plain quoted string, or a bare token. Bare
localizable text also appears between tags as ""$$$/Key=English"" or
"$$$/Key=English".

Nothing here executes IRIDAScript; the script text is captured verbatim as the
authoritative statement of what each shader computes.
"""
import os
import re

DOLLAR = re.compile(r'^\$\$\$/([^=]*)=(.*)$', re.S)

_TAG = re.compile(r"<(/?[A-Za-z][\w]*)((?:[^>\"]|\"[^\"]*\")*)>")

CONTROL_TAGS = {
    "slider": ("slider", "number"),
    "hwslider": ("hardware surface slider", "number"),
    "checkbox": ("checkbox", "boolean"),
    "colorselector": ("colour wheel / colour selector", "colour"),
    "hwcolor": ("hardware surface colour", "colour"),
    "curveeditor": ("curve editor", "curve"),
    "dropdown": ("dropdown", "enum"),
    "editbox": ("numeric entry box", "number"),
    "hwvalue": ("hardware surface value", "number"),
    "hwvector": ("hardware surface vector", "vector"),
    "rangecontrol": ("keying range control", "range"),
    "printerlights": ("printer lights control", "number triple"),
    "gang": ("gang / link toggle", "boolean"),
    "tabbar": ("tab bar", "enum"),
    "texture": ("texture input", "image"),
    "texturelist": ("texture list", "enum of images"),
    "extern": ("no UI; script input supplied by the host", "any"),
    "lut": ("LUT sampler binding", "lut"),
    "lookup": ("LUT lookup declaration", "lut"),
}
LAYOUT_TAGS = {"br", "p", "b", "tab", "settab", "resettabs", "window",
               "tabbar", "if", "else"}


def split_localized(v):
    if isinstance(v, str):
        m = DOLLAR.match(v)
        if m:
            return m.group(1), m.group(2)
    return None, v


def _parse_attrs(text):
    """Tokenise an attribute run, honouring the doubled-quote label form."""
    out = {}
    i, n = 0, len(text)
    while i < n:
        m = re.compile(r"\s*([A-Za-z_]\w*)\s*=").match(text, i)
        if not m:
            i += 1
            continue
        key = m.group(1)
        i = m.end()
        while i < n and text[i] == " ":
            i += 1
        if i < n and text[i] == '"':
            if text.startswith('""', i):
                j = text.find('""', i + 2)
                if j < 0:
                    j = n
                val = text[i + 2:j]
                i = j + 2
            else:
                j = text.find('"', i + 1)
                if j < 0:
                    j = n
                val = text[i + 1:j]
                i = j + 1
        else:
            m2 = re.compile(r"[^\s>]*").match(text, i)
            val = m2.group(0)
            i = m2.end()
        if key in out:
            k = key
            c = 2
            while k in out:
                k = "%s#%d" % (key, c)
                c += 1
            key = k
        out[key] = val
    return out


def _num(v):
    if v is None or v == "":
        return None
    try:
        f = float(v)
    except (TypeError, ValueError):
        return v
    return int(f) if f == int(f) and abs(f) < 1e15 else f


def parse_irtp(path):
    """Return {shader_name, controls[], script, ...} for one .irtp file."""
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        text = fh.read()

    script = None
    m = re.search(r"<IRIDAScript>(.*?)(?:</IRIDAScript>|\Z)", text, re.S)
    if m:
        script = m.group(1)
    head = text.split("<IRIDAScript>", 1)[0]

    shaders = []
    controls = []
    luts = []
    lookups = []
    tabs = []
    free_text = []
    conditionals = []
    current_tab = None

    pos = 0
    for tm in _TAG.finditer(head):
        # capture bare quoted text between tags (section labels)
        between = head[pos:tm.start()]
        pos = tm.end()
        for qm in re.finditer(r'"(\$\$\$/[^"]*)"', between):
            key, txt = split_localized(qm.group(1))
            free_text.append({"string_key": key, "text": txt,
                              "tab": current_tab})

        tag = tm.group(1).lower()
        attrs = _parse_attrs(tm.group(2) or "")

        if tag == "shader":
            rec = {"shader_name": attrs.get("name")}
            for k in ("noopwhendefault", "passes", "secondpass", "minPixelSize",
                      "minpixelsize"):
                if k in attrs:
                    rec[k.lower()] = _num(attrs[k])
            shaders.append(rec)
            continue
        if tag == "lut":
            luts.append({"name": attrs.get("name"), "unit": _num(attrs.get("unit"))})
            continue
        if tag == "lookup":
            lookups.append({
                "lut": attrs.get("lut"),
                "function": attrs.get("function"),
                "swizzle": attrs.get("swizzle"),
                "variables": [v for v in (attrs.get("variables") or "").split(",") if v],
            })
            continue
        if tag == "tabbar":
            tl = attrs.get("tabs") or ""
            key, txt = split_localized(tl)
            tabs.append({"name": attrs.get("name"),
                         "tabs": [t for t in (txt or "").split("|") if t],
                         "tabs_string_key": key,
                         "variables": attrs.get("variables")})
            continue
        if tag == "settab":
            current_tab = (current_tab or 0) + 1
            continue
        if tag == "resettabs":
            current_tab = None
            continue
        if tag == "if":
            conditionals.append({"condition": attrs.get("condition")})
            continue
        if tag in LAYOUT_TAGS or tag.startswith("/"):
            continue

        role, vkind = CONTROL_TAGS.get(tag, (None, None))
        lbl_key, lbl = split_localized(attrs.get("label"))
        rec = {
            "control": tag,
            "control_role": role,
            "value_kind": vkind,
            "name": attrs.get("name"),
            "hardware_id": _num(attrs.get("id")),
            "label": lbl,
            "label_string_key": lbl_key,
            "tab_index": current_tab,
        }
        for src, dst in (("min", "min"), ("max", "max"), ("default", "default"),
                         ("step", "step"), ("size", "ui_width"),
                         ("align", "ui_align"), ("orientation", "ui_orientation"),
                         ("mode", "mode"), ("shape", "shape"),
                         ("color", "colour"), ("mincolor", "min_colour"),
                         ("maxcolor", "max_colour"), ("elevation", "elevation"),
                         ("picker", "has_picker"), ("negative", "allows_negative"),
                         ("gang", "gang_group"), ("type", "value_type"),
                         ("unit", "texture_unit"), ("range", "range"),
                         ("tolerance", "tolerance"), ("invert", "invert"),
                         ("enable", "enable_var"), ("visible", "visible"),
                         ("noop", "no_op_when_default"),
                         ("forcemin", "clamp_at_min"),
                         ("forcemax", "clamp_at_max"),
                         ("forcerange", "clamp_to_range"),
                         ("reload", "forces_reload"),
                         ("colorselector", "linked_colorselector")):
            if src in attrs:
                rec[dst] = _num(attrs[src])
        if "items" in attrs:
            ik, itxt = split_localized(attrs["items"])
            rec["enum_options"] = [x for x in (itxt or "").split("|") if x]
            rec["enum_options_string_key"] = ik
        if "values" in attrs:
            rec["enum_values"] = [_num(x) for x in
                                  (attrs["values"] or "").split("|") if x != ""]
        if tag == "extern" and "default" in attrs:
            d = attrs["default"]
            if "," in str(d):
                rec["default"] = [_num(x) for x in str(d).split(",")]
        rec = {k: v for k, v in rec.items() if v is not None}
        controls.append(rec)

    return {
        "file": path,
        "shader_declarations": shaders,
        "shader_name": shaders[0]["shader_name"] if shaders else None,
        "lut_bindings": luts,
        "lut_lookups": lookups,
        "tab_bars": tabs,
        "section_labels": free_text,
        "conditional_blocks": conditionals,
        "controls": controls,
        "control_count": len(controls),
        "iridascript": script,
        "iridascript_lines": len(script.splitlines()) if script else 0,
    }


# ---------------------------------------------------------------------------
# .look files -- an XML shader stack with concrete parameter values
# ---------------------------------------------------------------------------
_LOOK_VAL = re.compile(r'^"?([A-Za-z]?)([-0-9.eE,]*)"?$')


def decode_look_value(raw):
    """'"D0"' -> (0, 'default'); '"N0.5"' -> (0.5, 'explicit'); '"0"' -> (0, ...).

    The one-letter prefix records whether the stored number is the shader's own
    default (D) or an explicitly set value (N). Values with no prefix are plain
    numbers. This mapping is DERIVED from the shipped files, not documented.
    """
    if raw is None:
        return None, None
    s = raw.strip().strip('"')
    if not s:
        return None, None
    tag = None
    if s and s[0].isalpha() and (len(s) == 1 or s[1] in "-.0123456789"):
        tag = s[0]
        s = s[1:]
    parts = [p for p in s.split(",") if p != ""]
    vals = []
    for p in parts:
        try:
            f = float(p)
        except ValueError:
            vals.append(p)
            continue
        vals.append(int(f) if f == int(f) and abs(f) < 1e15 else f)
    val = vals[0] if len(vals) == 1 else (vals or None)
    kind = {"D": "shader default", "N": "explicitly set"}.get(tag, "literal")
    return val, kind


def parse_look(path):
    import xml.etree.ElementTree as ET
    with open(path, "rb") as fh:
        raw = fh.read()
    raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", raw)
    root = ET.fromstring(raw)
    shaders = []
    for sh in root.iter("shader"):
        def txt(tag):
            e = sh.find(tag)
            return (e.text or "").strip().strip('"') if e is not None else None
        params = {}
        pe = sh.find("parameters")
        if pe is not None:
            for p in pe:
                v, kind = decode_look_value(p.text)
                params[p.tag] = {"value": v, "value_kind": kind}
        shaders.append({
            "name": txt("name"),
            "custom_name": txt("customname"),
            "visible": txt("visible"),
            "opacity": txt("opacity"),
            "mask": txt("mask"),
            "vectormask": txt("vectormask"),
            "linked": txt("linked"),
            "parameters": params,
            "parameter_count": len(params),
        })
    return {
        "file": path,
        "look_name": os.path.splitext(os.path.basename(path))[0],
        "shader_stack": shaders,
        "shader_count": len(shaders),
    }


# ---------------------------------------------------------------------------
# .cube LUT headers
# ---------------------------------------------------------------------------
def parse_cube_header(path, max_lines=40):
    info = {"format": "Iridas/Adobe .cube"}
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as fh:
            for i, line in enumerate(fh):
                if i > max_lines:
                    break
                s = line.strip()
                if not s or s.startswith("#"):
                    continue
                up = s.upper()
                if up.startswith("TITLE"):
                    info["title"] = s.split(None, 1)[1].strip().strip('"') if " " in s else None
                elif up.startswith("LUT_3D_SIZE"):
                    info["kind"] = "3D"
                    info["size"] = int(s.split()[1])
                elif up.startswith("LUT_1D_SIZE"):
                    info["kind"] = "1D"
                    info["size"] = int(s.split()[1])
                elif up.startswith("DOMAIN_MIN"):
                    info["domain_min"] = [float(x) for x in s.split()[1:]]
                elif up.startswith("DOMAIN_MAX"):
                    info["domain_max"] = [float(x) for x in s.split()[1:]]
                elif up.startswith("LUT_IN_VIDEO_RANGE"):
                    info["lut_in_video_range"] = True
    except Exception as exc:                          # noqa: BLE001
        info["parse_error"] = repr(exc)
    return info
