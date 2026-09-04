#!/usr/bin/env python
r"""illustrator-panels.py

Separate Illustrator's genuine UI panels from the service and machine-learning
manifests that a naive UXP scan lumps together.

CORRECTS THE PRIOR HARVEST: uxp_manifests.json reported "count: 46" as if that
were a panel list.  It is not.  23 of those 46 are ONNX model-weight manifests
under Support Files\Contents\Windows\weights\ (AutoHandles, Denoiser, OCR,
match_font, segmentation, ...).  They describe neural-network payloads and have
nothing to do with the UI.  Of the remaining 23 CEP/UXP extension manifests,
several target Photoshop or InDesign only, and several expose `command` or
`view` entry points rather than a dockable panel.

Illustrator's REAL panel set is overwhelmingly native C++ and never appears in
a UXP manifest at all.  Three parsed channels recover it:

 1. WORKSPACE LAYOUTS  Presets\en_US\Workspaces\<name> is Adobe's text
    "collection" format holding `/OWLBookMark [ <len> <hex> ]`.  The hex
    decodes to the workspace XML, whose `<palette app-data="...">`,
    `<toolbar app-data="...">` and `<control-bar app-data="...">` attributes
    are the panels' internal identifiers.

 2. WINDOW-MENU COMMANDS  The shipped keyboard-shortcut file lists every
    Window-menu panel toggle as a command id ("Adobe Color Palette",
    "AdobeLayerPalette1", "Adobe PathfinderUI", ...), including panels absent
    from every default workspace.

 3. PANEL TITLE STRINGS  ZStrings in the Plug-ins\Illustrator UI\*.aip binaries
    supply human-readable panel titles.

Reads files only.  Never launches Illustrator.
"""
from __future__ import annotations

import argparse
import binascii
import collections
import datetime
import json
import os
import re
import sys
import xml.etree.ElementTree as ET

INSTALL_DEFAULT = r"C:\Program Files\Adobe\Adobe Illustrator 2026"

_RE_OWL = re.compile(rb"/OWLBookMark\s*\[\s*(\d+)\s*(.*?)\]", re.S)
_RE_COLLNAME = re.compile(rb"/collectionName\s*\[\s*(\d+)\s*(.*?)\]", re.S)

# UXP manifests that live under this path tree are neural-network model
# descriptors, not UI.
ML_WEIGHTS_MARKER = os.path.join("Support Files", "Contents", "Windows", "weights")


def _hexblob(m) -> bytes:
    return binascii.unhexlify(re.sub(rb"\s+", b"", m.group(2)))


def parse_workspace(path: str) -> dict:
    data = open(path, "rb").read()
    rec = {"file": os.path.basename(path), "bytes": len(data),
           "surfaces": [], "error": None}
    m = _RE_COLLNAME.search(data)
    if m:
        try:
            rec["collection_name"] = _hexblob(m).decode("utf-8", "replace")
        except Exception:
            pass
    m = _RE_OWL.search(data)
    if not m:
        rec["error"] = "no_OWLBookMark"
        return rec
    try:
        xml = _hexblob(m)
    except Exception as exc:
        rec["error"] = f"hex_decode_failed:{exc}"
        return rec
    rec["owl_bytes"] = len(xml)
    try:
        root = ET.fromstring(xml.decode("utf-8", "replace"))
    except Exception as exc:
        rec["error"] = f"xml_parse_failed:{exc}"
        return rec
    for el in root.iter():
        ad = el.attrib.get("app-data")
        if not ad:
            continue
        rec["surfaces"].append({
            "surface_type": el.tag,
            "app_data": ad,
            "is_closed": el.attrib.get("is-closed"),
            "preferred_unconstrained_size":
                el.attrib.get("preferred-unconstrained-size"),
            "preferred_constrained_size":
                el.attrib.get("preferred-constrained-size"),
        })
    return rec


# ---------------------------------------------------------------- manifests
def classify_manifest(rel: str, man: dict) -> dict:
    is_ml = ML_WEIGHTS_MARKER.lower().replace("\\", "/") in rel.lower().replace("\\", "/")
    rec = {
        "path": rel.replace("\\", "/"),
        "id": man.get("id"),
        "name": man.get("name"),
        "version": man.get("version"),
        "manifest_version": man.get("manifestVersion"),
    }
    if is_ml:
        rec["manifest_class"] = "ml_model_weights"
        rec["is_ui_panel"] = False
        rec["reason"] = "lives under Support Files/Contents/Windows/weights; " \
                        "describes a neural-network model payload"
        return rec

    hosts = man.get("host")
    host_list = hosts if isinstance(hosts, list) else ([hosts] if hosts else [])
    apps = [h.get("app") for h in host_list if isinstance(h, dict) and h.get("app")]
    rec["host_apps"] = apps
    rec["hosts_illustrator"] = "AI" in apps
    rec["illustrator_min_version"] = next(
        (h.get("minVersion") for h in host_list
         if isinstance(h, dict) and h.get("app") == "AI"), None)

    # entrypoints may be top level or nested inside the AI host entry
    eps = []
    for e in (man.get("entrypoints") or []):
        if isinstance(e, dict):
            eps.append((e, "manifest"))
    for h in host_list:
        if isinstance(h, dict) and h.get("app") == "AI":
            for e in (h.get("entrypoints") or []):
                if isinstance(e, dict):
                    eps.append((e, "host:AI"))

    def lbl(e):
        v = e.get("label")
        if isinstance(v, dict):
            v = v.get("default")
        if isinstance(v, str) and v.startswith("$$$/") and "=" in v:
            return v.split("=", 1)[1]
        return v

    rec["entrypoints"] = [{
        "type": e.get("type"), "id": e.get("id"), "label": lbl(e),
        "label_raw": e.get("label") if isinstance(e.get("label"), str) else None,
        "declared_in": src,
        "minimum_size": e.get("minimumSize"),
        "maximum_size": e.get("maximumSize"),
        "preferred_docked_size": e.get("preferredDockedSize"),
        "preferred_floating_size": e.get("preferredFloatingSize"),
    } for e, src in eps]
    panels = [e for e in rec["entrypoints"] if e["type"] == "panel"]
    rec["panel_entrypoint_count"] = len(panels)
    rec["manifest_class"] = (
        "extension_panel" if panels else
        "extension_command_or_service" if rec["entrypoints"] else
        "extension_no_entrypoints")
    rec["is_ui_panel"] = bool(panels) and rec["hosts_illustrator"]
    if not rec["is_ui_panel"]:
        rec["reason"] = (
            "no panel entrypoint" if not panels else
            "declares a panel but does not list AI as a host app")
    return rec


# --------------------------------------------------------------- ZStrings
_RE_Z_A = re.compile(rb"\$\$\$/[ -~]{3,300}")
_RE_Z_W = re.compile(rb"\$\x00\$\x00\$\x00/\x00(?:[ -~]\x00){3,300}")
_RE_SPLIT = re.compile(r"(?=\$\$\$/)")
_RE_KV = re.compile(r"^\$\$\$/([^=]{1,220})=(.*)$", re.S)
_RE_PANEL_KEY = re.compile(
    r"(?:^|/)(?:[A-Za-z0-9_]*)(?:Panel|Palette|Pallete)(?:[A-Za-z0-9_]*)"
    r"(?:/[A-Za-z0-9_]+)*?/(Title|DefaultPanelTitle|PanelTitle|TabTitle|"
    r"TabName|PanelName|Name|Label|Caption|Header)$", re.I)


def panel_titles(path: str) -> dict[str, str]:
    data = open(path, "rb").read()
    raw = [m.group().decode("latin-1") for m in _RE_Z_A.finditer(data)]
    raw += [m.group().decode("utf-16-le", "replace") for m in _RE_Z_W.finditer(data)]
    out = {}
    for s in raw:
        parts = [p for p in _RE_SPLIT.split(s) if p.startswith("$$$/")]
        for i, part in enumerate(parts):
            m = _RE_KV.match(part)
            if not m:
                continue
            key, val = m.group(1), m.group(2)
            if i < len(parts) - 1 and val:
                val = val[:-1]
            val = val.split('"')[0].split("\r")[0].split("\n")[0].strip()
            if val and _RE_PANEL_KEY.search(key):
                out.setdefault(key, val)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--install", default=INSTALL_DEFAULT)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    R = args.install

    # ---- channel 1: workspaces
    ws_dir = os.path.join(R, "Presets", "en_US", "Workspaces")
    workspaces = []
    if os.path.isdir(ws_dir):
        for f in sorted(os.listdir(ws_dir)):
            p = os.path.join(ws_dir, f)
            if os.path.isfile(p):
                workspaces.append(parse_workspace(p))

    surf_index = collections.defaultdict(
        lambda: {"surface_types": set(), "workspaces": set(), "occurrences": 0})
    for w in workspaces:
        for s in w["surfaces"]:
            e = surf_index[s["app_data"]]
            e["surface_types"].add(s["surface_type"])
            e["workspaces"].add(w["file"])
            e["occurrences"] += 1

    # ---- channel 2: Window-menu panel commands from the shortcut file
    kys = os.path.join(R, "Presets", "en_US", "Keyboard Shortcuts",
                       "Illustrator Defaults.kys")
    menu_cmds = []
    if os.path.exists(kys):
        text = open(kys, "rb").read().decode("latin-1")
        sm = re.search(r"^/Menus\s*\{\r?\n(.*?)^\}", text, re.S | re.M)
        body = sm.group(1) if sm else ""
        for m in re.finditer(r"^\t/((?:\\.|[^\s{])+)\s*\{", body, re.M):
            menu_cmds.append(re.sub(r"\\(.)", r"\1", m.group(1)))
    PANEL_WORDS = re.compile(r"(palette|panel)", re.I)
    panel_cmds = sorted({c for c in menu_cmds if PANEL_WORDS.search(c)})

    # ---- channel 3: panel titles from the UI plug-ins
    ui_dir = os.path.join(R, "Plug-ins", "Illustrator UI")
    titles = {}
    if os.path.isdir(ui_dir):
        for f in sorted(os.listdir(ui_dir)):
            if f.lower().endswith(".aip"):
                for k, v in panel_titles(os.path.join(ui_dir, f)).items():
                    titles.setdefault(k, {"title": v, "plugin": f})

    # ---- manifests
    manifests = []
    for base in [os.path.join("Support Files", "Contents", "Windows", "weights"),
                 os.path.join("Support Files", "Required", "UXP", "extensions"),
                 os.path.join("Support Files", "Required", "CEP", "extensions")]:
        root = os.path.join(R, base)
        if not os.path.isdir(root):
            continue
        for dp, _dn, fn in os.walk(root):
            if "manifest.json" not in fn:
                continue
            p = os.path.join(dp, "manifest.json")
            try:
                man = json.load(open(p, encoding="utf-8-sig"))
            except Exception as exc:
                manifests.append({"path": os.path.relpath(p, R).replace("\\", "/"),
                                  "manifest_class": "unreadable",
                                  "is_ui_panel": False, "error": str(exc)})
                continue
            manifests.append(classify_manifest(os.path.relpath(p, R), man))

    ai_panels_raw = [m for m in manifests if m.get("is_ui_panel")]
    by_class = collections.Counter(m["manifest_class"] for m in manifests)
    ai_panels = [{
        "id": m["id"], "name": m["name"], "path": m["path"],
        "min_version": m.get("illustrator_min_version"),
        "panels": [e for e in m["entrypoints"] if e["type"] == "panel"],
    } for m in ai_panels_raw]

    native = []
    for ad, e in sorted(surf_index.items()):
        native.append({
            "app_data_id": ad,
            "surface_types": sorted(e["surface_types"]),
            "in_workspaces": sorted(e["workspaces"]),
            "workspace_count": len(e["workspaces"]),
            "is_extension_host": ad.startswith("CSXSExtension_")
                                 or "UXP" in ad,
        })

    # ---- consolidated catalogue: one row per dockable UI surface
    ext_ids = {m["id"] for m in ai_panels if m.get("id")}
    ext_names = {m.get("name") for m in ai_panels if m.get("name")}
    catalogue = []
    for n in native:
        ad = n["app_data_id"]
        origin = "native"
        if n["is_extension_host"]:
            origin = "extension_host_slot"
        elif ad in ext_names or ad in ext_ids:
            origin = "extension"
        catalogue.append({
            "surface_id": ad,
            "surface_type": n["surface_types"][0] if n["surface_types"] else None,
            "origin": origin,
            "evidence": ["workspace_layout"],
            "workspace_count": n["workspace_count"],
            "in_default_workspaces": n["in_workspaces"],
            "docked_by_default": n["workspace_count"] > 0,
        })
    seen_ids = {c["surface_id"] for c in catalogue}
    for m in ai_panels:
        for p in m["panels"]:
            sid = p.get("label") or p.get("id") or m.get("name")
            if sid in seen_ids:
                for c in catalogue:
                    if c["surface_id"] == sid:
                        c["evidence"].append("extension_manifest")
                        c["extension_id"] = m["id"]
                continue
            catalogue.append({
                "surface_id": sid,
                "surface_type": "palette",
                "origin": "extension",
                "evidence": ["extension_manifest"],
                "extension_id": m["id"],
                "extension_path": m["path"],
                "workspace_count": 0,
                "docked_by_default": False,
                "minimum_size": p.get("minimum_size"),
                "maximum_size": p.get("maximum_size"),
                "preferred_docked_size": p.get("preferred_docked_size"),
            })
            seen_ids.add(sid)
    catalogue.sort(key=lambda c: (c["origin"], str(c["surface_id"]).lower()))

    out = {
        "schema_id": "handshake.studio.illustrator.panels.v1",
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "method": {
            "tool": "illustrator-panels.py",
            "install_root": R,
            "app_launched": False,
            "claim_corrected": "the prior harvest's uxp_manifests.json count of 46 "
                               "is not a panel list; it mixes neural-network model "
                               "manifests with CEP/UXP extensions, and most "
                               "Illustrator panels are native and appear in none "
                               "of them",
            "channels": {
                "workspace_layouts": "PARSED - Presets/en_US/Workspaces/<name>, "
                                     "/OWLBookMark hex -> workspace XML -> "
                                     "app-data identifiers",
                "window_menu_commands": "PARSED - panel toggle command ids in "
                                        "Presets/en_US/Keyboard Shortcuts/"
                                        "Illustrator Defaults.kys",
                "panel_title_strings": "PARSED - ZStrings in "
                                       "Plug-ins/Illustrator UI/*.aip",
                "extension_manifests": "PARSED - manifest.json entrypoints and host "
                                       "app declarations",
            },
            "labelling": {
                "app_data_id / entrypoints / titles": "parsed",
                "manifest_class and is_ui_panel": "DERIVED from the parsed path, "
                                                  "host app list and entrypoint "
                                                  "types",
                "window_menu panel command selection": "DERIVED - command ids "
                                                       "matching /palette|panel/i",
            },
        },
        "totals": {
            "manifests_scanned": len(manifests),
            "manifests_by_class": dict(by_class),
            "ml_model_manifests": by_class.get("ml_model_weights", 0),
            "extension_manifests": len(manifests) - by_class.get("ml_model_weights", 0),
            "extension_panels_hosted_in_illustrator": len(ai_panels),
            "workspaces_parsed": len(workspaces),
            "workspaces_failed": sum(1 for w in workspaces if w["error"]),
            "distinct_workspace_surfaces": len(surf_index),
            "window_menu_commands_total": len(menu_cmds),
            "window_menu_panel_commands": len(panel_cmds),
            "panel_title_strings": len(titles),
            "panel_catalogue_rows": len(catalogue),
            "panel_catalogue_by_origin": dict(
                collections.Counter(c["origin"] for c in catalogue)),
            "panel_catalogue_by_surface_type": dict(
                collections.Counter(c["surface_type"] for c in catalogue)),
        },
        "panel_catalogue": catalogue,
        "extension_panels_in_illustrator": ai_panels,
        "manifests": manifests,
        "workspace_surfaces": native,
        "workspaces": workspaces,
        "window_menu_panel_commands": panel_cmds,
        "window_menu_commands": sorted(menu_cmds),
        "panel_title_strings": dict(sorted(titles.items())),
    }

    os.makedirs(args.out, exist_ok=True)
    fp = os.path.join(args.out, "illustrator_panels.json")
    with open(fp, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print(f"WROTE {fp} ({os.path.getsize(fp):,} bytes)")
    print(json.dumps(out["totals"], indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
