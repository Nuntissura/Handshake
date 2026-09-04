#!/usr/bin/env python3
"""Handshake Studio green room: offline harvest of an installed Adobe app tree.

No app process is launched. Channels:
  typelib      - pywin32 makepy over the registered type library -> scripting DOM (classes/props/methods/enums)
  shortcuts    - default keyboard shortcut set files (.kys) parsed when XML
  uxp          - built-in UXP plugin/panel manifests (manifest.json)
  workspaces   - workspace/layout definition files
  plugins      - native plug-in modules (.8bf/.8bi/.8li/.8be/.8bx/.8ba) by folder
  presets      - preset files by family with size + sha256 (binary parsing is a later pass)

Outputs under <out>/<app>/offline/. Reference material only.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import inspect
import json
import os
import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

APP_ROOTS = {
    "photoshop": {"install": r"C:\Program Files\Adobe\Adobe Photoshop 2026", "user": r"%APPDATA%\Adobe\Adobe Photoshop 2026", "typelib_guid": "{E891EE9A-D0AE-4cb4-8871-F92C0109F18E}"},
    "illustrator": {"install": r"C:\Program Files\Adobe\Adobe Illustrator 2026", "user": r"%APPDATA%\Adobe\Adobe Illustrator 30 Settings", "typelib_guid": "{38E4E28D-F058-4D11-A6F0-9F70ABF345DC}"},
    "indesign": {"install": r"C:\Program Files\Adobe\Adobe InDesign 2026", "user": r"%APPDATA%\Adobe\InDesign", "typelib_guid": "{268860D4-B7CB-4DEF-A5D6-F7F9F9261D11}", "extra_roots": [r"C:\ProgramData\Adobe\InDesign"]},
    "aftereffects": {"install": r"C:\Program Files\Adobe\Adobe After Effects 2026", "user": r"%APPDATA%\Adobe\After Effects", "typelib_guid": None},
    "premiere": {"install": r"C:\Program Files\Adobe\Adobe Premiere Pro 2026", "user": r"%APPDATA%\Adobe\Premiere Pro", "typelib_guid": None},
    "media_encoder": {"install": r"C:\Program Files\Adobe\Adobe Media Encoder 2026", "user": r"%APPDATA%\Adobe\Adobe Media Encoder", "typelib_guid": None},
    "lightroom_classic": {"install": r"C:\Program Files\Adobe\Adobe Lightroom Classic", "user": r"%APPDATA%\Adobe\Lightroom", "typelib_guid": None},
    "lightroom": {"install": r"C:\Program Files\Adobe\Adobe Lightroom CC", "user": r"%APPDATA%\Adobe\Lightroom CC", "typelib_guid": None},
    # Affinity 3 (Canva) ships as an MSIX Store package; the package root is readable without elevation on this host.
    "dreamweaver": {"install": r"C:\Program Files\Adobe\Adobe Dreamweaver 2021", "user": r"%APPDATA%\Adobe\Dreamweaver CC 2021", "typelib_guid": None},
    "affinity": {"install": r"C:\Program Files\WindowsApps\Canva.Affinity_3.2.3.4646_x64__8a0j1tnjnt4a4", "user": r"%LOCALAPPDATA%\Packages\Canva.Affinity_8a0j1tnjnt4a4", "typelib_guid": None},
}
PLUGIN_EXT = {".8bf", ".8bi", ".8li", ".8be", ".8bx", ".8ba", ".8bp", ".8by", ".aex", ".prm", ".lrplugin"}
PRESET_EXT_FAMILY = {
    ".abr": "brushes", ".tpl": "tool_presets", ".aco": "swatches", ".ase": "swatches", ".acb": "color_books", ".grd": "gradients",
    ".pat": "patterns", ".asl": "styles", ".csh": "custom_shapes", ".shc": "contours", ".atn": "actions", ".kys": "keyboard_shortcuts",
    ".mnu": "menu_customization", ".psw": "workspaces", ".cube": "luts", ".3dl": "luts", ".look": "luts", ".csf": "color_settings",
    ".acv": "curves", ".alv": "levels", ".ahu": "hue_saturation", ".amp": "curves_arbitrary", ".ado": "duotones", ".blw": "black_and_white",
    ".cha": "channel_mixer", ".hdt": "hdr_toning", ".axt": "exposure", ".asp": "selective_color", ".ahs": "hue_sat", ".cmx": "color_range",
    ".jsx": "scripts", ".js": "scripts", ".jsxbin": "scripts", ".xmp": "raw_presets", ".lrtemplate": "lightroom_templates",
    ".prfpset": "premiere_effect_presets", ".ffx": "ae_animation_presets", ".aep": "ae_projects", ".prproj": "premiere_projects",
    # Affinity
    ".propcol": "affinity_property_collections", ".afstudio": "affinity_studio_presets", ".afbrushes": "brushes", ".afassets": "affinity_assets",
    ".afstyles": "styles", ".afpalette": "swatches", ".afmacros": "affinity_macros", ".aftemplate": "affinity_templates", ".afdesign": "affinity_documents",
    ".afphoto": "affinity_documents", ".afpub": "affinity_documents", ".af": "affinity_documents", ".affinity": "affinity_documents",
    ".strings": "ui_string_tables", ".icc": "icc_profiles", ".icm": "icc_profiles", ".ppd": "printer_descriptions",
    # Illustrator stores brush / symbol / graphic-style / swatch libraries as .ai documents; classify by parent folder.
    ".aia": "actions", ".irs": "save_for_web_settings", ".act": "color_tables", ".ai": "illustrator_library",
    ".vst": "vector_styles", ".idms": "indesign_snippets", ".indl": "indesign_libraries", ".indt": "indesign_templates",
}
# folder name (lowercased, matched on any path segment) -> preset family, overrides extension mapping
FOLDER_FAMILY = {
    "brushes": "brushes", "graphic styles": "styles", "symbols": "symbols", "swatches": "swatches",
    "actions": "actions", "workspaces": "workspaces", "tools": "tool_presets", "scripts": "scripts",
    "save for web settings": "export_presets", "gradients": "gradients", "patterns": "patterns",
    "keyboard shortcuts": "keyboard_shortcuts", "color books": "color_books", "3dluts": "luts",
}


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def sha256_file(p: Path, limit_mb: int = 64) -> str | None:
    try:
        if p.stat().st_size > limit_mb * 1024 * 1024:
            return None
        h = hashlib.sha256()
        with p.open("rb") as fh:
            for chunk in iter(lambda: fh.read(1 << 20), b""):
                h.update(chunk)
        return h.hexdigest()
    except OSError:
        return None


def write_json(path: Path, data) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")


def typelib_from_registry(guid: str | None) -> tuple[str | None, str | None]:
    """Return (typelib_path, version) from HKCR\\TypeLib\\{guid}."""
    if not guid:
        return None, None
    try:
        import winreg  # type: ignore

        with winreg.OpenKey(winreg.HKEY_CLASSES_ROOT, rf"TypeLib\{guid}") as k:
            i = 0
            while True:
                try:
                    ver = winreg.EnumKey(k, i)
                except OSError:
                    break
                i += 1
                for arch in ("win64", "win32"):
                    try:
                        with winreg.OpenKey(k, rf"{ver}\0\{arch}") as vk:
                            path, _ = winreg.QueryValueEx(vk, "")
                            return path, ver
                    except OSError:
                        continue
    except OSError:
        pass
    return None, None


def typelib_dump(typelib_path: str) -> dict:
    from win32com.client import makepy  # type: ignore
    from win32com.client import gencache  # type: ignore
    import pythoncom  # type: ignore

    tlb = pythoncom.LoadTypeLib(typelib_path)
    attr = tlb.GetLibAttr()
    guid, lcid, major, minor = attr[0], attr[1], attr[3], attr[4]
    module = gencache.EnsureModule(guid, lcid, major, minor)
    if module is None:
        makepy.GenerateFromTypeLibSpec(typelib_path)
        module = gencache.EnsureModule(guid, lcid, major, minor)
    if module is None:
        return {"error": "makepy failed", "typelib": typelib_path}

    classes = {}
    for name, cls in inspect.getmembers(module, inspect.isclass):
        if name.startswith("_") or cls.__module__ != module.__name__:
            continue
        props = {}
        for attr_name in ("_prop_map_get_", "_prop_map_put_"):
            pm = getattr(cls, attr_name, None)
            if isinstance(pm, dict):
                for prop in pm:
                    props.setdefault(prop, {"get": False, "put": False})
                    props[prop]["get" if attr_name.endswith("get_") else "put"] = True
        methods = []
        for m_name, m in inspect.getmembers(cls, inspect.isfunction):
            if m_name.startswith("_"):
                continue
            try:
                sig = str(inspect.signature(m))
            except (TypeError, ValueError):
                sig = "(...)"
            methods.append({"name": m_name, "signature": sig, "doc": (m.__doc__ or "").strip()[:160]})
        base = [b.__name__ for b in cls.__bases__]
        entry = {"bases": base, "doc": (cls.__doc__ or "").strip()[:200], "properties": props, "methods": methods}
        # A CoClass (e.g. Application) carries no members itself; they live on its default interface.
        di = getattr(cls, "default_interface", None)
        if di is not None:
            entry["default_interface"] = getattr(di, "__name__", str(di))
            if not props and not methods:
                for attr_name in ("_prop_map_get_", "_prop_map_put_"):
                    pm = getattr(di, attr_name, None)
                    if isinstance(pm, dict):
                        for prop in pm:
                            props.setdefault(prop, {"get": False, "put": False})
                            props[prop]["get" if attr_name.endswith("get_") else "put"] = True
                for m_name, m in inspect.getmembers(di, inspect.isfunction):
                    if m_name.startswith("_"):
                        continue
                    try:
                        sig = str(inspect.signature(m))
                    except (TypeError, ValueError):
                        sig = "(...)"
                    methods.append({"name": m_name, "signature": sig, "doc": (m.__doc__ or "").strip()[:160]})
                entry["properties"], entry["methods"] = props, methods
                entry["members_resolved_via"] = "default_interface"
        classes[name] = entry

    constants = {}
    const_obj = getattr(module, "constants", None)
    dicts = getattr(const_obj, "__dicts__", None) if const_obj is not None else None
    if dicts:
        for d in dicts:
            constants.update(d)
    # enum grouping: makepy names constants by enum member; group by the module's enum map when present
    enums = {}
    for k, v in vars(module).items():
        if isinstance(v, dict) and k.endswith("Enum") is False and k.startswith("Ps") is False:
            continue
    return {
        "typelib": typelib_path,
        "guid": str(guid),
        "version": f"{major}.{minor}",
        "module": module.__name__,
        "class_count": len(classes),
        "constant_count": len(constants),
        "classes": classes,
        "constants": constants,
        "enum_groups": enums,
    }


def parse_kys(path: Path) -> dict:
    raw = path.read_bytes()
    head = raw[:64]
    result = {"file": str(path), "size": len(raw), "format": None}
    text = None
    for enc in ("utf-8-sig", "utf-16", "latin-1"):
        try:
            text = raw.decode(enc)
            if "<" in text[:200]:
                break
        except UnicodeDecodeError:
            continue
    if text and text.lstrip().startswith("<"):
        try:
            root = ET.fromstring(text)
            result["format"] = "xml"
            rows = []

            def walk(el, path_parts):
                tag = el.tag
                attrs = dict(el.attrib)
                parts = path_parts + [attrs.get("name") or attrs.get("id") or tag]
                if any(k.lower().startswith(("key", "shortcut", "cmd", "command")) for k in attrs):
                    rows.append({"path": "/".join(parts), "tag": tag, "attrs": attrs})
                for c in el:
                    walk(c, parts)

            walk(root, [])
            result["root_tag"] = root.tag
            result["root_attrs"] = dict(root.attrib)
            result["rows"] = rows
            result["row_count"] = len(rows)
            return result
        except ET.ParseError as exc:
            result["xml_error"] = str(exc)
    result["format"] = "binary_or_unknown"
    result["head_hex"] = head.hex()
    # crude string scrape for binary sets
    strings = re.findall(rb"[\x20-\x7e]{4,}", raw)
    result["ascii_strings_sample"] = [s.decode("ascii") for s in strings[:400]]
    result["ascii_string_count"] = len(strings)
    return result


def harvest_tree(root: Path, rel_label: str) -> tuple[list[dict], dict]:
    files = []
    families: dict[str, int] = {}
    if not root.exists():
        return files, families
    for dirpath, _dirs, filenames in os.walk(root):
        for fn in filenames:
            p = Path(dirpath) / fn
            ext = p.suffix.lower()
            fam = PRESET_EXT_FAMILY.get(ext)
            segs = {s.lower() for s in p.parts}
            for folder, ffam in FOLDER_FAMILY.items():
                if folder in segs:
                    fam = ffam
                    break
            kind = "plugin" if ext in PLUGIN_EXT else ("preset:" + fam if fam else "other")
            families[kind] = families.get(kind, 0) + 1
            try:
                size = p.stat().st_size
            except OSError:
                size = None
            files.append({"root": rel_label, "path": str(p.relative_to(root)).replace("\\", "/"), "ext": ext, "kind": kind, "size": size})
    return files, families


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--app", choices=sorted(APP_ROOTS), required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--hash", action="store_true", help="sha256 preset files (slower)")
    args = ap.parse_args()

    cfg = APP_ROOTS[args.app]
    install = Path(cfg["install"])
    user = Path(os.path.expandvars(cfg["user"]))
    out = args.out / args.app / "offline"
    record = {"harvester_id": f"handshake.adobe.{args.app}.install_harvest.v1", "harvested_at": now_iso(), "install_root": str(install), "user_root": str(user), "install_exists": install.exists(), "user_exists": user.exists(), "channels": {}, "errors": []}
    if not install.exists():
        write_json(out / "harvest_record.json", record)
        print(f"[harvest] install root missing: {install}")
        return 2

    # typelib
    tl_path, tl_ver = typelib_from_registry(cfg["typelib_guid"])
    if tl_path:
        try:
            tl = typelib_dump(tl_path)
            write_json(out / "dom_typelib.json", tl)
            record["channels"]["typelib"] = {"file": "dom_typelib.json", "source": tl_path, "registry_version": tl_ver, "class_count": tl.get("class_count"), "constant_count": tl.get("constant_count")}
            print(f"[harvest] typelib classes={tl.get('class_count')} constants={tl.get('constant_count')}")
        except Exception as exc:  # noqa: BLE001
            record["errors"].append(f"typelib: {exc}")
    else:
        record["channels"]["typelib"] = {"status": "no registered typelib guid configured"}

    # shortcuts (.kys) in install + user trees
    kys_files = list(install.rglob("*.kys")) + (list(user.rglob("*.kys")) if user.exists() else [])
    shortcuts = []
    for k in kys_files:
        try:
            shortcuts.append(parse_kys(k))
        except Exception as exc:  # noqa: BLE001
            record["errors"].append(f"kys {k}: {exc}")
    if shortcuts:
        write_json(out / "keyboard_shortcuts.json", {"files": shortcuts})
        record["channels"]["shortcuts"] = {"file": "keyboard_shortcuts.json", "sets": [{"file": s["file"], "format": s["format"], "rows": s.get("row_count")} for s in shortcuts]}
        print(f"[harvest] shortcut sets={len(shortcuts)}")

    # UXP manifests
    manifests = []
    for m in install.rglob("manifest.json"):
        try:
            data = json.loads(m.read_text(encoding="utf-8-sig"))
            manifests.append({"path": str(m.relative_to(install)).replace("\\", "/"), "id": data.get("id"), "name": data.get("name"), "version": data.get("version"), "host": data.get("host"), "entrypoints": data.get("entrypoints"), "requiredPermissions": data.get("requiredPermissions"), "manifestVersion": data.get("manifestVersion")})
        except Exception as exc:  # noqa: BLE001
            manifests.append({"path": str(m), "error": str(exc)[:120]})
    write_json(out / "uxp_manifests.json", {"count": len(manifests), "manifests": manifests})
    record["channels"]["uxp"] = {"file": "uxp_manifests.json", "count": len(manifests)}
    print(f"[harvest] uxp manifests={len(manifests)}")

    # workspaces / layouts
    ws = []
    for pat in ("Required/Workspaces/**/*", "Required/layouts/**/*", "Required/drover_layouts/**/*", "Presets/Workspaces/**/*"):
        for p in install.glob(pat):
            if p.is_file():
                ws.append({"path": str(p.relative_to(install)).replace("\\", "/"), "size": p.stat().st_size})
    if user.exists():
        for p in user.rglob("*"):
            if p.is_file() and ("WorkSpaces" in str(p) or p.suffix.lower() == ".psw"):
                ws.append({"path": "USER/" + str(p.relative_to(user)).replace("\\", "/"), "size": p.stat().st_size})
    write_json(out / "workspaces.json", {"count": len(ws), "files": ws})
    record["channels"]["workspaces"] = {"file": "workspaces.json", "count": len(ws)}

    # full tree manifest (install + user)
    files_i, fam_i = harvest_tree(install, "INSTALL")
    files_u, fam_u = harvest_tree(user, "USER")
    if args.hash:
        for f in files_i + files_u:
            if f["kind"].startswith("preset:") or f["kind"] == "plugin":
                base = install if f["root"] == "INSTALL" else user
                f["sha256"] = sha256_file(base / f["path"])
    plugins = [f for f in files_i if f["kind"] == "plugin"]
    presets = [f for f in files_i + files_u if f["kind"].startswith("preset:")]
    write_json(out / "tree_manifest.json", {"install_file_count": len(files_i), "user_file_count": len(files_u), "install_kinds": fam_i, "user_kinds": fam_u, "files": files_i + files_u})
    write_json(out / "plugins.json", {"count": len(plugins), "plugins": plugins})
    fam_counts: dict[str, int] = {}
    for f in presets:
        fam_counts[f["kind"]] = fam_counts.get(f["kind"], 0) + 1
    write_json(out / "presets.json", {"count": len(presets), "by_family": fam_counts, "files": presets})
    record["channels"]["tree"] = {"file": "tree_manifest.json", "install_files": len(files_i), "user_files": len(files_u)}
    record["channels"]["plugins"] = {"file": "plugins.json", "count": len(plugins)}
    record["channels"]["presets"] = {"file": "presets.json", "count": len(presets), "by_family": fam_counts}
    print(f"[harvest] files install={len(files_i)} user={len(files_u)} plugins={len(plugins)} presets={len(presets)}")
    write_json(out / "harvest_record.json", record)
    print(f"[harvest] wrote {out}")
    return 0 if not record["errors"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
