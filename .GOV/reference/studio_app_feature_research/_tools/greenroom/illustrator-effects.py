#!/usr/bin/env python
r"""illustrator-effects.py

Recover Illustrator's effect catalogue and each effect's parameter surface.

THREE INDEPENDENT PARSED CHANNELS, cross-joined:

 1. MENU REGISTRATION (what the Effect menu contains)
    Every effect plug-in embeds its menu registration as Adobe ZStrings:
        $$$/ZigZag/Str/Filter/1=&Distort              legacy Filter-menu group
        $$$/ZigZag/Str/Filter/2=&Zig Zag...           menu item
        $$$/ZigZag/Str/Filter/8=&Distort && Transform Effect-menu group
    Scanned from Plug-ins\**\*.aip (both ASCII and UTF-16LE encodings).

 2. DIALOG PARAMETERS (the controls, ranges, units, enumerations)
    Each effect ships a companion <Name>UI.aip whose dialog is stored as PLAIN
    TEXT in Adobe's EVE layout language.  ai_uidsl.py parses the `layout`
    (widget tree) and `sheet ... interface:` (numeric ranges) blocks and joins
    them, yielding control type, label, binding, slider/edit range, unit,
    decimal places and enumerated popup/radio values.

 3. SERIALISED LIVE EFFECTS (real parameter names, types and Adobe's own values)
    Illustrator's shipped Graphic Styles / Symbols / Swatches .ai libraries
    contain saved live effects inside %AI9_BeginArtStyles:
        /BasicFilter :
        (Adobe 3D Effect) 1 0 /Filter ,
        (3D.aip) /PluginFileName ,
        (3D Effect) /Title ,
        ($$$/3D/Menus/3DExtrude=Extrude and Bevel) /String (DisplayString) ,
        1 /Int (numLights) ,
        50 /Real (surfaceAmbient) ,
    Every `<value> /<Type> (<key>)` triple is a real parameter of that effect
    with a real value Adobe shipped.  Observed value sets are reported per key.

Reads files only.  Never launches Illustrator.
"""
from __future__ import annotations

import argparse
import collections
import datetime
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ai_private  # noqa: E402
import ai_uidsl  # noqa: E402

INSTALL_DEFAULT = r"C:\Program Files\Adobe\Adobe Illustrator 2026"

# ---------------------------------------------------------------- ZStrings
_RE_Z_A = re.compile(rb"\$\$\$/[ -~]{3,300}")
_RE_Z_W = re.compile(rb"\$\x00\$\x00\$\x00/\x00(?:[ -~]\x00){3,300}")
_RE_SPLIT = re.compile(r"(?=\$\$\$/)")
_RE_KV = re.compile(r"^\$\$\$/([^=]{1,220})=(.*)$", re.S)


def zstrings(data: bytes) -> dict[str, str]:
    """key -> value for every ZString literal in a binary.

    Adobe stores ZStrings back-to-back in a table, each preceded by a one-byte
    length.  When the table is read as one printable run, that length byte
    lands as a spurious trailing character on the PREVIOUS value:

        $$$/ZigZag/Str/Filter/1=&Distort#$$$/ZigZag/Str/Filter/2=&Zig Zag...
                                       ^-- 0x23 == 35 == len of the next entry

    Any fragment that is directly followed by another `$$$/` therefore has its
    final character dropped.  Trailing source punctuation is also trimmed when
    the literal was lifted out of embedded UI source.
    """
    out = {}
    raw = []
    for m in _RE_Z_A.finditer(data):
        raw.append(m.group().decode("latin-1"))
    for m in _RE_Z_W.finditer(data):
        raw.append(m.group().decode("utf-16-le", "replace"))
    for s in raw:
        parts = [p for p in _RE_SPLIT.split(s) if p.startswith("$$$/")]
        for i, part in enumerate(parts):
            m = _RE_KV.match(part)
            if not m:
                continue
            key, val = m.group(1), m.group(2)
            if i < len(parts) - 1 and val:
                val = val[:-1]          # drop the next entry's length byte
            # a literal lifted out of embedded UI source ends at the quote
            val = val.split('"')[0].split("\r")[0].split("\n")[0]
            val = val.rstrip("\x00").strip()
            if val and key not in out:
                out[key] = val
    return out


def clean_menu(s: str) -> str:
    """Strip Adobe accelerator markers and trailing ellipsis from a menu label."""
    s = s.replace("&&", "\x00").replace("&", "").replace("\x00", "&")
    return s.strip()


# Effect-menu group names Illustrator actually publishes.  Used to decide
# whether a `/Str/Filter/N=` value is a GROUP or an ITEM.
EFFECT_GROUPS = {
    "Distort & Transform", "Path", "Pathfinder", "Rasterize", "Stylize",
    "SVG Filters", "Warp", "3D and Materials", "3D", "Convert to Shape",
    "Crop Marks", "Artistic", "Blur", "Brush Strokes", "Distort", "Pixelate",
    "Sharpen", "Sketch", "Stylize (Photoshop)", "Texture", "Video",
    "Colors", "Create", "Ink Pen", "Effect Gallery", "Document Raster Effects Settings",
}

# Any Adobe string-table key: <Plugin>/Str[/<Section>]/<numeric id>
_RE_STRTAB = re.compile(
    r"^(?P<plugin>[^/]+)/(?:Str|STR)/(?:(?P<section>[A-Za-z]\w*)/)?(?P<id>\d+)$")


def is_menu_label(raw: str) -> bool:
    """A Windows menu string always carries an '&' accelerator marker.

    Requiring '&' separates real menu entries from the progress and undo
    strings that share the same string table ('Applying Zig Zag Filter...',
    'Warping...', 'Undo Feather').
    """
    if not raw or len(raw) > 60 or "%" in raw:
        return False
    return "&" in raw


def build_menu_index(menu_by_plugin: dict) -> tuple[dict, list]:
    """Group Effect-menu items under their group heading.

    DERIVED.  Adobe writes a menu group and the items beneath it as a
    contiguous run of ids inside one string-table section (e.g. Deform/Str/
    16401='&Warp' then 16402='&Arc...', 16403='&Arc Lower...', ...).  A value
    that matches a known Effect-menu group name opens a run; following ids in
    the same section that still look like menu labels belong to it, and the run
    closes at the first entry that does not.
    """
    index = collections.defaultdict(set)
    runs = []
    for rel, m in sorted(menu_by_plugin.items()):
        for sec, entries in m["sections"].items():
            ids = sorted(entries)
            cur_group, members = None, []
            for i in ids:
                raw = entries[i]
                clean = clean_menu(raw)
                if clean in EFFECT_GROUPS:
                    if cur_group and members:
                        runs.append({"plugin": m["plugin"], "section": sec,
                                     "group": cur_group, "items": members})
                        index[cur_group].update(members)
                    cur_group, members = clean, []
                    continue
                if cur_group is None:
                    continue
                if is_menu_label(raw):
                    label = clean.rstrip(".").strip()
                    if label and label not in members:
                        members.append(label)
                else:
                    if members:
                        runs.append({"plugin": m["plugin"], "section": sec,
                                     "group": cur_group, "items": members})
                        index[cur_group].update(members)
                    cur_group, members = None, []
            if cur_group and members:
                runs.append({"plugin": m["plugin"], "section": sec,
                             "group": cur_group, "items": members})
                index[cur_group].update(members)
    return {k: sorted(v) for k, v in sorted(index.items())}, runs

# ------------------------------------------------- serialized live effects
_RE_BASIC_FILTER = re.compile(
    rb"/BasicFilter\s*:(.*?)(?=/BasicFilter\s*:|/CompoundFilter\s*:|%AI9_EndArtStyles)",
    re.S)
_RE_PARAM = re.compile(
    rb"(?:^|\n)%?_?\s*(\(?[^\n(]*?\)?)\s*/(Int|Real|Bool|String|Enum)\s*\(([^)]*)\)\s*,",
    re.M)
_RE_TITLE = re.compile(rb"\(([^)]*)\)\s*/Title\s*,")
_RE_PLUGFILE = re.compile(rb"\(([^)]*)\)\s*/PluginFileName\s*,")
_RE_FILTERNAME = re.compile(rb"\(([^)]*)\)\s*\d+\s+\d+\s*/Filter\s*,")
_RE_DISPLAY = re.compile(rb"\(([^)]*)\)\s*/String\s*\(DisplayString\)\s*,")


def _dec(b: bytes) -> str:
    try:
        return b.decode("utf-8")
    except UnicodeDecodeError:
        return b.decode("latin-1")


def parse_live_effects(payload: bytes) -> list[dict]:
    a = payload.find(b"%AI9_BeginArtStyles")
    if a < 0:
        return []
    b = payload.find(b"%AI9_EndArtStyles", a)
    block = payload[a:b if b > 0 else len(payload)]
    out = []
    for m in _RE_BASIC_FILTER.finditer(block):
        seg = m.group(1)
        fn = _RE_FILTERNAME.search(seg)
        if not fn:
            continue
        rec = {"filter": _dec(fn.group(1))}
        t = _RE_TITLE.search(seg)
        if t:
            rec["title"] = _dec(t.group(1))
        pf = _RE_PLUGFILE.search(seg)
        if pf:
            rec["plugin_file"] = _dec(pf.group(1))
        ds = _RE_DISPLAY.search(seg)
        if ds:
            v = _dec(ds.group(1))
            rec["display_zstring"] = v
            rec["display"] = v.split("=", 1)[1] if v.startswith("$$$/") and "=" in v else v
        params = {}
        for pm in _RE_PARAM.finditer(seg):
            rawv, typ, key = pm.group(1), _dec(pm.group(2)), _dec(pm.group(3))
            v = _dec(rawv).strip()
            if v.startswith("(") and v.endswith(")"):
                v = v[1:-1]
            if typ == "Int":
                try:
                    v = int(v)
                except ValueError:
                    pass
            elif typ == "Real":
                try:
                    v = float(v)
                except ValueError:
                    pass
            elif typ == "Bool":
                v = v not in ("0", "false", "")
            params[key] = {"type": typ, "value": v}
        if params:
            rec["parameters"] = params
        out.append(rec)
    return out


# ---------------------------------------------------------------- main
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install", default=INSTALL_DEFAULT)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    plug_root = os.path.join(args.install, "Plug-ins")
    preset_root = os.path.join(args.install, "Presets", "en_US")

    # ---- channel 1 + 2: scan plug-in binaries
    aips = []
    for dp, _dn, fn in os.walk(plug_root):
        for f in fn:
            if f.lower().endswith(".aip"):
                aips.append(os.path.join(dp, f))
    aips.sort()

    menu_by_plugin = {}
    dialogs_by_plugin = {}
    zstring_total = 0
    for p in aips:
        rel = os.path.relpath(p, args.install).replace("\\", "/")
        base = os.path.splitext(os.path.basename(p))[0]
        with open(p, "rb") as fh:
            data = fh.read()
        zs = zstrings(data)
        zstring_total += len(zs)

        # string-table entries: <Plugin>/Str[/<Section>]/<numeric id>
        table = {}
        for k, v in zs.items():
            mm = _RE_STRTAB.match(k)
            if mm:
                sec = mm.group("section") or ""
                table.setdefault(sec, {})[int(mm.group("id"))] = v
        if table:
            menu_by_plugin[rel] = {"plugin": base, "sections": table}

        # dialog DSL
        if b"layout " in data or b"sheet " in data:
            try:
                res = ai_uidsl.extract(p)
            except Exception as exc:
                res = {"layouts": {}, "sheets": {}, "error": str(exc)}
            if res["layouts"]:
                lay = {}
                for lname, widgets in res["layouts"].items():
                    sheet = res["sheets"].get(lname) or (
                        next(iter(res["sheets"].values())) if res["sheets"] else {})
                    params = []
                    for w in widgets:
                        spec = ai_uidsl.parameter_spec(w, sheet)
                        if spec and (spec.get("label") or spec.get("identifier")):
                            params.append(spec)
                    lay[lname] = {
                        "widget_count": len(widgets),
                        "parameter_count": len(params),
                        "parameters": params,
                        "interface_symbols": sorted(sheet) if sheet else [],
                    }
                dialogs_by_plugin[rel] = {"plugin": base, "layouts": lay}

    # ---- channel 3: serialized live effects from every shipped .ai library
    live_files = []
    for dp, _dn, fn in os.walk(preset_root):
        for f in fn:
            if f.lower().endswith(".ai"):
                live_files.append(os.path.join(dp, f))
    live_files.sort()

    live_by_filter: dict[str, dict] = {}
    live_instances = 0
    live_failed = []
    for p in live_files:
        res = ai_private.extract(p)
        if res.error or not res.payload:
            live_failed.append({"path": os.path.relpath(p, preset_root),
                                "error": res.error})
            continue
        for eff in parse_live_effects(res.payload):
            live_instances += 1
            rec = live_by_filter.setdefault(eff["filter"], {
                "filter": eff["filter"], "titles": set(), "plugin_files": set(),
                "displays": set(), "instances": 0, "parameters": {}})
            rec["instances"] += 1
            if eff.get("title"):
                rec["titles"].add(eff["title"])
            if eff.get("plugin_file"):
                rec["plugin_files"].add(eff["plugin_file"])
            if eff.get("display"):
                rec["displays"].add(eff["display"])
            for k, pv in (eff.get("parameters") or {}).items():
                pr = rec["parameters"].setdefault(
                    k, {"type": pv["type"], "occurrences": 0, "observed_values": []})
                pr["occurrences"] += 1
                if pv["value"] not in pr["observed_values"] and \
                        len(pr["observed_values"]) < 24:
                    pr["observed_values"].append(pv["value"])

    live_out = {}
    for k, v in sorted(live_by_filter.items()):
        params = {}
        for pk, pv in sorted(v["parameters"].items()):
            vals = pv["observed_values"]
            entry = {"type": pv["type"], "occurrences": pv["occurrences"],
                     "distinct_values_seen": len(vals),
                     "observed_values": vals}
            nums = [x for x in vals if isinstance(x, (int, float))
                    and not isinstance(x, bool)]
            if len(nums) > 1:
                entry["observed_range"] = {"min": min(nums), "max": max(nums)}
            params[pk] = entry
        live_out[k] = {
            "filter": v["filter"],
            "titles": sorted(v["titles"]),
            "plugin_files": sorted(v["plugin_files"]),
            "display_names": sorted(v["displays"]),
            "instances_found": v["instances"],
            "parameter_count": len(params),
            "parameters": params,
        }

    # ---- build the effect-menu catalogue
    grouped, runs = build_menu_index(menu_by_plugin)
    catalogue = []
    for rel, m in sorted(menu_by_plugin.items()):
        secs = {}
        for sec, entries in m["sections"].items():
            secs[sec or "(root)"] = {str(i): entries[i] for i in sorted(entries)}
        catalogue.append({"plugin_path": rel, "plugin": m["plugin"],
                          "string_table_sections": secs,
                          "entry_count": sum(len(v) for v in secs.values())})

    now = datetime.datetime.now(datetime.timezone.utc).isoformat()
    out = {
        "schema_id": "handshake.studio.illustrator.effects.v1",
        "generated_at": now,
        "method": {
            "tool": "illustrator-effects.py",
            "install_root": args.install,
            "app_launched": False,
            "channels": {
                "menu_registration": "PARSED - ZString literals ($$$/<Plugin>/Str/"
                                     "Filter/<n>=<text>) scanned from every .aip in "
                                     "Plug-ins, ASCII and UTF-16LE",
                "dialog_parameters": "PARSED - Adobe EVE layout source embedded as "
                                     "plain text in the <Name>UI.aip binaries; "
                                     "`layout` widget tree joined to `sheet ... "
                                     "interface:` numeric ranges",
                "serialized_live_effects": "PARSED - %AI9_BeginArtStyles blocks in "
                                           "the shipped .ai libraries; each "
                                           "`<value> /<Type> (<key>)` triple is a "
                                           "real effect parameter with a value "
                                           "Adobe shipped",
            },
            "labelling": {
                "menu_items / menu_groups": "parsed text; accelerator '&' stripped "
                                            "(DERIVED cleanup)",
                "group-vs-item split": "DERIVED - a registration value is treated as "
                                       "a menu GROUP when it matches the known "
                                       "Effect-menu group list, else as an ITEM",
                "dialog parameter ranges/units/defaults": "parsed from the EVE source",
                "observed_values / observed_range": "PARSED values, but they are the "
                                                    "values present in Adobe's shipped "
                                                    "presets - they are NOT the "
                                                    "effect's legal min/max",
            },
        },
        "totals": {
            "plugins_scanned": len(aips),
            "plugins_with_menu_registration": len(menu_by_plugin),
            "plugins_with_dialog_source": len(dialogs_by_plugin),
            "zstrings_recovered": zstring_total,
            "dialog_layouts": sum(len(v["layouts"]) for v in dialogs_by_plugin.values()),
            "dialog_parameters": sum(
                l["parameter_count"] for v in dialogs_by_plugin.values()
                for l in v["layouts"].values()),
            "library_files_scanned_for_live_effects": len(live_files),
            "library_files_failed": len(live_failed),
            "live_effect_instances": live_instances,
            "distinct_live_effect_filters": len(live_out),
            "live_effect_parameter_keys": sum(
                v["parameter_count"] for v in live_out.values()),
            "effect_menu_groups": sorted(grouped),
            "effect_menu_items": sum(len(v) for v in grouped.values()),
            "string_table_entries": sum(c["entry_count"] for c in catalogue),
        },
        "effect_menu_index": grouped,
        "effect_menu_runs": runs,
        "string_tables": catalogue,
        "dialogs": dialogs_by_plugin,
        "serialized_live_effects": live_out,
        "live_effect_parse_failures": live_failed,
    }

    os.makedirs(args.out, exist_ok=True)
    fp = os.path.join(args.out, "illustrator_effects.json")
    with open(fp, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print(f"WROTE {fp} ({os.path.getsize(fp):,} bytes)")
    print(json.dumps(out["totals"], indent=1)[:2400])
    return 0


if __name__ == "__main__":
    sys.exit(main())
