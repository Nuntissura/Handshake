#!/usr/bin/env python3
"""Handshake Studio green room: capture an installed Adobe app through COM.

Channels:
  typelib  - pywin32 gencache introspection of the app COM type library (full scripting DOM).
  jsx      - app.DoJavaScript(<jsx file>) returning JSON (menuBarInfo, presets, fonts, prefs).

Outputs land under <out>/<app>/ as JSON plus an export_record summary shaped after
32-adobe-installed-ui-export-playbook.md. Reference material only.

Run with the tool venv:
  ../Handshake_Artifacts/handshake-tool/studio-greenroom-venv/Scripts/python.exe adobe-com-capture.py --app photoshop --out <corpus>/installed_exports
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import inspect
import json
import sys
import time
from pathlib import Path

PROGIDS = {
    "photoshop": ["Photoshop.Application"],
    "illustrator": ["Illustrator.Application"],
    "indesign": ["InDesign.Application", "InDesign.Application.2026", "InDesign.Application.2025"],
}
JSX_FILES = {
    "photoshop": "photoshop-menubar-presets.jsx",
    "illustrator": "illustrator-menus-dom.jsx",
    "indesign": "indesign-menus-dom.jsx",
}
HERE = Path(__file__).resolve().parent


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def write_json(path: Path, data) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(data, indent=2, ensure_ascii=False, sort_keys=False)
    path.write_text(text, encoding="utf-8", newline="\n")
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def dispatch(app: str):
    import win32com.client  # type: ignore
    from win32com.client import gencache  # type: ignore

    # Order matters. Adobe apps started as hidden /Automation servers have refused
    # CoCreateInstance on this host, and EnsureDispatch fails for apps whose type library
    # is not registered (InDesign generates none until scripting support is initialised),
    # so plain Dispatch onto the already-running instance is tried before makepy.
    attempts = []
    for progid in PROGIDS[app]:
        for how, fn in (("GetActiveObject", win32com.client.GetActiveObject), ("Dispatch", win32com.client.Dispatch), ("EnsureDispatch", gencache.EnsureDispatch)):
            try:
                obj = fn(progid)
                print(f"[capture] attached via {how}({progid})")
                return obj, f"{progid} ({how})"
            except Exception as exc:  # noqa: BLE001
                attempts.append(f"{how}({progid}): {exc}")
    raise SystemExit("COM dispatch failed for " + app + ":\n  " + "\n  ".join(attempts))


def typelib_dump(app_obj) -> dict:
    """Introspect the gencache-generated module for the dispatched object."""
    module = sys.modules.get(type(app_obj).__module__)
    if module is None:
        return {"error": "no gencache module; dynamic dispatch only"}
    classes = {}
    for name, cls in inspect.getmembers(module, inspect.isclass):
        if name.startswith("_") or not cls.__module__ == module.__name__:
            continue
        entry = {"doc": (cls.__doc__ or "").strip()[:200]}
        props = {}
        for attr in ("_prop_map_get_", "_prop_map_put_"):
            pm = getattr(cls, attr, None)
            if isinstance(pm, dict):
                for prop in pm:
                    props.setdefault(prop, {"get": False, "put": False})
                    props[prop]["get" if attr.endswith("get_") else "put"] = True
        methods = []
        for m_name, m in inspect.getmembers(cls, inspect.isfunction):
            if m_name.startswith("_"):
                continue
            try:
                sig = str(inspect.signature(m))
            except (TypeError, ValueError):
                sig = "(...)"
            methods.append({"name": m_name, "signature": sig})
        entry["properties"] = props
        entry["methods"] = methods
        classes[name] = entry
    constants = {}
    const_obj = getattr(module, "constants", None)
    if const_obj is not None:
        for k, v in vars(const_obj).items():
            if not k.startswith("_"):
                constants[k] = v
    # gencache exposes enum constants through module.constants.__dicts__
    dicts = getattr(const_obj, "__dicts__", None)
    if dicts:
        for d in dicts:
            for k, v in d.items():
                constants[k] = v
    return {
        "module": module.__name__,
        "class_count": len(classes),
        "constant_count": len(constants),
        "classes": classes,
        "constants": constants,
    }


def run_jsx(app_obj, app: str, jsx_path: Path) -> dict:
    source = jsx_path.read_text(encoding="utf-8")
    started = time.time()
    if app == "photoshop":
        raw = app_obj.DoJavaScript(source, [], 1)
    elif app == "illustrator":
        raw = app_obj.DoJavaScript(source, [], 1)
    else:  # indesign
        raw = app_obj.DoScript(source, 1246973031)  # idJavascript
    elapsed = round(time.time() - started, 2)
    try:
        data = json.loads(raw)
    except Exception:  # noqa: BLE001
        data = {"raw": str(raw)[:20000], "parse_error": True}
    data["_jsx_elapsed_s"] = elapsed
    return data


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--app", choices=sorted(PROGIDS), required=True)
    ap.add_argument("--out", type=Path, required=True, help="installed_exports root")
    ap.add_argument("--skip-jsx", action="store_true")
    ap.add_argument("--skip-typelib", action="store_true")
    args = ap.parse_args()

    out_dir = args.out / args.app
    record = {
        "exporter_id": f"handshake.adobe.{args.app}.com_capture.v1",
        "exported_at": now_iso(),
        "app": args.app,
        "platform": "windows",
        "source_method": "script_export",
        "channels": {},
        "errors": [],
    }

    print(f"[capture] dispatching {args.app} via COM (app window may open)")
    app_obj, progid = dispatch(args.app)
    record["progid"] = progid
    try:
        record["app_version"] = str(getattr(app_obj, "Version", ""))
    except Exception as exc:  # noqa: BLE001
        record["errors"].append(f"version: {exc}")
    try:
        record["locale"] = str(getattr(app_obj, "Locale", ""))
    except Exception:  # noqa: BLE001
        pass

    if not args.skip_typelib:
        try:
            tl = typelib_dump(app_obj)
            sha = write_json(out_dir / "dom_typelib.json", tl)
            record["channels"]["typelib"] = {"file": "dom_typelib.json", "sha256": sha, "class_count": tl.get("class_count"), "constant_count": tl.get("constant_count")}
            print(f"[capture] typelib classes={tl.get('class_count')} constants={tl.get('constant_count')}")
        except Exception as exc:  # noqa: BLE001
            record["errors"].append(f"typelib: {exc}")

    if not args.skip_jsx:
        jsx_path = HERE / "jsx" / JSX_FILES[args.app]
        if not jsx_path.exists():
            record["errors"].append(f"jsx missing: {jsx_path.name}")
        else:
            try:
                data = run_jsx(app_obj, args.app, jsx_path)
                sha = write_json(out_dir / "jsx_capture.json", data)
                summary = {"file": "jsx_capture.json", "sha256": sha, "elapsed_s": data.get("_jsx_elapsed_s"), "errors": data.get("errors")}
                mb = data.get("menu_bar_info")
                if isinstance(mb, dict):
                    summary["menu_bar_top_level"] = len(mb.get("submenu", []) or mb.get("menu", []) or [])
                if isinstance(data.get("fonts"), list):
                    summary["font_count"] = len(data["fonts"])
                record["channels"]["jsx"] = summary
                print(f"[capture] jsx ok elapsed={summary['elapsed_s']}s errors={summary['errors']}")
            except Exception as exc:  # noqa: BLE001
                record["errors"].append(f"jsx: {exc}")

    write_json(out_dir / "export_record.json", record)
    print(f"[capture] wrote {out_dir}")
    return 0 if not record["errors"] else 2


if __name__ == "__main__":
    raise SystemExit(main())
