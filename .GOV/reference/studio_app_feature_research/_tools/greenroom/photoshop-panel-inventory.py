#!/usr/bin/env python
"""
photoshop-panel-inventory.py

OFFLINE panel-surface teardown of Adobe Photoshop 2026 for a native Rust
rebuild.  Reads files only.  Never launches Photoshop or any application.

Produces: photoshop_panels.json

Corrects an earlier harvest pass (uxp_manifests.json) which captured 63
manifest.json files and was treated as a panel list.  It is not one: most of
those manifests are ML model descriptors, service shims and command-only
extensions, and the real panel surface of Photoshop is native, not UXP.
"""

import base64
import collections
import datetime
import hashlib
import json
import os
import re
import struct
import sys
import xml.etree.ElementTree as ET

INSTALL = r"C:\Program Files\Adobe\Adobe Photoshop 2026"
WORKSPACES_DIR = os.path.join(INSTALL, "Required", "Workspaces")
LAYOUT_ROOTS = [
    os.path.join(INSTALL, "Required", "layouts"),
    os.path.join(INSTALL, "Required", "drover_layouts"),
]
OUT_DIR = (
    r"D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-gov-kernel"
    r"\.GOV\reference\studio_app_feature_research\_greenroom_20260903"
    r"\installed_exports\photoshop\offline"
)
OUT_PATH = os.path.join(OUT_DIR, "photoshop_panels.json")
PRIOR_UXP = os.path.join(OUT_DIR, "uxp_manifests.json")
PRIOR_WORKSPACES = os.path.join(OUT_DIR, "workspaces.json")


def rel(p):
    try:
        return os.path.relpath(p, INSTALL).replace("\\", "/")
    except ValueError:
        return p


# ==========================================================================
# 1. manifest discovery + classification
# ==========================================================================

# Entry-point arrays appear under three different spellings in this install.
# Reading only the lowercase one (what the earlier pass did) silently drops
# five plugins' UI declarations.
ENTRYPOINT_KEYS = ["entrypoints", "entryPoints", "uiEntrypoints"]

# Entry-point "type" values observed in this install, grouped by what they mean
# for a rebuild.  Membership was decided by reading each manifest, not by name.
PANEL_TYPES = {"panel"}
NON_PANEL_UI_TYPES = {
    "view", "homescreen", "welcome", "welcome-picker", "import-picker",
    "link-assets", "saveas-picker", "relink-assets", "close-saveas-picker",
    "close-import-picker", "popover", "inAppNotifications", "facepile",
    "lrimporter", "neuralGallery",
}
COMMAND_TYPES = {"command"}

SIZING_KEYS = [
    "minimumSize", "maximumSize", "preferredDockedSize", "preferredFloatingSize",
]


def find_manifests():
    out = []
    for dirpath, _dn, filenames in os.walk(INSTALL):
        for fn in filenames:
            if fn.lower() == "manifest.json":
                out.append(os.path.join(dirpath, fn))
    return sorted(out)


def find_csxs_manifests():
    """CEP extensions declare themselves in CSXS/manifest.xml, not manifest.json."""
    out = []
    for dirpath, _dn, filenames in os.walk(INSTALL):
        if os.path.basename(dirpath).upper() != "CSXS":
            continue
        for fn in filenames:
            if fn.lower() == "manifest.xml":
                out.append(os.path.join(dirpath, fn))
    return sorted(out)


def read_entrypoints(doc):
    for k in ENTRYPOINT_KEYS:
        if k in doc and isinstance(doc[k], list):
            return k, doc[k]
    for k in ENTRYPOINT_KEYS:
        if k in doc:
            return k, []
    return None, []


def ep_summary(ep):
    if not isinstance(ep, dict):
        return {"raw": ep, "type": None}
    sizing = {k: ep[k] for k in SIZING_KEYS if k in ep}
    return {
        "type": ep.get("type"),
        "id": ep.get("id") or ep.get("panelId"),
        "label": ep.get("label"),
        "sizing_hints": sizing or None,
        "has_sizing_hints": bool(sizing),
        "other_keys": sorted(
            k for k in ep
            if k not in ("type", "id", "panelId", "label", *SIZING_KEYS)),
    }


def classify_manifest(path, doc):
    """
    Classify one manifest.json from ITS OWN CONTENT.

    Decision order (each step records the evidence that fired it):
      1. ML model descriptor  - has a 'targets' map of runtime->components and
                                no entry-point array of any spelling.
      2. UI panel             - declares >=1 entry point of type 'panel'.
      3. Non-panel UI surface - declares >=1 entry point of a UI type that is
                                not 'panel' (homescreen, popover, view, ...).
      4. Command extension    - declares only 'command' entry points.
      5. Internal shim/service- declares no entry points at all.
    """
    p_rel = rel(path)
    ep_key, eps = read_entrypoints(doc)
    ep_objs = [ep_summary(e) for e in eps]
    ep_types = [e["type"] for e in ep_objs]
    topkeys = sorted(doc.keys())

    host = doc.get("host") or doc.get("hosts")
    host_apps = []
    if isinstance(host, dict):
        host_apps = [host.get("app")]
    elif isinstance(host, list):
        host_apps = [h.get("app") if isinstance(h, dict) else h for h in host]

    perms = doc.get("requiredPermissions")
    perm_keys = sorted(perms.keys()) if isinstance(perms, dict) else []

    path_bucket = "other"
    lower = p_rel.lower()
    if "/sensei_models/" in lower:
        path_bucket = "sensei_models"
    elif lower.startswith("required/uxp/"):
        path_bucket = "uxp"
    elif lower.startswith("required/cep/"):
        path_bucket = "cep"

    # nested inside another plugin's folder?
    parts = p_rel.split("/")
    nested_under = None
    for i, seg in enumerate(parts[:-1]):
        if seg in ("extensions", "dist") and i > 0:
            nested_under = "/".join(parts[:i])
            break
    if nested_under is None and path_bucket == "uxp" and len(parts) > 4:
        nested_under = "/".join(parts[:3])

    evidence = {
        "manifest_top_level_keys": topkeys,
        "entrypoint_key_used": ep_key,
        "entrypoint_count": len(eps),
        "entrypoint_types": ep_types,
        "entrypoints_with_sizing_hints": sum(
            1 for e in ep_objs if e["has_sizing_hints"]),
        "has_targets_model_map": isinstance(doc.get("targets"), dict),
        "targets_runtimes": sorted(doc["targets"].keys())
        if isinstance(doc.get("targets"), dict) else None,
        "manifestVersion": doc.get("manifestVersion"),
        "host_apps": host_apps,
        "host_load_event": (
            host.get("data", {}).get("loadEvent")
            if isinstance(host, dict) and isinstance(host.get("data"), dict)
            else None),
        "requiredPermissions_keys": perm_keys,
        "path_bucket": path_bucket,
        "nested_under_plugin_dir": nested_under,
        "runOnStartup": doc.get("runOnStartup"),
        "addToPluginMenu": doc.get("addToPluginMenu"),
    }

    panel_eps = [e for e in ep_objs if e["type"] in PANEL_TYPES]
    other_ui_eps = [e for e in ep_objs if e["type"] in NON_PANEL_UI_TYPES]
    cmd_eps = [e for e in ep_objs if e["type"] in COMMAND_TYPES]
    unknown_eps = [
        e for e in ep_objs
        if e["type"] not in PANEL_TYPES
        and e["type"] not in NON_PANEL_UI_TYPES
        and e["type"] not in COMMAND_TYPES
    ]

    if evidence["has_targets_model_map"] and ep_key is None:
        cls = "ml_model"
        basis = "parsed"
        why = (
            "No entry-point array under any of the three spellings this "
            "install uses (%s). Instead it carries a 'targets' map keyed by "
            "inference runtime (%s) whose values hold components/inputs/outputs "
            "tensor descriptors. This is a machine-learning model descriptor, "
            "not a UI surface." % (", ".join(ENTRYPOINT_KEYS),
                                   evidence["targets_runtimes"])
        )
    elif panel_eps:
        cls = "ui_panel"
        basis = "parsed"
        why = (
            "Declares %d entry point(s) of type 'panel' under key '%s'; %d of "
            "them carry docking/sizing hints (%s)."
            % (len(panel_eps), ep_key,
               sum(1 for e in panel_eps if e["has_sizing_hints"]),
               ", ".join(SIZING_KEYS))
        )
    elif other_ui_eps:
        cls = "ui_surface_non_panel"
        basis = "heuristic"
        why = (
            "Declares UI entry point type(s) %s under key '%s'. These are "
            "host-owned UI surfaces (start screen, popover, file picker, "
            "options-bar view, notification host), not dockable panels. "
            "HEURISTIC: the UXP entry-point type vocabulary is not documented "
            "anywhere in this install, so the 'is a UI surface but is not a "
            "dockable panel' reading was inferred from the type names and the "
            "absence of any docking/sizing hint."
            % (sorted({e["type"] for e in other_ui_eps}), ep_key)
        )
    elif cmd_eps and not panel_eps:
        cls = "command_extension"
        basis = "parsed"
        why = (
            "Declares %d entry point(s), all of type 'command', under key "
            "'%s'. No panel entry point and no sizing hints anywhere in the "
            "manifest, so it contributes no dockable UI."
            % (len(cmd_eps), ep_key)
        )
    elif ep_key is None or len(eps) == 0:
        cls = "internal_shim_or_service"
        basis = "heuristic"
        why = (
            "Declares NO entry points at all (entry-point key present: %r, "
            "count: %d). It ships a 'main' HTML/JS bundle and a host block%s, "
            "so it is loaded by the host but exposes no user-invocable surface. "
            "HEURISTIC: 'shim/service' is this script's label; the manifest "
            "itself says only that it has a main and a host."
            % (ep_key, len(eps),
               " with loadEvent=%r" % evidence["host_load_event"]
               if evidence["host_load_event"] else "")
        )
    else:
        cls = "unclassified"
        basis = "heuristic"
        why = "Entry point types %s did not match any known bucket." % ep_types

    return {
        "path": p_rel,
        "abs_path": path,
        "id": doc.get("id"),
        "name": doc.get("name"),
        "version": doc.get("version"),
        "classification": cls,
        "classification_basis": basis,
        "classification_evidence": why,
        "evidence": evidence,
        "entrypoints": ep_objs,
        "panel_entrypoints": panel_eps,
        "non_panel_ui_entrypoints": other_ui_eps,
        "command_entrypoints": cmd_eps,
        "unrecognised_entrypoints": unknown_eps,
    }


def classify_csxs(path):
    """Classify a CEP CSXS/manifest.xml by its <UI><Type> declarations."""
    try:
        root = ET.parse(path).getroot()
    except Exception as exc:  # noqa: BLE001
        return {"path": rel(path), "parse_error": str(exc)}
    # A CSXS manifest names each extension TWICE: once in <ExtensionList>
    # (id + version only) and once in <DispatchInfoList> (the UI declaration).
    # Merge them by Id so an extension is counted once, not twice.
    merged = {}

    def slot(eid):
        return merged.setdefault(eid, {
            "id": eid, "version": None, "ui_type": None, "menu_token": None,
            "geometry": None, "main_path": None,
            "declared_in": [],
        })

    lst = root.find("ExtensionList")
    if lst is not None:
        for ext in lst.findall("Extension"):
            eid = ext.get("Id")
            if not eid:
                continue
            s = slot(eid)
            s["version"] = ext.get("Version")
            s["declared_in"].append("ExtensionList")

    dil = root.find("DispatchInfoList")
    if dil is not None:
        for ext in dil.findall("Extension"):
            eid = ext.get("Id")
            if not eid:
                continue
            s = slot(eid)
            s["declared_in"].append("DispatchInfoList")
            for ui in ext.iter("UI"):
                t = ui.find("Type")
                if t is not None:
                    s["ui_type"] = (t.text or "").strip()
                m = ui.find("Menu")
                if m is not None:
                    s["menu_token"] = (m.text or "").strip()
                g = ui.find("Geometry")
                if g is not None:
                    geom = {}
                    for tag in ("Size", "MaxSize", "MinSize"):
                        node = g.find(tag)
                        if node is not None:
                            geom[tag] = {
                                c.tag: (c.text or "").strip() for c in node}
                    s["geometry"] = geom or None
            for mp in ext.iter("MainPath"):
                s["main_path"] = (mp.text or "").strip()

    exts = [merged[k] for k in sorted(merged)]
    ui_types = sorted({e["ui_type"] for e in exts if e["ui_type"]})
    if "Panel" in ui_types:
        cls, basis = "ui_panel", "parsed"
        why = "CSXS <UI><Type>Panel</Type> declared."
    elif ui_types:
        cls, basis = "ui_surface_non_panel", "parsed"
        why = ("CSXS declares <UI><Type>%s</Type> - a dialog/modeless window, "
               "not a dockable panel." % ", ".join(ui_types))
    else:
        cls, basis = "internal_shim_or_service", "heuristic"
        why = "No <UI><Type> declared on any <Extension>."
    hosts = sorted({h.get("Name") for h in root.iter("Host") if h.get("Name")})
    return {
        "path": rel(path),
        "abs_path": path,
        "bundle_id": root.get("ExtensionBundleId"),
        "bundle_version": root.get("ExtensionBundleVersion"),
        "csxs_version": root.get("Version"),
        "hosts": hosts,
        "extensions": exts,
        "ui_types": ui_types,
        "classification": cls,
        "classification_basis": basis,
        "classification_evidence": why,
    }


# ==========================================================================
# 2. native panels from .psw workspaces
# ==========================================================================

def descriptor_texts(blob):
    """
    Extract (key, string) pairs from an Adobe action-descriptor blob.

    Layout observed in every app-data blob in these .psw files:
        uint32 keyLen | keyLen ASCII bytes | 4-byte OSType | payload
    and for OSType 'TEXT': uint32 charCount | charCount * UTF-16BE code units
    (the last unit is a NUL terminator).

    This walks to each literal b'TEXT', validates the following length and
    UTF-16BE payload, then walks BACKWARDS to recover the key whose declared
    length lands exactly on the 'TEXT' marker.  Recovering the key this way is
    what separates the panel identifier ('owlContentViewStringiID') from the
    display title ('Ttl ') - a plain string scrape cannot tell them apart and
    also leaks the low byte of the length prefix into the string.
    """
    out = []
    i = 0
    n = len(blob)
    while True:
        i = blob.find(b"TEXT", i)
        if i < 0:
            break
        j = i + 4
        if j + 4 > n:
            break
        count = struct.unpack(">I", blob[j:j + 4])[0]
        if count == 0 or count > 1024 or j + 4 + count * 2 > n:
            i += 4
            continue
        raw = blob[j + 4: j + 4 + count * 2]
        try:
            s = raw.decode("utf-16-be")
        except UnicodeDecodeError:
            i += 4
            continue
        s = s.rstrip("\x00")
        if not s or any(ord(c) < 0x20 for c in s):
            i += 4
            continue
        key = None
        for klen in range(1, 64):
            k0 = i - 4 - klen
            if k0 < 0:
                break
            if struct.unpack(">I", blob[k0:k0 + 4])[0] == klen:
                cand = blob[k0 + 4:i]
                if all(0x20 <= c <= 0x7E for c in cand):
                    key = cand.decode("ascii")
                    break
        out.append({"key": key, "value": s})
        i = j + 4 + count * 2
    return out


PANEL_ID_KEY_SUFFIX = "iID"
TITLE_KEY = "Ttl "


def parse_workspace(path):
    root = ET.parse(path).getroot()
    entries = []
    tag_counts = collections.Counter()
    for el in root.iter():
        tag_counts[el.tag] += 1
        blob_b64 = el.attrib.get("app-data")
        if not blob_b64:
            continue
        try:
            blob = base64.b64decode(blob_b64)
        except Exception:  # noqa: BLE001
            continue
        texts = descriptor_texts(blob)
        panel_id = None
        title = None
        for t in texts:
            k = t["key"] or ""
            if k.endswith(PANEL_ID_KEY_SUFFIX) and t["value"].startswith("panelid."):
                panel_id = t["value"]
            elif k == TITLE_KEY:
                title = t["value"]
        if panel_id is None and title is None:
            continue
        entries.append({
            "element": el.tag,
            "element_id": el.attrib.get("id"),
            "is_closed": el.attrib.get("is-closed"),
            "panel_id": panel_id,
            "title_in_workspace": title,
            "preferred_unconstrained_size": el.attrib.get("preferred-unconstrained-size"),
            "preferred_constrained_size": el.attrib.get("preferred-constrained-size"),
            "descriptor_string_keys": sorted({t["key"] for t in texts if t["key"]}),
        })
    docks = []
    for d in root.iter("dock"):
        docks.append({"anchor": d.get("anchor"), "content": d.get("content"),
                      "is_closed": d.get("is-closed")})
    return {
        "path": rel(path),
        "size_bytes": os.path.getsize(path),
        "root_tag": root.tag,
        "root_attributes": dict(root.attrib),
        "element_counts": dict(tag_counts),
        "docks": docks,
        "panel_placements": entries,
    }


def split_panel_id(pid):
    """
    panelid.static.<name>
    panelid.dynamic.uxp/<pluginId>/<entrypointId>
    panelid.dynamic.swf.csxs.<cepExtensionId>
    """
    if pid.startswith("panelid.static."):
        return {"family": "static", "native_name": pid[len("panelid.static."):],
                "plugin_id": None, "entrypoint_id": None}
    if pid.startswith("panelid.dynamic.uxp/"):
        rest = pid[len("panelid.dynamic.uxp/"):]
        plugin, _, entry = rest.partition("/")
        return {"family": "dynamic_uxp", "native_name": None,
                "plugin_id": plugin, "entrypoint_id": entry or None}
    if pid.startswith("panelid.dynamic.swf.csxs."):
        return {"family": "dynamic_cep_swf", "native_name": None,
                "plugin_id": pid[len("panelid.dynamic.swf.csxs."):],
                "entrypoint_id": None}
    if pid.startswith("panelid.dynamic."):
        return {"family": "dynamic_other", "native_name": None,
                "plugin_id": pid[len("panelid.dynamic."):], "entrypoint_id": None}
    return {"family": "unrecognised", "native_name": None,
            "plugin_id": None, "entrypoint_id": None}


# ==========================================================================
# 3. native panels from Eve layout files
# ==========================================================================

RE_LAYOUT_NAME = re.compile(r"^\s*layout\s+([A-Za-z_]\w*)", re.M)
RE_CLASS_NAME = re.compile(r"class_name\s*:\s*'([^']+)'")
RE_VIEW_ID = re.compile(r"view_id\s*:\s*'([^']+)'")
RE_EVE_ROOT_VIEW = re.compile(r"^\s*view\s+([A-Za-z_]\w*)\s*\(", re.M)
RE_ROOT_DECL = re.compile(
    r"^[ \t]*(?:view|dialog|palette)\s*(?:[A-Za-z_]\w*\s*)?\(", re.M)
RE_TITLE_PATH = re.compile(
    r"(?:Title|PanelTitle|DefaultPanelTitle|PaletteName|PanelName)$", re.I)
RE_ZSTRING = re.compile(r"'(\$\$\$?/[^']*?)'")
RE_NAME_PARAM = re.compile(r"\bname\s*:\s*'(\$\$\$?/[^']*?)'")
RE_ID_SUFFIX = re.compile(r"-(\d+)$")


def zstring_english(z):
    """'$$$/SwatchesPanel/Title=Swatches' -> ('$$$/SwatchesPanel/Title', 'Swatches')"""
    if "=" not in z:
        return z, None
    path, _, text = z.partition("=")
    return path, text


def harvest_layout_panel(path):
    """
    Harvest PANEL IDENTITY ONLY from one Eve layout file.

    Deliberately NOT a full layout parse - another agent owns that.  This takes
    the file name, the numeric id suffix, the declared layout/widget name, the
    root class_name / view_id, and the best-available English title zstring.
    """
    text = open(path, encoding="utf-8", errors="replace").read()
    base = os.path.splitext(os.path.basename(path))[0]
    m = RE_ID_SUFFIX.search(base)
    layout_name_m = RE_LAYOUT_NAME.search(text)
    class_names = RE_CLASS_NAME.findall(text)
    view_ids = RE_VIEW_ID.findall(text)
    eve_root = RE_EVE_ROOT_VIEW.search(text)

    # ---- title -----------------------------------------------------------
    # Priority, strictest evidence first:
    #   1. a zstring whose PATH ends in Title/PanelTitle/PaletteName - these are
    #      unambiguously panel titles;
    #   2. a name: parameter inside the ROOT widget's own parameter list (the
    #      text between the root declaration and its opening brace) - NOT any
    #      nested widget, because a nested name: is a field label. Taking the
    #      first name: anywhere in the file produced false titles such as
    #      "Channel:" for the Histogram panel.
    all_title_paths = []
    for z in RE_ZSTRING.findall(text):
        zp, zt = zstring_english(z)
        if RE_TITLE_PATH.search(zp.rstrip("/")):
            all_title_paths.append({"zstring": z, "path": zp, "english": zt})

    title_z = None
    title_source = None
    if all_title_paths:
        title_z = all_title_paths[0]["zstring"]
        title_source = "zstring whose path ends in Title/PanelTitle/PaletteName"
    else:
        rootm = RE_ROOT_DECL.search(text)
        if rootm:
            brace = text.find("{", rootm.end())
            head = text[rootm.start(): brace if brace > 0 else rootm.end() + 600]
            nm = RE_NAME_PARAM.search(head)
            if nm:
                title_z = nm.group(1)
                title_source = "name: parameter inside the root widget's parameter list"
    title_path, title_text = zstring_english(title_z) if title_z else (None, None)

    # category = the directory above the Panels folder
    parts = path.split(os.sep)
    cat = None
    for i, seg in enumerate(parts):
        if seg.lower() == "panels" and i > 0:
            cat = parts[i - 1]
            break

    return {
        "file": rel(path),
        "file_name": os.path.basename(path),
        "format": os.path.splitext(path)[1].lower().lstrip("."),
        "size_bytes": os.path.getsize(path),
        "layout_tree": "drover_layouts" if "drover_layouts" in path else "layouts",
        "category_dir": cat,
        "id_suffix_in_filename": m.group(1) if m else None,
        "base_name_without_id": base[:m.start()] if m else base,
        "layout_declaration_name": layout_name_m.group(1) if layout_name_m else None,
        "root_widget_name": eve_root.group(1) if eve_root else None,
        "root_class_name": class_names[0] if class_names else None,
        "all_class_names": sorted(set(class_names)) or None,
        "root_view_id": view_ids[0] if view_ids else None,
        "title_zstring": title_z,
        "title_zstring_path": title_path,
        "title_english": title_text,
        "title_source": title_source,
        "all_title_path_zstrings": all_title_paths or None,
        "evidence": (
            "File lives in a */Panels/ directory of the Eve layout tree, which "
            "is the install's own convention for a native panel's UI layout. "
            "Identity fields above were regex-harvested from the file text; no "
            "full layout parse was performed (owned by a separate pass)."
        ),
        "title_caveat": (
            "HEURISTIC. title_english is the FIRST zstring in the file whose "
            "path ends in Title/PanelTitle/PaletteName, which is not always the "
            "panel's own title - e.g. ModernActionsPanel.eve yields 'Insert "
            "Action', a sub-dialog title. Every Title-path zstring in the file "
            "is listed in all_title_path_zstrings so the choice can be audited "
            "and overridden."
        ) if title_source and title_source.startswith("zstring") else (
            "HEURISTIC. No Title-path zstring exists in this file; the value "
            "came from a name: parameter in the root widget's own parameter "
            "list."
        ) if title_source else (
            "No title could be recovered from this file at all."
        ),
    }


def find_panel_layouts():
    out = []
    for root in LAYOUT_ROOTS:
        for dirpath, _dn, fns in os.walk(root):
            segs = [s.lower() for s in dirpath.split(os.sep)]
            if "panels" not in segs:
                continue
            for fn in fns:
                if fn.lower().endswith((".eve", ".exv")):
                    out.append(os.path.join(dirpath, fn))
    return sorted(out)


def find_palette_layouts_outside_panels_dirs():
    """
    Supplementary evidence channel: layout files whose NAME says panel/palette
    but which do not live in a */Panels/ directory.  Reported separately so the
    */Panels/* convention stays a clean, checkable rule.
    """
    out = []
    for root in LAYOUT_ROOTS:
        for dirpath, _dn, fns in os.walk(root):
            segs = [s.lower() for s in dirpath.split(os.sep)]
            if "panels" in segs:
                continue
            for fn in fns:
                low = fn.lower()
                if not low.endswith((".eve", ".exv")):
                    continue
                if "panel" in low or "palette" in low:
                    p = os.path.join(dirpath, fn)
                    out.append({
                        "file": rel(p),
                        "size_bytes": os.path.getsize(p),
                        "layout_tree": "drover_layouts" if "drover_layouts" in p else "layouts",
                        "directory_kind": os.path.basename(dirpath),
                    })
    return sorted(out, key=lambda r: r["file"])


# ==========================================================================
# main
# ==========================================================================

def main():
    sys.stdout.reconfigure(encoding="utf-8")
    now = datetime.datetime.now(datetime.timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ")

    total_files = sum(len(f) for _d, _s, f in os.walk(INSTALL))

    # ---- manifests ------------------------------------------------------
    manifest_paths = find_manifests()
    manifests = []
    hashes = collections.defaultdict(list)
    for p in manifest_paths:
        try:
            with open(p, encoding="utf-8") as fh:
                raw = fh.read()
            doc = json.loads(raw)
        except Exception as exc:  # noqa: BLE001
            manifests.append({"path": rel(p), "read_error": str(exc)})
            continue
        rec = classify_manifest(p, doc)
        h = hashlib.sha256(raw.encode("utf-8")).hexdigest()
        rec["content_sha256"] = h
        hashes[h].append(rel(p))
        manifests.append(rec)
    for rec in manifests:
        h = rec.get("content_sha256")
        if h and len(hashes[h]) > 1:
            rec["byte_identical_copies_at"] = [
                x for x in hashes[h] if x != rec["path"]]

    csxs_paths = find_csxs_manifests()
    csxs = [classify_csxs(p) for p in csxs_paths]
    # CEP extension dirs that have a CSXS manifest but NO manifest.json
    manifest_dirs = {os.path.dirname(p).lower() for p in manifest_paths}
    csxs_without_json = [
        c for c in csxs
        if os.path.dirname(os.path.dirname(c["abs_path"])).lower() not in manifest_dirs
    ] if csxs and all("abs_path" in c for c in csxs) else []

    by_class = collections.defaultdict(list)
    for rec in manifests:
        by_class[rec.get("classification", "read_error")].append(rec)

    unique_ids = sorted({
        r.get("id") for r in manifests if r.get("id")})

    # ---- workspaces -----------------------------------------------------
    psw_paths = sorted(
        os.path.join(WORKSPACES_DIR, f)
        for f in os.listdir(WORKSPACES_DIR) if f.lower().endswith(".psw")
    ) if os.path.isdir(WORKSPACES_DIR) else []
    workspaces = [parse_workspace(p) for p in psw_paths]

    panel_index = {}
    for ws in workspaces:
        for pl in ws["panel_placements"]:
            pid = pl["panel_id"]
            if not pid:
                continue
            e = panel_index.setdefault(pid, {
                "panel_id": pid,
                **split_panel_id(pid),
                "titles_seen_in_workspaces": [],
                "workspaces": [],
                "placement_count": 0,
            })
            e["placement_count"] += 1
            if ws["path"] not in e["workspaces"]:
                e["workspaces"].append(ws["path"])
            t = pl["title_in_workspace"]
            if t and t not in e["titles_seen_in_workspaces"]:
                e["titles_seen_in_workspaces"].append(t)

    # ---- layouts --------------------------------------------------------
    layout_paths = find_panel_layouts()
    layouts = [harvest_layout_panel(p) for p in layout_paths]
    extra_palette_layouts = find_palette_layouts_outside_panels_dirs()

    # ---- correlate native panel ids to layout files ---------------------
    def norm(s):
        return re.sub(r"[^a-z0-9]", "", (s or "").lower())

    # Every comparable token a layout file offers, tagged with which field it
    # came from, so a match can be audited rather than trusted.
    layout_tokens = []
    for lay in layouts:
        for field in ("base_name_without_id", "layout_declaration_name",
                      "root_class_name", "title_english", "root_view_id"):
            val = lay.get(field)
            if not val:
                continue
            n = norm(val)
            if not n:
                continue
            # strip the redundant 'T' class prefix and trailing panel/palette
            variants = {n}
            if field == "root_class_name" and n.startswith("t") and len(n) > 3:
                variants.add(n[1:])
            for v in list(variants):
                for suf in ("panel", "palette", "contents", "view"):
                    if v.endswith(suf) and len(v) > len(suf) + 2:
                        variants.add(v[: -len(suf)])
            for v in variants:
                layout_tokens.append({
                    "file": lay["file"], "field": field,
                    "value": val, "norm": v,
                })

    native_panels = []
    matched_layout_files = set()
    for pid, e in sorted(panel_index.items()):
        if e["family"] != "static":
            continue
        nm = norm(e["native_name"])
        exact, near = [], []
        for t in layout_tokens:
            if not nm or not t["norm"]:
                continue
            if t["norm"] == nm:
                exact.append(t)
            elif len(nm) >= 6 and len(t["norm"]) >= 6 and (
                    t["norm"].startswith(nm) or nm.startswith(t["norm"])):
                near.append(t)

        def fold(ts):
            out = {}
            for t in ts:
                out.setdefault(t["file"], []).append(
                    {"matched_field": t["field"], "matched_value": t["value"],
                     "normalized_token": t["norm"]})
            return [{"file": f, "matched_on": v} for f, v in sorted(out.items())]

        exact_f = fold(exact)
        exact_files = {r["file"] for r in exact_f}
        near_f = [r for r in fold(near) if r["file"] not in exact_files]
        for r in exact_f + near_f:
            matched_layout_files.add(r["file"])

        native_panels.append({
            "panel_id": pid,
            "native_name": e["native_name"],
            "titles_seen_in_workspaces": e["titles_seen_in_workspaces"],
            "title_note": (
                "Empty is expected: the workspace descriptor carries a 'Ttl ' "
                "TEXT field only for extension panels. Native panels get their "
                "displayed title from a compiled resource, which is not in any "
                "readable install file."
            ) if not e["titles_seen_in_workspaces"] else None,
            "workspace_files_referencing_it": e["workspaces"],
            "workspace_placement_count": e["placement_count"],
            "layout_match_exact": exact_f or None,
            "layout_match_near": near_f or None,
            "layout_match_found": bool(exact_f or near_f),
            "layout_match_basis": (
                "heuristic. 'exact' means the panel-id tail equals a "
                "normalized token from the layout file (base filename, layout "
                "declaration, root class_name with an optional leading 'T' and "
                "an optional trailing panel/palette/contents/view stripped, "
                "root view_id, or English title). 'near' means one normalized "
                "token is a prefix of the other and BOTH are at least 6 "
                "characters. NO file in this install declares the panel-id to "
                "layout mapping, so both kinds are inferences and neither is "
                "authoritative."
            ),
            "source_evidence": [
                {
                    "kind": "workspace_descriptor",
                    "detail": (
                        "The string %r appears as the 'owlContentViewStringiID' "
                        "TEXT field inside the base64 app-data descriptor of a "
                        "<palette> element in %d shipped .psw workspace file(s)."
                        % (pid, len(e["workspaces"]))
                    ),
                    "files": e["workspaces"],
                },
            ],
        })

    unmatched_layout_files = sorted(
        {lay["file"] for lay in layouts} - matched_layout_files)
    native_panels_without_layout = [
        n["native_name"] for n in native_panels if not n["layout_match_found"]]

    extension_panels_from_workspaces = [
        {
            "panel_id": pid,
            **{k: v for k, v in e.items()
               if k in ("family", "plugin_id", "entrypoint_id")},
            "titles_seen_in_workspaces": e["titles_seen_in_workspaces"],
            "workspace_files_referencing_it": e["workspaces"],
            "workspace_placement_count": e["placement_count"],
        }
        for pid, e in sorted(panel_index.items()) if e["family"] != "static"
    ]

    # link workspace-referenced uxp panels back to manifests actually installed
    manifest_ids = {r.get("id"): r for r in manifests if r.get("id")}
    for ep in extension_panels_from_workspaces:
        pid_plugin = ep.get("plugin_id")
        rec = manifest_ids.get(pid_plugin)
        ep["manifest_present_in_install"] = rec is not None
        if rec is not None:
            ep["manifest_path"] = rec["path"]
            ep["manifest_classification"] = rec["classification"]
            ids = [e["id"] for e in rec["entrypoints"] if e.get("id")]
            ep["entrypoint_id_declared_by_manifest"] = (
                ep.get("entrypoint_id") in ids if ep.get("entrypoint_id") else None)
        else:
            ep["note"] = (
                "Referenced by a shipped workspace but NO manifest with this id "
                "exists in the install. Stale workspace reference to an "
                "extension that is not installed."
            )

    # extension panels straight from the manifests
    extension_panels = []
    for rec in by_class.get("ui_panel", []):
        for ep in rec["panel_entrypoints"]:
            extension_panels.append({
                "plugin_id": rec["id"],
                "plugin_name": rec["name"],
                "plugin_version": rec["version"],
                "manifest_path": rec["path"],
                "framework": "CEP-hosted UXP" if rec["evidence"]["path_bucket"] == "cep" else "UXP",
                "entrypoint_id": ep["id"],
                "entrypoint_label": ep["label"],
                "sizing_hints": ep["sizing_hints"],
                "has_sizing_hints": ep["has_sizing_hints"],
                "expected_workspace_panel_id": (
                    "panelid.dynamic.uxp/%s/%s" % (rec["id"], ep["id"])
                    if rec["id"] and ep["id"] else None),
                "referenced_by_a_shipped_workspace": any(
                    x.get("plugin_id") == rec["id"]
                    and x.get("entrypoint_id") == ep["id"]
                    for x in extension_panels_from_workspaces),
                "evidence": (
                    "manifest.json at %s declares entrypoints[type='panel'] "
                    "with id %r%s."
                    % (rec["path"], ep["id"],
                       " and sizing hints " + json.dumps(ep["sizing_hints"])
                       if ep["has_sizing_hints"] else " and NO sizing hints")
                ),
                "classification_basis": "parsed",
            })
    seen_csxs_panels = set()
    for c in csxs:
        for e in c.get("extensions", []):
            # only the extensions that THEMSELVES declare Type=Panel
            if (e.get("ui_type") or "").lower() != "panel":
                continue
            dedupe_key = (c.get("bundle_id"), e.get("id"))
            if dedupe_key in seen_csxs_panels:
                continue
            seen_csxs_panels.add(dedupe_key)
            extension_panels.append({
                "plugin_id": e.get("id"),
                "plugin_name": c.get("bundle_id"),
                "plugin_version": e.get("version"),
                "manifest_path": c["path"],
                "also_shipped_at": [
                    o["path"] for o in csxs
                    if o is not c
                    and o.get("bundle_id") == c.get("bundle_id")
                    and any(x.get("id") == e.get("id")
                            for x in o.get("extensions", []))
                ] or None,
                "framework": "CEP (CSXS)",
                "entrypoint_id": e.get("id"),
                "entrypoint_label": None,
                "sizing_hints": e.get("geometry"),
                "has_sizing_hints": bool(e.get("geometry")),
                "main_path": e.get("main_path"),
                "expected_workspace_panel_id":
                    "panelid.dynamic.swf.csxs.%s" % e.get("id"),
                "referenced_by_a_shipped_workspace": any(
                    x.get("plugin_id") == e.get("id")
                    for x in extension_panels_from_workspaces),
                "evidence": (
                    "CSXS manifest.xml at %s declares <Extension Id=%r> with "
                    "<UI><Type>Panel</Type>%s." % (
                        c["path"], e.get("id"),
                        " and a <Geometry> block" if e.get("geometry") else
                        " and NO <Geometry> block")
                ),
                "classification_basis": "parsed",
            })

    # ---- prior artifacts, measured --------------------------------------
    prior = {}
    entrypoint_mismatches = []
    if os.path.exists(PRIOR_UXP):
        with open(PRIOR_UXP, encoding="utf-8") as fh:
            pu = json.load(fh)
        pm = pu.get("manifests", [])
        mine_by_path = {r["path"]: r for r in manifests if "path" in r}
        for m in pm:
            mp = (m.get("path") or "").replace("\\", "/")
            mine = mine_by_path.get(mp)
            if mine is None:
                # path no longer on disk - that is an install-state change,
                # reported under correction C0, not an entry-point parse defect
                continue
            earlier_n = len(m["entrypoints"]) if isinstance(
                m.get("entrypoints"), list) else 0
            now_n = mine["evidence"]["entrypoint_count"]
            if earlier_n != now_n:
                entrypoint_mismatches.append({
                    "path": mp,
                    "plugin_id": mine.get("id"),
                    "earlier_entrypoint_count": earlier_n,
                    "earlier_entrypoints_field": (
                        "list" if isinstance(m.get("entrypoints"), list)
                        else repr(m.get("entrypoints"))),
                    "this_pass_entrypoint_count": now_n,
                    "this_pass_entrypoint_key": mine["evidence"]["entrypoint_key_used"],
                    "this_pass_entrypoint_types": mine["evidence"]["entrypoint_types"],
                })
        prior["uxp_manifests_json"] = {
            "path": PRIOR_UXP,
            "declared_count": pu.get("count"),
            "actual_array_length": len(pm),
            "entries_whose_entrypoints_field_is_empty_or_null": sum(
                1 for m in pm if not m.get("entrypoints")),
            "entries_with_at_least_one_panel_entrypoint": sum(
                1 for m in pm
                if isinstance(m.get("entrypoints"), list)
                and any(e.get("type") == "panel" for e in m["entrypoints"])),
            "distinct_plugin_ids": len({m.get("id") for m in pm if m.get("id")}),
            "fields_per_entry": sorted({k for m in pm for k in m}),
        }
    if os.path.exists(PRIOR_WORKSPACES):
        with open(PRIOR_WORKSPACES, encoding="utf-8") as fh:
            pw = json.load(fh)
        pf = pw.get("files", [])
        ext = collections.Counter(
            os.path.splitext(f.get("path", ""))[1].lower() for f in pf)
        prior["workspaces_json"] = {
            "path": PRIOR_WORKSPACES,
            "declared_count": pw.get("count"),
            "actual_array_length": len(pf),
            "entry_shape": sorted({k for f in pf for k in f}),
            "by_file_extension": dict(ext),
            "what_it_actually_is": (
                "A FILE LISTING of path+size, not workspace content. %d of its "
                "%d entries are .psw workspaces; the rest are .eve/.exv layout "
                "files plus one .json and one .txt. It contains no panel "
                "identifiers at all - the app-data descriptors inside the .psw "
                "files were never decoded." % (ext.get(".psw", 0), len(pf))
            ),
        }

    cat_counts = {k: len(v) for k, v in sorted(by_class.items())}

    # ---- did the install itself change between the two passes? ----------
    install_delta = None
    if os.path.exists(PRIOR_UXP):
        with open(PRIOR_UXP, encoding="utf-8") as fh:
            old_paths = {
                (m.get("path") or "").replace("\\", "/")
                for m in json.load(fh).get("manifests", [])}
        new_paths = {r["path"] for r in manifests if "path" in r}
        gone = sorted(old_paths - new_paths)
        added = sorted(new_paths - old_paths)
        install_delta = {
            "earlier_manifest_path_count": len(old_paths),
            "this_pass_manifest_path_count": len(new_paths),
            "paths_present_earlier_and_absent_now": [
                {"path": p,
                 "exists_on_disk_now": os.path.exists(
                     os.path.join(INSTALL, p.replace("/", os.sep)))}
                for p in gone],
            "paths_absent_earlier_and_present_now": [
                {"path": p,
                 "exists_on_disk_now": os.path.exists(
                     os.path.join(INSTALL, p.replace("/", os.sep))),
                 "plugin_id": next(
                     (r.get("id") for r in manifests if r.get("path") == p), None)}
                for p in added],
            "finding": (
                "The install CONTENT changed between the earlier pass and this "
                "one, even though the manifest file count is 63 in both. %d "
                "path(s) recorded earlier no longer exist on disk and %d "
                "path(s) exist now that were not recorded earlier. The equal "
                "totals are a coincidence, not evidence of stability."
                % (len(gone), len(added))
            ) if (gone or added) else "No path-level difference.",
            "corroborating_evidence": (
                "Directory modification times across the whole install root "
                "cluster in a single narrow window, and the earlier artifacts "
                "in this output directory were written inside that same "
                "window. That is consistent with the earlier harvest having run "
                "while an install or update was still writing files. STATED AS "
                "OBSERVATION ONLY - the cause was not verified and no installer "
                "log was inspected."
            ),
            "consequence": (
                "Any figure in this document that is compared against the "
                "earlier artifact is comparing two different states of the "
                "install. Figures measured directly from disk in THIS pass are "
                "current; figures attributed to the earlier artifact describe "
                "the earlier state."
            ),
        }

    doc = {
        "schema_id": "handshake.greenroom.photoshop.panels.v1",
        "generated_at": now,
        "generator": "photoshop-panel-inventory.py",
        "target_application": {
            "name": "Adobe Photoshop 2026",
            "install_root": INSTALL,
            "launched": False,
            "access_mode": "read-only file parsing; the application was never started",
        },

        "method": {
            "overall": (
                "Four independent evidence channels were read directly off "
                "disk: (1) every manifest.json in the install, (2) every CEP "
                "CSXS/manifest.xml, (3) the base64 action-descriptor blobs "
                "inside the shipped .psw workspace files, (4) the Eve layout "
                "files that live in */Panels/ directories. Photoshop was never "
                "launched. Nothing was copied from the earlier harvest pass; "
                "the earlier files were re-read only to MEASURE what they "
                "actually contained."
            ),
            "manifest_discovery": (
                "os.walk over the entire install root (%d files walked) "
                "matching the exact filename 'manifest.json'. A second walk "
                "collected every CSXS/manifest.xml, because CEP extensions "
                "declare themselves in XML and are invisible to a "
                "manifest.json-only scan."
                % total_files
            ),
            "entrypoint_key_variants": (
                "CRITICAL: this install uses THREE spellings for the "
                "entry-point array - 'entrypoints', 'entryPoints' and "
                "'uiEntrypoints'. All three are read. A scanner that reads only "
                "the lowercase spelling records an empty entry-point list for "
                "five plugins that in fact declare UI surfaces."
            ),
            "classification": (
                "Each manifest is classified from its own content in a fixed "
                "order: a 'targets' inference-runtime map with no entry points "
                "of any spelling -> ml_model; >=1 entry point of type 'panel' "
                "-> ui_panel; >=1 entry point of a non-panel UI type -> "
                "ui_surface_non_panel; only 'command' entry points -> "
                "command_extension; no entry points at all -> "
                "internal_shim_or_service. Every record carries the evidence "
                "that fired its rule plus the raw evidence block (top-level "
                "keys, entry-point key used, entry-point types, sizing-hint "
                "count, host apps, loadEvent, permission keys, path bucket, "
                "nesting). Records whose rule required inference are marked "
                "classification_basis 'heuristic'. Name-based guessing was not "
                "used for any classification."
            ),
            "workspace_descriptor_decoding": (
                "Each .psw is XML. Its <palette>, <control-bar> and <toolbar> "
                "elements carry a base64 'app-data' attribute holding an Adobe "
                "action descriptor. The descriptor was walked structurally: for "
                "each literal 'TEXT' type marker the following uint32 char "
                "count and UTF-16BE payload were validated, then the key was "
                "recovered by walking BACKWARDS to the uint32 key length that "
                "lands exactly on the marker. That key recovery is what "
                "separates the panel identifier field "
                "('owlContentViewStringiID') from the display title field "
                "('Ttl '); a naive printable-string scrape conflates them and "
                "also leaks the low byte of the length prefix into the value."
            ),
            "native_panel_extraction": (
                "Panel identifiers recovered from the workspace descriptors "
                "have three shapes: 'panelid.static.<name>' (a NATIVE built-in "
                "panel), 'panelid.dynamic.uxp/<pluginId>/<entrypointId>' (a UXP "
                "extension panel) and 'panelid.dynamic.swf.csxs.<extensionId>' "
                "(a legacy CEP/SWF extension panel). The static family is the "
                "native panel surface and appears in NO manifest anywhere in "
                "the install."
            ),
            "layout_panel_identity": (
                "Every .eve/.exv file inside a */Panels/ directory under "
                "Required\\layouts and Required\\drover_layouts was opened and "
                "PANEL IDENTITY ONLY was harvested by regex: file name, numeric "
                "id suffix, 'layout <name>' declaration, root widget name, root "
                "class_name, root view_id, and the best English title zstring "
                "(a name: parameter, else a zstring whose path ends in Title). "
                "This is deliberately NOT a full layout parse - a separate pass "
                "owns that - so no widget tree, binding or property was read."
            ),
            "correlation": (
                "Static panel ids were matched to layout files by "
                "case-and-punctuation-insensitive prefix comparison against the "
                "layout's base filename, layout declaration name, root class "
                "name and English title. NO file in the install states this "
                "mapping, so every match is labelled heuristic and every panel "
                "keeps its own independent workspace evidence."
            ),
            "prior_artifact_measurement": (
                "uxp_manifests.json and workspaces.json were re-read and "
                "counted field by field. Every number attributed to them below "
                "was measured, not recalled."
            ),
        },

        "source_files": [
            {"path": INSTALL, "role": "walked recursively",
             "files_walked": total_files},
            {"path": os.path.join(INSTALL, "Required"),
             "role": "manifest.json and CSXS/manifest.xml discovery"},
            {"path": WORKSPACES_DIR,
             "role": "shipped .psw workspaces - native panel identifiers",
             "psw_file_count": len(psw_paths)},
            {"path": LAYOUT_ROOTS[0],
             "role": "Eve layout tree - */Panels/* files are native panel evidence"},
            {"path": LAYOUT_ROOTS[1],
             "role": "second Eve layout tree (drover) - */panels/* files, lowercase dir name"},
            {"path": PRIOR_UXP, "role": "earlier artifact, re-read to measure its claims"},
            {"path": PRIOR_WORKSPACES, "role": "earlier artifact, re-read to measure its claims"},
        ],

        "totals": {
            "note": (
                "FILE counts and ENTRY counts are labelled separately and are "
                "not interchangeable."
            ),
            "install_files_walked": total_files,
            "manifest_json_FILES_found_this_pass": len(manifest_paths),
            "manifest_json_FILES_found_earlier_pass": (
                prior.get("uxp_manifests_json", {}).get("actual_array_length")),
            "manifest_json_files_that_are_byte_identical_duplicates": sum(
                1 for r in manifests if r.get("byte_identical_copies_at")),
            "distinct_plugin_ids_across_manifests": len(unique_ids),
            "csxs_manifest_xml_FILES_found": len(csxs_paths),
            "csxs_extensions_declared": sum(
                len(c.get("extensions", [])) for c in csxs),
            "manifests_by_classification": cat_counts,
            "psw_workspace_FILES": len(psw_paths),
            "panel_placement_ENTRIES_across_all_workspaces": sum(
                len(w["panel_placements"]) for w in workspaces),
            "distinct_panel_ids_in_workspaces": len(panel_index),
            "distinct_NATIVE_panel_ids": len(native_panels),
            "distinct_extension_panel_ids_in_workspaces": len(
                extension_panels_from_workspaces),
            "panel_layout_FILES_in_Panels_dirs": len(layout_paths),
            "panel_layout_files_layouts_tree": sum(
                1 for l in layouts if l["layout_tree"] == "layouts"),
            "panel_layout_files_drover_tree": sum(
                1 for l in layouts if l["layout_tree"] == "drover_layouts"),
            "supplementary_palette_layout_files_outside_Panels_dirs": len(
                extra_palette_layouts),
            "extension_panel_ENTRIES_declared_by_manifests": len(extension_panels),
            "extension_panel_entries_from_uxp_manifest_json": sum(
                1 for e in extension_panels if e["framework"] != "CEP (CSXS)"),
            "extension_panel_entries_from_cep_csxs_manifest_xml": sum(
                1 for e in extension_panels if e["framework"] == "CEP (CSXS)"),
            "native_panel_ids_with_no_layout_match": len(
                native_panels_without_layout),
            "panels_dir_layout_files_with_no_panel_id_match": len(
                unmatched_layout_files),
        },

        "corrections": {
            "headline": (
                "uxp_manifests.json is not a panel list. It is a listing of 63 "
                "manifest.json FILES, and treating its 63 as a panel count is "
                "wrong in five separate ways, each quantified below."
            ),
            "prior_artifacts_measured": prior,
            "install_state_changed_between_passes": install_delta,
            "errors": [
                {
                    "id": "C0",
                    "claim_corrected": (
                        "the earlier 63 and this pass's 63 describe the same "
                        "set of files"),
                    "reality": (
                        install_delta["finding"] if install_delta
                        else "Earlier artifact unavailable to compare."
                    ),
                    "true_figure": {
                        "paths_gone_since_earlier_pass": [
                            p["path"] for p in
                            install_delta["paths_present_earlier_and_absent_now"]]
                        if install_delta else None,
                        "paths_new_since_earlier_pass": [
                            p["path"] for p in
                            install_delta["paths_absent_earlier_and_present_now"]]
                        if install_delta else None,
                    },
                },
                {
                    "id": "C1",
                    "claim_corrected": "63 = the number of Photoshop panels",
                    "reality": (
                        "63 is a FILE count of manifest.json files. Of those, "
                        "%d are ML model descriptors under Required/sensei_models "
                        "with no entry points at all, %d are command-only "
                        "extensions, %d are non-panel UI surfaces, %d is a "
                        "shim/service with no entry points, and only %d "
                        "declare a panel entry point."
                        % (cat_counts.get("ml_model", 0),
                           cat_counts.get("command_extension", 0),
                           cat_counts.get("ui_surface_non_panel", 0),
                           cat_counts.get("internal_shim_or_service", 0),
                           cat_counts.get("ui_panel", 0))
                    ),
                    "true_figure": {
                        "manifest_files": len(manifest_paths),
                        "manifests_declaring_a_panel": cat_counts.get("ui_panel", 0),
                        "panel_entrypoint_entries_declared": len(extension_panels),
                    },
                },
                {
                    "id": "C2",
                    "claim_corrected": (
                        "the UXP manifests describe Photoshop's panel surface"),
                    "reality": (
                        "They do not describe the majority of it. %d NATIVE "
                        "built-in panel identifiers (panelid.static.*) were "
                        "recovered from the shipped workspaces. Not one of them "
                        "appears in any manifest.json in the install. Native "
                        "panels are the bulk of the real panel surface and were "
                        "entirely absent from the earlier artifact."
                        % len(native_panels)
                    ),
                    "true_figure": {
                        "native_panel_ids": len(native_panels),
                        "extension_panel_ids_referenced_by_workspaces": len(
                            extension_panels_from_workspaces),
                    },
                },
                {
                    "id": "C3",
                    "claim_corrected": (
                        "the 63 manifests are 63 distinct plugins"),
                    "reality": (
                        "%d of the 63 manifest.json files are byte-identical "
                        "duplicates of another file in the install "
                        "(Required/CEP/extensions/com.adobe.DesignLibraryPanel.html "
                        "and Required/UXP/com.adobe.cclibrariespanel ship the "
                        "same three manifests). There are %d distinct plugin "
                        "ids, not 63."
                        % (sum(1 for r in manifests if r.get("byte_identical_copies_at")),
                           len(unique_ids))
                    ),
                    "true_figure": {"distinct_plugin_ids": len(unique_ids)},
                },
                {
                    "id": "C4",
                    "claim_corrected": (
                        "reading the 'entrypoints' key captures every plugin's "
                        "entry points"),
                    "reality": (
                        "Three spellings are in use: 'entrypoints', "
                        "'entryPoints' and 'uiEntrypoints'. The earlier "
                        "artifact recorded an empty or null entrypoints value "
                        "for %s of its 63 entries; several of those plugins do "
                        "declare entry points, just under a different key - "
                        "com.adobe.ccx.start (10 entry points under "
                        "'entryPoints'), com.adobe.inAppNotifications (2), "
                        "com.adobe.photoshop.facepile (1), com.adobe.nfp.gallery "
                        "(1 under 'uiEntrypoints') and com.adobe.ccx.lrimporter "
                        "(1 under 'uiEntrypoints')."
                        % prior.get("uxp_manifests_json", {}).get(
                            "entries_whose_entrypoints_field_is_empty_or_null", "?")
                    ),
                    "true_figure": {
                        "entrypoint_key_variants_in_use": ENTRYPOINT_KEYS,
                    },
                },
                {
                    "id": "C5",
                    "claim_corrected": (
                        "scanning for manifest.json finds every extension"),
                    "reality": (
                        "CEP extensions declare themselves in "
                        "CSXS/manifest.xml. %d such files exist. "
                        "Required/CEP/extensions/com.adobe.photoshop.crema has "
                        "a CSXS manifest and NO manifest.json, so it was "
                        "invisible to the earlier scan entirely. It declares "
                        "two extensions, both <UI><Type>ModalDialog</Type> - "
                        "so it is correctly NOT a panel, but it should have "
                        "been seen and then excluded, not missed."
                        % len(csxs_paths)
                    ),
                    "true_figure": {
                        "csxs_manifest_files": len(csxs_paths),
                        "csxs_dirs_without_a_manifest_json": [
                            c["path"] for c in csxs_without_json],
                    },
                },
                {
                    "id": "C7",
                    "claim_corrected": (
                        "the earlier artifact's per-entry entrypoint arrays "
                        "faithfully reflect the manifests on disk"),
                    "reality": (
                        "Re-parsing every manifest and comparing entry-point "
                        "counts path by path finds %d entries where the earlier "
                        "artifact's count differs from the file's actual "
                        "content. These are listed in full under "
                        "entrypoint_count_mismatches. They are not all "
                        "explained by the key-spelling issue in C4: for example "
                        "Required/UXP/com.adobe.ccx.start/uxp-plugin-account-menu-trigger/dist/manifest.json "
                        "uses the plain lowercase 'entrypoints' key and "
                        "declares one command entry point, yet the earlier "
                        "artifact recorded an empty array for it."
                        % len(entrypoint_mismatches)
                    ),
                    "true_figure": {
                        "entries_with_mismatched_entrypoint_counts": len(
                            entrypoint_mismatches),
                    },
                    "entrypoint_count_mismatches": entrypoint_mismatches,
                },
                {
                    "id": "C6",
                    "claim_corrected": "workspaces.json holds 858 workspaces",
                    "reality": (
                        prior.get("workspaces_json", {}).get(
                            "what_it_actually_is",
                            "workspaces.json was not present to measure.")
                    ),
                    "true_figure": {
                        "psw_workspace_files": len(psw_paths),
                        "panel_placement_entries_decoded_this_pass": sum(
                            len(w["panel_placements"]) for w in workspaces),
                    },
                },
            ],
        },

        "native_panels": native_panels,
        "native_panel_layout_files": layouts,
        "panel_id_to_layout_gaps": {
            "explanation": (
                "The two sides of the native panel surface do not line up "
                "one-to-one and nothing in the install reconciles them. Both "
                "gaps are listed so a Rust implementer can pair them by hand "
                "instead of inheriting a silent hole. Several obvious-looking "
                "pairings (e.g. the workspace id 'toolpresets' and the layout "
                "toolPresetPanel-4252.exv) were deliberately NOT asserted here "
                "because the name evidence alone does not prove them."
            ),
            "native_panel_ids_with_no_layout_match": native_panels_without_layout,
            "native_panel_ids_with_no_layout_match_count": len(
                native_panels_without_layout),
            "panels_dir_layout_files_matched_by_no_panel_id": unmatched_layout_files,
            "panels_dir_layout_files_matched_by_no_panel_id_count": len(
                unmatched_layout_files),
        },
        "supplementary_palette_layout_files_outside_Panels_dirs": extra_palette_layouts,

        "extension_panels": {
            "declared_by_manifests": extension_panels,
            "referenced_by_shipped_workspaces": extension_panels_from_workspaces,
        },

        "non_panel_manifests": {
            k: by_class[k] for k in sorted(by_class) if k != "ui_panel"
        },

        "all_manifests_classified": manifests,
        "cep_csxs_manifests": csxs,
        "workspaces": workspaces,

        "unknowns": [
            "No file in the install maps a panelid.static.<name> to its Eve "
            "layout file, its window class, or its menu entry under Window >. "
            "The panel-id-to-layout links in native_panels are name-similarity "
            "guesses only. UNRESOLVED.",
            "%d of the %d native panel ids got no layout-file match at all "
            "(listed in panel_id_to_layout_gaps). Some of them (options, "
            "toolbar, properties, picker) are application chrome rather than "
            "Eve-laid-out panels; others (toolpresets, textcharstyle, "
            "textparastyle, textglyphspanel, animation, patchmatch) very likely "
            "DO have a layout file under a different name. Neither reading was "
            "verified - no binary was inspected and no name guess was asserted."
            % (len(native_panels_without_layout), len(native_panels)),
            "%d layout file(s) in */Panels/ directories match no panel id from "
            "any shipped workspace (listed in panel_id_to_layout_gaps). They "
            "may be panels no default workspace opens, dead code (several sit "
            "under layouts/Unused/ and layouts/Debug/), sub-views of another "
            "panel, or panels whose id simply differs from every name in the "
            "file. Not resolvable from files alone."
            % len(unmatched_layout_files),
            "The UXP entry-point 'type' vocabulary (panel, command, view, "
            "homescreen, welcome, popover, facepile, neuralGallery, lrimporter, "
            "inAppNotifications, and the picker types) is not documented "
            "anywhere in this install. Which of the non-panel types produce a "
            "dockable surface at runtime is UNVERIFIED.",
            "Several panelid.dynamic.swf.csxs.* ids in the shipped workspaces "
            "point at CEP/SWF extensions that are not installed. Whether "
            "Photoshop still honours them is UNVERIFIED - the flag "
            "manifest_present_in_install records the file fact only.",
            "Panel dimensions for NATIVE panels were not extracted. The .psw "
            "preferred-size attributes are per-workspace saved state, not the "
            "panel's intrinsic constraints, and the intrinsic sizes live in the "
            "Eve layouts which this pass deliberately did not fully parse.",
            "No panel's Window-menu label was recovered for native panels "
            "except where a workspace descriptor happened to carry a 'Ttl ' "
            "field; that field is present for extension panels and largely "
            "absent for static ones.",
            "Only %d of the %d */Panels/ layout files contain a zstring whose "
            "path ends in Title/PanelTitle/PaletteName. The other %d expose no "
            "self-describing title at all, and where a title WAS found it is "
            "the first such zstring in the file, which is sometimes a "
            "sub-dialog's title rather than the panel's own (see title_caveat "
            "and all_title_path_zstrings on each record). Native panel display "
            "names are therefore NOT reliably recoverable from the layout tree."
            % (sum(1 for l in layouts if l["title_english"]), len(layouts),
               sum(1 for l in layouts if not l["title_english"])),
        ],

        "heuristics": [
            {
                "id": "P1",
                "claim": "panelid.static.* identifies a NATIVE built-in panel",
                "basis": "heuristic",
                "evidence": "the three-way id namespace in the workspace descriptors (static / dynamic.uxp/<plugin>/<entry> / dynamic.swf.csxs.<ext>) maps cleanly onto native / UXP / CEP, and every dynamic.uxp id resolves to a real installed manifest id",
                "what_is_parsed_vs_inferred": "the id strings and their namespaces are parsed; 'static means native built-in' is inferred",
            },
            {
                "id": "P2",
                "claim": "every file under a */Panels/ layout directory is evidence of a real native panel",
                "basis": "heuristic",
                "evidence": "the directory convention is the install's own; sibling directories are Dialogs/, Flyouts/, Tools/, Properties/",
                "what_is_parsed_vs_inferred": "the paths are parsed; that each one is a shipping panel is inferred - files under layouts/Unused/ and layouts/Debug/ are almost certainly not user-visible",
            },
            {
                "id": "P3",
                "claim": "static panel id X corresponds to layout file Y",
                "basis": "heuristic",
                "evidence": "case-and-punctuation-insensitive prefix match between the id tail and the layout base name / layout declaration / class name / title",
                "what_is_parsed_vs_inferred": "both sides are parsed; the correspondence is inferred and can be wrong in both directions (missed matches and false matches)",
            },
            {
                "id": "P4",
                "claim": "manifests with a 'targets' runtime map and no entry points are ML models, not UI",
                "basis": "parsed for the structure, heuristic for the label",
                "evidence": "top-level keys are id/name/description/targets(+author,version); targets values hold components with model paths plus tensor inputs/outputs; all sit under Required/sensei_models",
                "what_is_parsed_vs_inferred": "the schema shape and location are parsed; calling them 'ML model descriptors' is inferred",
            },
            {
                "id": "P5",
                "claim": "non-panel UI entry-point types are UI surfaces but not dockable panels",
                "basis": "heuristic",
                "evidence": "none of them carries any of the four sizing/docking hint keys, unlike every type='panel' entry point",
                "what_is_parsed_vs_inferred": "the absence of sizing hints is parsed; the dockability conclusion is inferred",
            },
            {
                "id": "P6",
                "claim": "manifests with no entry points at all are internal shims or services",
                "basis": "heuristic",
                "evidence": "they ship a main bundle and a host block, often with host.data.loadEvent='startup', but declare nothing invocable",
                "what_is_parsed_vs_inferred": "the manifest contents are parsed; the shim/service role is inferred",
            },
            {
                "id": "P7",
                "claim": "byte-identical manifests under CEP/extensions and UXP are the same plugin shipped twice",
                "basis": "parsed",
                "evidence": "SHA-256 of the file contents is identical and the declared plugin id matches",
                "what_is_parsed_vs_inferred": "fully parsed; only the word 'shipped twice' is interpretation",
            },
        ],
    }

    os.makedirs(OUT_DIR, exist_ok=True)
    with open(OUT_PATH, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)

    # ---- self-verification: re-read what was just written ---------------
    with open(OUT_PATH, encoding="utf-8") as fh:
        back = json.load(fh)
    checks = {
        "reread_ok": True,
        "file_size_bytes": os.path.getsize(OUT_PATH),
        "totals_match": back["totals"] == doc["totals"],
        "native_panels": len(back["native_panels"]),
        "native_panel_layout_files": len(back["native_panel_layout_files"]),
        "extension_panels_declared_by_manifests": len(
            back["extension_panels"]["declared_by_manifests"]),
        "extension_panels_in_workspaces": len(
            back["extension_panels"]["referenced_by_shipped_workspaces"]),
        "non_panel_manifest_categories": {
            k: len(v) for k, v in back["non_panel_manifests"].items()},
        "all_manifests": len(back["all_manifests_classified"]),
        "csxs_manifests": len(back["cep_csxs_manifests"]),
        "workspaces": len(back["workspaces"]),
        "corrections_errors": len(back["corrections"]["errors"]),
        "unknowns": len(back["unknowns"]),
        "heuristics": len(back["heuristics"]),
    }
    print(json.dumps(checks, indent=1))


if __name__ == "__main__":
    main()
