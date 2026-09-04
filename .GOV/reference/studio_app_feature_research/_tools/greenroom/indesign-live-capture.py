#!/usr/bin/env python3
"""Handshake Studio green room: resilient InDesign live capture over COM.

Why this exists instead of one big DoScript: InDesign's scripting bridge on this host wedges
after a handful of calls (every subsequent DoScript returns "The server threw an exception"),
and a single large script loses everything when that happens. So the capture is split into many
small paged calls, each retried, with progress written to disk after every batch. If the bridge
wedges the tool restarts InDesign and resumes from where it stopped.

Output: <out>/indesign/live/{menu_actions,menus,panels,presets,preferences}.json + capture_record.json
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import time
from pathlib import Path

import pythoncom  # type: ignore
import win32com.client  # type: ignore
from win32com.client import dynamic  # type: ignore

JS = 1246973031
EXE = r"C:\Program Files\Adobe\Adobe InDesign 2026\InDesign.exe"


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


class Bridge:
    def __init__(self, restart_allowed: bool = True):
        self.app = None
        self.restarts = 0
        self.restart_allowed = restart_allowed
        self.connect()

    def connect(self, wait: int = 90):
        pythoncom.CoInitialize()
        deadline = time.time() + wait
        last = None
        while time.time() < deadline:
            try:
                self.app = dynamic.Dispatch("InDesign.Application")
                _ = self.app.Version
                return
            except Exception as e:  # noqa: BLE001
                last = e
                time.sleep(3)
        raise SystemExit(f"cannot attach to InDesign: {last}")

    def restart(self):
        if not self.restart_allowed:
            raise RuntimeError("bridge wedged and restart disabled")
        self.restarts += 1
        print(f"[bridge] restarting InDesign (restart #{self.restarts})")
        subprocess.run(["taskkill", "/IM", "InDesign.exe", "/F"], capture_output=True)
        time.sleep(8)
        subprocess.Popen([EXE])
        time.sleep(55)
        self.app = None
        self.connect()
        time.sleep(5)

    def run(self, src: str, tries: int = 3, delay: float = 1.0, allow_restart: bool = True):
        for attempt in range(tries):
            try:
                return True, self.app.DoScript(src, JS)
            except Exception as e:  # noqa: BLE001
                last = e
                time.sleep(delay)
        if allow_restart:
            try:
                self.restart()
                return self.run(src, tries=2, delay=1.0, allow_restart=False)
            except Exception as e2:  # noqa: BLE001
                return False, e2
        return False, last


def jsonl_expr(expr: str) -> str:
    """ExtendScript has no JSON in some hosts; build delimited text instead."""
    return expr


def page_collection(br: Bridge, coll: str, fields: list[str], batch: int = 120, limit: int | None = None) -> list[dict]:
    ok, n = br.run(f"app.{coll}.length;")
    if not ok:
        print(f"  {coll}: length failed -> {n}")
        return []
    total = int(n)
    if limit:
        total = min(total, limit)
    print(f"  {coll}: {total} items")
    rows: list[dict] = []
    i = 0
    while i < total:
        j = min(i + batch, total)
        parts = []
        for f in fields:
            parts.append(f'try{{ r.push(String(o.{f})); }}catch(e){{ r.push("<err>"); }}')
        src = (
            "var out=[];"
            f"for(var i={i};i<{j};i++){{"
            f"var o=app.{coll}[i]; var r=[];"
            + "".join(parts)
            + 'out.push(r.join("\\u0001"));'
            "}"
            'out.join("\\u0002");'
        )
        ok, val = br.run(src)
        if not ok:
            print(f"    batch {i}-{j} failed: {str(val)[:90]}")
            i = j
            continue
        for line in str(val).split("\u0002"):
            if not line:
                continue
            vals = line.split("\u0001")
            rows.append({f: (vals[k] if k < len(vals) else None) for k, f in enumerate(fields)})
        i = j
    return rows


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--no-restart", action="store_true")
    args = ap.parse_args()
    out = args.out / "indesign" / "live"
    out.mkdir(parents=True, exist_ok=True)
    br = Bridge(restart_allowed=not args.no_restart)
    rec = {"capture_id": "handshake.indesign.live_capture.v2", "started_at": now(), "sections": {}, "restarts": 0}

    def save(name: str, data, extra: dict | None = None):
        (out / f"{name}.json").write_text(json.dumps(data, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        rec["sections"][name] = {"count": len(data) if isinstance(data, list) else None, **(extra or {})}
        rec["restarts"] = br.restarts
        (out / "capture_record.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        print(f"  saved {name}: {len(data) if isinstance(data, list) else 'obj'}")

    ok, v = br.run("app.version;")
    rec["app_version"] = str(v) if ok else None
    print(f"[capture] InDesign {rec['app_version']}")

    print("[capture] menu actions")
    save("menu_actions", page_collection(br, "menuActions", ["name", "id", "enabled", "area", "keyboardShortcut"]))

    print("[capture] script menu actions")
    save("script_menu_actions", page_collection(br, "scriptMenuActions", ["name", "id", "enabled"]))

    print("[capture] menus")
    menus = page_collection(br, "menus", ["name", "title", "id"])
    save("menus", menus)

    print("[capture] menu tree")
    tree = []
    for idx in range(len(menus)):
        src = (
            f"var m=app.menus[{idx}]; var out=[];"
            "for(var i=0;i<m.menuElements.length;i++){var el=m.menuElements[i];var nm='';var act='';var sc='';"
            "try{nm=String(el.name);}catch(e){}"
            "try{if(el.associatedMenuAction){act=String(el.associatedMenuAction.name);sc=String(el.associatedMenuAction.keyboardShortcut);}}catch(e){}"
            'out.push(nm+"\\u0001"+act+"\\u0001"+sc);}'
            'out.join("\\u0002");'
        )
        ok, val = br.run(src, tries=2)
        if not ok:
            continue
        items = []
        for line in str(val).split("\u0002"):
            if not line:
                continue
            p = line.split("\u0001")
            items.append({"name": p[0], "action": p[1] if len(p) > 1 else "", "shortcut": p[2] if len(p) > 2 else ""})
        tree.append({"menu": menus[idx].get("name"), "items": items})
    save("menu_tree", tree)

    for label, coll, fields in [
        ("panels", "panels", ["name", "id", "visible"]),
        ("document_presets", "documentPresets", ["name", "pageWidth", "pageHeight", "facingPages", "columnCount", "columnGutter", "top", "bottom", "left", "right"]),
        ("pdf_export_presets", "pdfExportPresets", ["name", "standardsCompliance", "acrobatCompatibility", "colorBitmapCompression", "colorBitmapQuality", "includeHyperlinks", "exportLayers", "pdfMarkType"]),
        ("printer_presets", "printerPresets", ["name", "printer", "paperSize", "colorOutput"]),
        ("preflight_profiles", "preflightProfiles", ["name", "id", "description"]),
        ("flattener_presets", "flattenerPresets", ["name", "rasterVectorBalance", "lineArtAndTextResolution", "gradientAndMeshResolution"]),
        ("swatches", "swatches", ["name", "model", "space", "colorValue"]),
        ("paragraph_styles", "paragraphStyles", ["name", "appliedFont", "pointSize", "leading", "justification"]),
        ("character_styles", "characterStyles", ["name", "appliedFont", "pointSize"]),
        ("object_styles", "objectStyles", ["name", "enableFill", "enableStroke"]),
        ("table_styles", "tableStyles", ["name"]),
        ("cell_styles", "cellStyles", ["name"]),
        ("trap_presets", "trapPresets", ["name", "trapWidth", "blackWidth"]),
        ("mojikumi_tables", "mojikumiTables", ["name"]),
        ("kinsoku_tables", "kinsokuTables", ["name"]),
        ("composite_fonts", "compositeFonts", ["name"]),
        ("languages", "languagesWithVendors", ["name", "id", "hyphenationVendor", "spellingVendor"]),
        ("fonts", "fonts", ["name", "fontFamily", "fontStyleName", "postscriptName", "status", "fontType"]),
        ("xml_tags", "xmlTags", ["name"]),
        ("conditions", "conditions", ["name", "visible"]),
    ]:
        print(f"[capture] {label}")
        save(label, page_collection(br, coll, fields))

    print("[capture] preferences")
    prefs = {}
    pref_names = ["generalPreferences", "textPreferences", "textEditingPreferences", "storyPreferences", "documentPreferences", "viewPreferences", "gridPreferences", "guidePreferences", "marginPreferences", "transparencyPreferences", "linkingPreferences", "displayPerformancePreferences", "spellPreferences", "autoCorrectPreferences", "dictionaryPreferences", "footnoteOptions", "trackChangesPreferences", "epubExportPreferences", "clipboardPreferences", "smartGuidePreferences", "polygonPreferences", "printPreferences", "imageIOPreferences", "textDefaults", "pathFinderOptions"]
    for pn in pref_names:
        src = (
            f"var p=app.{pn}.properties; var out=[];"
            'for(var k in p){ var v=""; try{ v=String(p[k]); }catch(e){ v="<err>"; } out.push(k+"\\u0001"+v); }'
            'out.join("\\u0002");'
        )
        ok, val = br.run(src, tries=2)
        if not ok:
            prefs[pn] = {"error": str(val)[:120]}
            continue
        d = {}
        for line in str(val).split("\u0002"):
            if not line:
                continue
            p = line.split("\u0001")
            d[p[0]] = p[1] if len(p) > 1 else None
        prefs[pn] = d
        print(f"  {pn}: {len(d)} properties")
    save("preferences", prefs)

    rec["finished_at"] = now()
    (out / "capture_record.json").write_text(json.dumps(rec, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[capture] done, restarts={br.restarts} -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
