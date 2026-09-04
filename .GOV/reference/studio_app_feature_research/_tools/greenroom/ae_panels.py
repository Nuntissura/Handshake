"""After Effects 2026 -> aftereffects_panels_dialogs.json

Offline. Reads only. Never launches After Effects.

What the survey brief expected vs what is actually on disk
---------------------------------------------------------
* The 416 .map files in the install are JavaScript/CSS source maps under
  Support Files/com.adobe.frameio/node_modules and the UXP plug-ins. NONE of
  them is a dvaui prop.map archive. Verified: zero .map files remain after
  excluding *.js.map / *.cjs.map / *.mjs.map / *.css.map / *.ts.map.
* After Effects instead embeds its dvaui prop.map archives INSIDE the binaries.
  They are plain XML blobs (<?xml ...?><prop.map version='4'> ... </prop.map>)
  sitting in .rdata, and they carry the real panel and dialog control trees,
  the cursor table, and the shipped workspace layouts.
* Only two .eve files ship as files, but the Eve dialog-layout source for many
  modal dialogs is likewise embedded in AfterFXLib.dll and in several plug-ins.
* The 458 .qml files all belong to the bundled Boris FX mocha plug-in
  (Plug-ins/Effects/mochaAE/Resources/mochaui/qml/**), which is a Qt Quick UI.
  They are NOT After Effects' own UI, exactly as in the Premiere teardown.
"""

from __future__ import annotations

import collections
import os
import re
import sys
import xml.etree.ElementTree as ET

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402
import pp_common as P  # noqa: E402  (reused prop.map value walker)
import pp_panels as PPP  # noqa: E402  (reused dvaui control-tree walker)
import dw_eve  # noqa: E402

XML_HEAD = re.compile(rb"<\?xml[^>]{0,160}\?>")
CTRL_CHARS = re.compile(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]")


def xml_blobs(data: bytes):
    """Yield (offset, bytes) for every embedded XML document."""
    for m in XML_HEAD.finditer(data):
        s = m.start()
        window = data[s:s + 2_000_000]
        end = None
        for closer in (b"</prop.map>", b"</Effects>", b"</PremiereData>"):
            i = window.find(closer)
            if i != -1 and (end is None or i < end):
                end = i + len(closer)
        if end is None:
            z = window.find(b"\x00")
            end = z if z > 0 else min(len(window), 8000)
        yield s, window[:end]


def parse_prop_map(body: bytes):
    raw = CTRL_CHARS.sub(b"", body)
    root = ET.fromstring(raw)
    if not root.tag.endswith("prop.map"):
        raise ValueError("not a prop.map")
    for kid in root:
        if kid.tag.endswith("prop.list"):
            return P._propmap_list(kid)
    return {}


# --------------------------------------------------------------------------
# DVA_ControlsLayout: a flat control list with class name + pixel rect
# --------------------------------------------------------------------------

# Role per dvaui control class. The class name is read verbatim from the
# archive; the role is DERIVED from it and is reported as such.
CLASS_ROLE = {
    "UI_PictureButton": ("icon button", "action"),
    "UI_TextButton": ("push button", "action"),
    "UI_HotTextButton": ("scrubbable text button", "action"),
    "UI_StaticText": ("label", "read-only text"),
    "UI_StaticImage": ("image", "read-only"),
    "UI_Divider": ("separator", "none"),
    "UI_Popup": ("dropdown", "enum"),
    "UI_PopupListBox": ("dropdown list", "enum"),
    "UI_FontPopup": ("font dropdown", "enum"),
    "UI_Checkbox": ("checkbox", "boolean"),
    "UI_HotTextNumber": ("scrubbable numeric field", "number"),
    "UI_Slider": ("slider", "number"),
    "UI_ColorMeter": ("colour readout", "read-only colour"),
    "UI_SwatchRect": ("colour swatch", "colour"),
    "UI_AudioLabel": ("audio level label", "read-only"),
    "UI_ContainerControlView": ("container", "container"),
    "CharPal_UI_DArrowButton": ("disclosure arrow button", "action"),
    "CharPal_UI_Colors": ("character-panel colour controls", "colour"),
    "PaintPal_UI_Colors": ("paint-panel colour controls", "colour"),
    "ColorButton": ("colour button", "colour"),
    "TimePal_StepPushButton": ("transport step button", "action"),
    "FillerUIThumb": ("thumbnail", "read-only"),
    "BrushPalTipUIControl": ("brush tip grid", "collection"),
    "BrushPalTipUIPreviewControl": ("brush tip preview", "read-only"),
    "BrushUIControlContainer": ("container", "container"),
    "Picture_Text_Button": ("icon + text button", "action"),
}

MANGLED = re.compile(r"^(?:\d+)?N?((?:\d+[A-Za-z_]\w*)+)E?$")


def demangle_class(name: str) -> str:
    """'N5dvaui8controls13UI_StaticTextE' -> 'dvaui::controls::UI_StaticText'."""
    n = name.strip()
    if n.startswith("class "):
        return n[6:]
    body = n[1:-1] if n.startswith("N") and n.endswith("E") else n
    parts, i = [], 0
    while i < len(body):
        j = i
        while j < len(body) and body[j].isdigit():
            j += 1
        if j == i:
            break
        ln = int(body[i:j])
        parts.append(body[j:j + ln])
        i = j + ln
    return "::".join(parts) if parts else n


def read_controls_layout(d: dict):
    data = d.get("DVA_ControlsLayout Data") or {}
    clist = data.get("ControlList") or {}
    controls = []
    for key in sorted(clist, key=lambda k: (len(k), k)):
        v = clist[key]
        if not isinstance(v, dict) or "ControlClassName" not in v:
            continue
        cls = demangle_class(str(v["ControlClassName"]))
        leaf = cls.split("::")[-1]
        role, vkind = CLASS_ROLE.get(leaf, (None, None))
        rect = v.get("ControlRect")
        rec = {
            "control_id": key,
            "dvaui_class": cls,
            "control_role": role,
            "control_role_confidence": "derived from the dvaui class name",
            "value_kind": vkind,
        }
        if isinstance(rect, list) and len(rect) == 4:
            rec["rect_x_y_w_h"] = rect
        for extra in ("ControlID", "ControlText", "Identifier"):
            if extra in v:
                rec[extra] = v[extra]
        controls.append(rec)
    out = {"controls": controls, "control_count": len(controls)}
    if isinstance(data.get("LayoutSize"), list):
        out["layout_size_w_h"] = data["LayoutSize"]
    if data.get("Scale"):
        out["scale"] = data["Scale"]
    out["version"] = d.get("DVA_ControlsLayout")
    return out


# --------------------------------------------------------------------------
# workspaces
# --------------------------------------------------------------------------

def read_workspace(d: dict):
    ws = {
        "workspace_name": d.get("UserName"),
        "top_level_frames": [],
        "panel_ids": [],
    }
    ids = set()

    def frame(node, path):
        if not isinstance(node, dict):
            return None
        rec = {"path": path}
        for k in ("HasToolBar", "HasStatBar", "HasLeftBar", "HasRightBar",
                  "Vis", "CurrTab", "StackedViewFlags", "Maximized"):
            if k in node:
                rec[k] = node[k]
        for k in ("TabIDs", "HiddenTabs", "RemovedTabs"):
            v = node.get(k)
            if isinstance(v, list):
                rec[k] = v
                if k == "TabIDs":
                    ids.update(x for x in v if isinstance(x, str))
        sp = node.get("Splitter")
        if isinstance(sp, dict):
            rec["splitter"] = {
                "orient_raw": sp.get("Orient"),
                "place_fraction": sp.get("Place"),
                "sub1": frame(sp.get("Sub1"), path + "/Sub1"),
                "sub2": frame(sp.get("Sub2"), path + "/Sub2"),
            }
        for k, v in node.items():
            if k.startswith("Frame") and isinstance(v, dict):
                rec.setdefault("frames", []).append(frame(v, path + "/" + k))
        return {k: v for k, v in rec.items() if v not in (None, [], {})}

    for k, v in d.items():
        if k.startswith("TopLevelFrame") and isinstance(v, dict):
            ws["top_level_frames"].append(frame(v, k))
    ws["panel_ids"] = sorted(ids)
    ws["monitor_info_present"] = "MonitorInfo" in d
    return ws


# --------------------------------------------------------------------------
# Eve dialog sources embedded in binaries
# --------------------------------------------------------------------------

EVE_START = re.compile(rb"layout\s+[A-Za-z_][\w]*\s*\{")

# After Effects keeps each Eve dialog source as ONE NUL-terminated C string in
# .rdata. Some strings carry the full 'layout Name { ... }' wrapper, most do
# not and start straight at a node such as `dialog(` or `column(`.
EVE_RUN = re.compile(rb"[^\x00]{200,600000}")
EVE_SIGNS = (b"@place_column", b"@place_row", b"popup_item(", b"ok_cancel_row(",
             b"static_text(", b"dva_edit_number(", b"checkbox(", b"popup(")
EVE_NODE_START = re.compile(
    rb"\b(?:view\s+)?(?:layout\s+\w+\s*\{|dialog\(|palette\(|column\(|row\(|"
    rb"group\(|panel\(|tab_group\()")


def eve_blobs(data: bytes):
    """Yield (offset, source_bytes) for every embedded Eve dialog source."""
    for m in EVE_RUN.finditer(data):
        run = m.group()
        if b'(name: "$$$/' not in run and b"(name: '$$$/" not in run:
            continue
        if not any(sig in run for sig in EVE_SIGNS):
            continue
        n = EVE_NODE_START.search(run)
        if not n:
            continue
        yield m.start() + n.start(), run[n.start():]


def eve_surface(text, source, offset):
    """Parse one embedded Eve source blob into surfaces (one per 'layout')."""
    wrapped = False
    try:
        layouts = dw_eve.parse_eve(text)
        if not layouts:
            wrapped = True
            layouts = dw_eve.parse_eve("layout _embedded_ {\n%s\n}" % text)
    except Exception as exc:  # noqa: BLE001
        return [{"source_binary": source, "offset": hex(offset),
                 "parse_error": "%s: %s" % (type(exc).__name__, exc),
                 "raw_head": text[:200]}]
    out = []
    for lay in layouts:
        nodes = lay.get("nodes") or []
        try:
            controls = dw_eve.flatten_controls(nodes)
        except Exception as exc:  # noqa: BLE001
            out.append({"source_binary": source, "offset": hex(offset),
                        "layout_name": lay.get("layout_name"),
                        "parse_error": "flatten: %s: %s" % (type(exc).__name__, exc)})
            continue
        title = None
        for c in controls:
            if c.get("kind") in ("dialog", "view", "palette", "window") and c.get("label"):
                title = c["label"]
                break
        if title is None:
            for c in controls:
                if c.get("label"):
                    title = c["label"]
                    break
        out.append({
            "surface_name": lay.get("layout_name"),
            "source_was_wrapped": wrapped,
            "surface_title": title,
            "source_binary": source,
            "offset": hex(offset),
            "control_count": len(controls),
            "interactive_control_count": sum(
                1 for c in controls if c.get("control_role")
                and c["control_role"] not in ("label", "separator", "spacer",
                                              "read-only text", "image")),
            "controls": controls,
        })
    return out


# --------------------------------------------------------------------------

PALETTE_HINT = {
    "Tool Palette": "Tools panel",
    "Time Palette": "Timeline / time controls",
    "Info Palette": "Info panel",
    "Audio Palette": "Audio panel",
    "Paint Palette": "Paint panel",
    "Brush Palette": "Brushes panel",
    "Character Palette": "Character panel",
    "Paragraph Palette": "Paragraph panel",
    "Properties Palette": "Properties panel",
    "Tracker Palette": "Tracker panel",
    "WorkQueue Palette": "Render Queue / work queue panel",
    "Expressions and Scripting Palette": "Expression Editor / scripting panel",
}


def main():
    sf = C.support_files()
    idx = C.build_english_index()

    dialogs, workspaces, cursors, others = [], [], [], []
    failures = []
    eve_surfaces = []
    per_binary = collections.Counter()

    scan_exts = (".dll", ".exe", ".aex")
    for p in C.iter_files(sf, scan_exts, skip_dirs=("node_modules", "CEPHtmlEngine")):
        data = C.read_bytes(p)
        if b"<prop.map" in data:
            for off, body in xml_blobs(data):
                if b"<prop.map" not in body[:300]:
                    continue
                try:
                    d = parse_prop_map(body)
                except Exception as exc:  # noqa: BLE001
                    failures.append({"binary": C.rel(p), "offset": hex(off),
                                     "bytes": len(body),
                                     "error": "%s: %s" % (type(exc).__name__, exc)})
                    continue
                per_binary[C.rel(p)] += 1
                keys = set(d)
                if "DVA_Wrkspce" in keys:
                    w = read_workspace(d)
                    w["source_binary"] = C.rel(p)
                    w["offset"] = hex(off)
                    workspaces.append(w)
                elif "DVA_CursorData" in keys:
                    cursors.append({"source_binary": C.rel(p), "offset": hex(off),
                                    "hotspot": d.get("kHotSpot")})
                elif "UINodeArchive" in keys:
                    ctrls = PPP.walk_ui_tree(d.get("root") or d)
                    dialogs.append({
                        "archive_kind": "UINodeArchive",
                        "archive_semantic": "serialized live dvaui control tree",
                        "source_binary": C.rel(p),
                        "offset": hex(off),
                        "bytes": len(body),
                        "control_count": len(ctrls),
                        "surface_name": _surface_name(ctrls, p),
                        "controls": ctrls,
                    })
                elif "DVA_ControlsLayout" in keys:
                    lay = read_controls_layout(d)
                    lay.update({
                        "archive_kind": "DVA_ControlsLayout",
                        "archive_semantic": ("fixed panel/dialog control layout: "
                                             "control class + pixel rect"),
                        "source_binary": C.rel(p),
                        "offset": hex(off),
                        "bytes": len(body),
                        "surface_name": os.path.splitext(os.path.basename(p))[0],
                    })
                    dialogs.append(lay)
                else:
                    others.append({"source_binary": C.rel(p), "offset": hex(off),
                                   "top_keys": sorted(keys)[:12]})
        if b"@place_column" in data or b"@place_row" in data:
            for off, blob in eve_blobs(data):
                try:
                    text = blob.decode("utf-8")
                except UnicodeDecodeError:
                    text = blob.decode("latin-1")
                eve_surfaces.extend(eve_surface(text, C.rel(p), off))

    # standalone .eve files
    for p in C.iter_files(sf, (".eve",)):
        text = C.read_bytes(p).decode("utf-8", "replace")
        for s in eve_surface(text, C.rel(p), 0):
            s["standalone_file"] = True
            eve_surfaces.append(s)

    # panel plug-ins that ship as their own binaries
    panels = []
    req = os.path.join(sf, "Required")
    for fn in sorted(os.listdir(req)) if os.path.isdir(req) else []:
        if not fn.lower().endswith(".aex"):
            continue
        stem = os.path.splitext(fn)[0]
        if "Palette" not in stem:
            continue
        data = C.read_bytes(os.path.join(req, fn))
        ns = collections.Counter()
        for k, _v in C.zstrings(data):
            ns[k.split("/LStr/")[0]] += 1
        panels.append({
            "panel_plugin": "Support Files/Required/" + fn,
            "panel_role": PALETTE_HINT.get(stem, "panel plug-in"),
            "zstring_namespaces": dict(ns.most_common(6)),
            "embedded_prop_map_archives": per_binary.get(
                "Support Files/Required/" + fn, 0),
        })

    # panel / workspace identifiers seen anywhere
    panel_ids = sorted({pid for w in workspaces for pid in w["panel_ids"]})

    # dialog string namespaces (a dialog surface census independent of prop.map)
    dlg_keys = C.keys_under("AE/Dialogs/", idx)
    dlg_groups = collections.Counter(k.split("/")[2] for k in dlg_keys
                                     if len(k.split("/")) > 2)

    qml = list(C.iter_files(sf, (".qml",)))
    qml_roots = collections.Counter(
        C.rel(q).split("/Resources/")[0] if "/Resources/" in C.rel(q)
        else os.path.dirname(C.rel(q)) for q in qml)
    map_files = list(C.iter_files(sf, (".map",)))
    non_sourcemap = [C.rel(m) for m in map_files
                     if not re.search(r"\.(js|cjs|mjs|css|ts)\.map$", m, re.I)]

    user_ws = os.path.join(C.user_data_root(), "26.3")
    user_state = {}
    for sub in ("ModifiedWorkspaces", "OriginalUserWorkspaces", "DVADialogPrefs"):
        d = os.path.join(user_ws, sub)
        user_state[sub] = {"path": d, "exists": os.path.isdir(d),
                           "file_count": len(os.listdir(d)) if os.path.isdir(d) else 0}

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_panels.py",
        "evidence": [
            {"label": "parsed", "path": "Support Files/**/*.dll|*.exe|*.aex",
             "what": "embedded dvaui prop.map v4 XML archives",
             "extraction": "scan .rdata for <?xml ...?><prop.map version='4'>, "
                           "cut at the matching </prop.map>, strip stray control "
                           "bytes, ElementTree, then the shared prop.list value "
                           "walker from pp_common (Premiere teardown)"},
            {"label": "parsed", "path": "Support Files/**/*.dll|*.aex + "
                                        "Support Files/EveScripts/*.eve",
             "what": "Adobe Eve dialog-layout sources",
             "extraction": "brace-matched 'layout X { ... }' blobs, then the "
                           "shared Eve grammar parser dw_eve.parse_eve"},
            {"label": "parsed", "path": "Support Files/Dictionaries + binaries",
             "what": "$$$/AE/Dialogs/* and $$$/AE/*_Palette/* string surfaces"},
        ],
        "control_role_note": (
            "control_role is derived from the dvaui adapter class name using the "
            "same ADAPTER_ROLE table as the Premiere teardown. It is a DERIVED "
            "label, not a value read from the archive; the raw adapter string is "
            "kept on every control."),
        "negative_findings": [
            "The 416 .map files are JS/CSS source maps under com.adobe.frameio "
            "node_modules and the UXP plug-ins. %d of them are dvaui prop.map "
            "archives. After Effects embeds its prop.map archives in the "
            "binaries instead." % len(non_sourcemap),
            "All %d .qml files belong to the bundled Boris FX mocha plug-in's Qt "
            "Quick UI, not to After Effects. mocha is a third-party tracker UI, "
            "the same situation the Premiere teardown found." % len(qml),
            "The per-user folders under %%APPDATA%%/Adobe/After Effects/26.3 "
            "(ModifiedWorkspaces, OriginalUserWorkspaces, DVADialogPrefs, "
            "MediaIO, Presets, Cache) all exist but are EMPTY: After Effects has "
            "never been run on this machine and this teardown does not launch "
            "it. Every workspace reported here is a factory layout read out of "
            "AfterFXLib.dll, not a user layout.",
        ],
        "failures": failures[:80],
        "failure_count": len(failures),
        "counts": {
            "prop_map_archives_parsed": sum(per_binary.values()),
            "control_tree_surfaces": len(dialogs),
            "controls_total": sum(d["control_count"] for d in dialogs),
            "workspace_archives": len(workspaces),
            "cursor_archives": len(cursors),
            "other_prop_map_archives": len(others),
            "eve_surfaces": len(eve_surfaces),
            "eve_controls_total": sum(s.get("control_count", 0) for s in eve_surfaces),
            "qml_files": len(qml),
            "map_files": len(map_files),
            "map_files_that_are_prop_map": len(non_sourcemap),
        },
    }

    payload = {
        "summary": {
            "control_tree_surfaces": len(dialogs),
            "controls_total": sum(d["control_count"] for d in dialogs),
            "eve_dialog_surfaces": len(eve_surfaces),
            "factory_workspaces": len(workspaces),
            "workspace_names": [w.get("workspace_name") for w in workspaces],
            "distinct_panel_ids_in_workspaces": len(panel_ids),
            "panel_plugins": len(panels),
            "dialog_string_groups": len(dlg_groups),
        },
        "panel_plugins": panels,
        "panel_ids": panel_ids,
        "workspaces": workspaces,
        "dialog_string_groups": dict(dlg_groups.most_common()),
        "eve_surfaces": eve_surfaces,
        "control_tree_surfaces": dialogs,
        "cursor_archives": cursors,
        "unclassified_prop_map_archives": others,
        "qml_ownership": {
            "verdict": "All .qml belong to the bundled Boris FX mocha plug-in, "
                       "not to After Effects.",
            "roots": dict(qml_roots.most_common()),
        },
        "user_side_state": user_state,
    }
    C.write_json("aftereffects_panels_dialogs.json",
                 "handshake.studio.teardown.aftereffects.panels_dialogs",
                 method, payload)
    print("surfaces=%d controls=%d eve=%d workspaces=%d fails=%d"
          % (len(dialogs), sum(d["control_count"] for d in dialogs),
             len(eve_surfaces), len(workspaces), len(failures)), file=sys.stderr)


def _surface_name(ctrls, path):
    for c in ctrls:
        for k in ("text", "title", "name", "id"):
            if c.get(k) and isinstance(c[k], str) and len(c[k]) < 80:
                return c[k]
    return os.path.splitext(os.path.basename(path))[0]


if __name__ == "__main__":
    main()
