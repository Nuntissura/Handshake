r"""ai_uidsl.py -- extract Adobe Illustrator's embedded declarative UI source.

Illustrator's .aip plug-ins are PE modules that carry their dialog and panel
definitions as PLAIN TEXT inside the binary.  The syntax is Adobe's internal
"EVE" layout language:

    layout RoughenUI
    {
        view subview(identifier: "AIEveSubView", layout_theme: @dialog_large, ...)
        {
            group(identifier: "RoughenOptionsGrp",
                  name: "$$$/RoughenUI/Dlg/OptionsGrp=Options")
            {
                combo_slider(min_max_filter: @sizeAbsRateFilter,
                             value_range: @sizeAbsEditRange,
                             identifier: "ComboSliderAbsSize",
                             decimal_places: 2,
                             unit: "CurrentDocumentUnit",
                             name: "$$$/RoughenUI/Dlg/Size=&Size:");
                radio_button(bind: @Mode, value: false,
                             name: ".../RelativeRadioButton=&Relative");
            }
        }
    }
    sheet RoughenUI
    {
      interface:
        sizeAbsRateFilter: {min_value: 0, max_value: 100};
        sizeAbsEditRange:  {min_value: 0, max_value: 7200};
        sizeAbsRateCell:   1;
    }

`layout` gives the widget tree (control type, label, identifier, binding, units,
precision, enumerated radio/popup values).  `sheet ... interface:` gives the
numeric ranges and initial cell values the widgets bind to.  Joining the two
yields a parameter's name, type, range, unit and default.

Labels are Adobe ZStrings: "$$$/<key>=<display text>"; `&` marks the keyboard
accelerator and is not part of the label.

Everything this module returns is PARSED from the shipped binaries.
Reads files only.  Never launches Illustrator.
"""
from __future__ import annotations

import re

# Contiguous printable-ASCII runs long enough to be source text.
_RE_RUN = re.compile(rb"[\t\r\n\x20-\x7E]{200,}")

_RE_BLOCK_HEAD = re.compile(r"\b(layout|sheet)\s+([A-Za-z_][\w.]*)\s*\{")

WIDGETS = {
    "combo_slider", "edit_text", "static_text", "checkbox", "radio_button",
    "radiogroup", "popup_menu", "popup", "button", "group", "panel", "column",
    "row", "overlay", "list_box", "color_swatch", "slider", "edit_number",
    "percent_edit", "angle_edit", "unit_edit", "text_edit", "combo_box",
    "dropdown", "view", "subview", "separator", "image_button", "tab_group",
    "spinner", "progress_bar", "link", "icon_button", "check_box",
}
# Widgets that carry a user-settable value (as opposed to pure layout).
VALUE_WIDGETS = {
    "combo_slider", "edit_text", "checkbox", "check_box", "radio_button",
    "popup_menu", "popup", "list_box", "color_swatch", "slider", "edit_number",
    "percent_edit", "angle_edit", "unit_edit", "text_edit", "combo_box",
    "dropdown", "spinner",
}

_RE_ZSTRING = re.compile(r"^\$\$\$/([^=]+)=(.*)$", re.S)


def zstring(value: str):
    """Split '$$$/Key=Display' -> (key, display_without_accelerator)."""
    if value is None:
        return None, None
    m = _RE_ZSTRING.match(value.strip())
    if not m:
        return None, value
    key, disp = m.group(1), m.group(2)
    disp = disp.replace("&&", "\x00").replace("&", "").replace("\x00", "&")
    return key, disp


def printable_runs(data: bytes) -> list[str]:
    return [m.group().decode("latin-1") for m in _RE_RUN.finditer(data)]


def _match_brace(text: str, open_idx: int) -> int:
    """Index just past the '}' matching the '{' at open_idx, honouring strings."""
    depth = 0
    i = open_idx
    n = len(text)
    while i < n:
        c = text[i]
        if c in "\"'":
            q = c
            i += 1
            while i < n and text[i] != q:
                if text[i] == "\\":
                    i += 1
                i += 1
        elif c == "/" and text[i:i + 2] == "//":
            j = text.find("\n", i)
            i = n if j < 0 else j
        elif c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return -1


def find_blocks(text: str) -> list[dict]:
    """Locate `layout X { ... }` and `sheet X { ... }` blocks."""
    out = []
    for m in _RE_BLOCK_HEAD.finditer(text):
        end = _match_brace(text, m.end() - 1)
        if end < 0:
            continue
        out.append({"kind": m.group(1), "name": m.group(2),
                    "body": text[m.end():end - 1]})
    return out


def _split_args(s: str) -> list[str]:
    parts, depth, cur = [], 0, []
    i, n = 0, len(s)
    while i < n:
        c = s[i]
        if c in "\"'":
            q = c
            cur.append(c)
            i += 1
            while i < n and s[i] != q:
                if s[i] == "\\":
                    cur.append(s[i])
                    i += 1
                cur.append(s[i])
                i += 1
            cur.append(q)
        elif c in "([{":
            depth += 1
            cur.append(c)
        elif c in ")]}":
            depth -= 1
            cur.append(c)
        elif c == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(c)
        i += 1
    if cur:
        parts.append("".join(cur))
    return [p.strip() for p in parts if p.strip()]


def _coerce(v: str):
    v = v.strip()
    if not v:
        return None
    if v[0] in "\"'" and v[-1:] == v[0] and len(v) > 1:
        return v[1:-1]
    if v.startswith("@"):
        return {"__ref__": v[1:]}
    low = v.lower()
    if low in ("true", "false"):
        return low == "true"
    try:
        return int(v)
    except ValueError:
        pass
    try:
        return float(v)
    except ValueError:
        pass
    if v.startswith("[") and v.endswith("]"):
        return [_coerce(x) for x in _split_args(v[1:-1])]
    if v.startswith("{") and v.endswith("}"):
        return _parse_kv(v[1:-1])
    return v


def _parse_kv(s: str) -> dict:
    out = {}
    for part in _split_args(s):
        if ":" in part:
            k, _, v = part.partition(":")
            k = k.strip()
            if re.fullmatch(r"[A-Za-z_]\w*", k):
                out[k] = _coerce(v)
    return out


_RE_CALL = re.compile(r"\b([a-z_][a-z0-9_]*)\s*\(", re.I)


def parse_widgets(body: str) -> list[dict]:
    """Every widget call in a `layout` block, with its attribute dict."""
    out = []
    for m in _RE_CALL.finditer(body):
        name = m.group(1)
        if name not in WIDGETS:
            continue
        # find matching close paren
        depth, i, n = 0, m.end() - 1, len(body)
        while i < n:
            c = body[i]
            if c in "\"'":
                q = c
                i += 1
                while i < n and body[i] != q:
                    if body[i] == "\\":
                        i += 1
                    i += 1
            elif c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        if i >= n:
            continue
        attrs = _parse_kv(body[m.end():i])
        out.append({"widget": name, "attrs": attrs, "offset": m.start()})
    return out


_RE_IFACE_ENTRY = re.compile(
    r"([A-Za-z_]\w*)\s*:\s*(\{[^}]*\}|\[[^\]]*\]|[^;{}\n]+)\s*;")


def parse_interface(body: str) -> dict:
    """`sheet X { interface: name: {min_value: 0, max_value: 100}; ... }`"""
    idx = body.find("interface:")
    seg = body[idx + len("interface:"):] if idx >= 0 else body
    seg = re.sub(r"//[^\n]*", "", seg)
    out = {}
    for m in _RE_IFACE_ENTRY.finditer(seg):
        out[m.group(1)] = _coerce(m.group(2))
    return out


def extract(path: str) -> dict:
    """Parse one .aip/.exe/.dll into {layouts: {...}, sheets: {...}}."""
    with open(path, "rb") as fh:
        data = fh.read()
    layouts, sheets = {}, {}
    for run in printable_runs(data):
        if "layout " not in run and "sheet " not in run:
            continue
        for blk in find_blocks(run):
            if blk["kind"] == "layout":
                layouts.setdefault(blk["name"], []).extend(
                    parse_widgets(blk["body"]))
            else:
                iface = parse_interface(blk["body"])
                if iface:
                    sheets.setdefault(blk["name"], {}).update(iface)
    return {"layouts": layouts, "sheets": sheets}


def resolve_ref(sheet: dict, ref):
    if isinstance(ref, dict) and "__ref__" in ref:
        return sheet.get(ref["__ref__"])
    return None


def parameter_spec(widget: dict, sheet: dict) -> dict | None:
    """Turn one widget into a parameter record, resolving ranges via the sheet."""
    a = widget["attrs"]
    if widget["widget"] not in VALUE_WIDGETS:
        return None
    key, label = zstring(a.get("name")) if isinstance(a.get("name"), str) else (None, None)
    rec = {
        "control": widget["widget"],
        "label": label,
        "label_zstring_key": key,
        "identifier": a.get("identifier") if isinstance(a.get("identifier"), str) else None,
    }
    b = a.get("bind")
    if isinstance(b, dict) and "__ref__" in b:
        rec["bind"] = b["__ref__"]
        init = sheet.get(b["__ref__"])
        if isinstance(init, (int, float, bool, str)):
            rec["initial_value"] = init
    for src, dst in (("min_max_filter", "slider_range"), ("value_range", "edit_range")):
        r = resolve_ref(sheet, a.get(src))
        if isinstance(r, dict):
            lo, hi = r.get("min_value"), r.get("max_value")
            if lo is not None or hi is not None:
                rec[dst] = {"min": lo, "max": hi}
            rec.setdefault("range_refs", {})[src] = a[src]["__ref__"]
        elif isinstance(a.get(src), dict) and "__ref__" in a[src]:
            rec.setdefault("range_refs_unresolved", {})[src] = a[src]["__ref__"]
    for k in ("unit", "decimal_places", "digits", "suffix"):
        if k in a:
            v = a[k]
            if k == "suffix" and isinstance(v, str):
                _, v = zstring(v)
            elif isinstance(v, dict) and "__ref__" in v:
                v = "@" + v["__ref__"]
            rec[k] = v
    if isinstance(a.get("items"), list):
        opts = []
        for it in a["items"]:
            if not isinstance(it, dict):
                continue
            nm = it.get("name")
            if nm == "-":
                opts.append({"separator": True})
                continue
            zk, disp = zstring(nm) if isinstance(nm, str) else (None, nm)
            o = {"label": disp, "zstring_key": zk}
            if "value" in it:
                o["value"] = it["value"]
            opts.append(o)
        rec["options"] = opts
        rec["option_count"] = sum(1 for o in opts if not o.get("separator"))
    if "value" in a:
        v = a["value"]
        rec["option_value"] = ("@" + v["__ref__"]) if isinstance(v, dict) and \
            "__ref__" in v else v
    return rec
