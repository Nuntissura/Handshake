#!/usr/bin/env python3
"""Handshake Studio green room: passive UI Automation tree dump of a running app.

Walks every top-level window of the target process with the UIA backend and records
control type, name, automation id, class, rect, enabled/offscreen state, and supported
patterns. Passive mode never clicks, focuses, or sends input (HBR-QUIET). Active menu
expansion is a separate opt-in flag and is disclosed by the caller before use.

Run with the tool venv:
  python uia-crawl.py --process Photoshop --out <corpus>/ui_crawl/photoshop --label idle_no_document
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import time
from pathlib import Path

import psutil  # type: ignore
from pywinauto import Desktop  # type: ignore


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def find_pids(name: str) -> list[int]:
    pids = []
    for p in psutil.process_iter(["pid", "name"]):
        pname = (p.info["name"] or "").lower()
        if pname.startswith(name.lower()):
            pids.append(p.info["pid"])
    return pids


def node_info(elem) -> dict:
    ei = elem.element_info
    try:
        rect = ei.rectangle
        rect_d = {"l": rect.left, "t": rect.top, "r": rect.right, "b": rect.bottom}
    except Exception:  # noqa: BLE001
        rect_d = None
    patterns = []
    try:
        for attr in ("is_expanded", "is_selected", "is_checked", "get_value", "get_toggle_state"):
            if hasattr(elem, attr):
                patterns.append(attr)
    except Exception:  # noqa: BLE001
        pass
    d = {
        "control_type": getattr(ei, "control_type", None),
        "name": getattr(ei, "name", None),
        "automation_id": getattr(ei, "automation_id", None),
        "class_name": getattr(ei, "class_name", None),
        "framework_id": getattr(ei, "framework_id", None),
        "enabled": getattr(ei, "enabled", None),
        "visible": getattr(ei, "visible", None),
        "rect": rect_d,
        "patterns": patterns,
    }
    try:
        v = elem.legacy_properties().get("Value") if hasattr(elem, "legacy_properties") else None
        if v:
            d["legacy_value"] = str(v)[:200]
    except Exception:  # noqa: BLE001
        pass
    return d


def walk(elem, depth: int, max_depth: int, counter: dict, max_nodes: int) -> dict:
    d = node_info(elem)
    counter["n"] += 1
    if depth >= max_depth or counter["n"] >= max_nodes:
        d["truncated"] = True
        return d
    children = []
    try:
        for child in elem.children():
            children.append(walk(child, depth + 1, max_depth, counter, max_nodes))
            if counter["n"] >= max_nodes:
                break
    except Exception as exc:  # noqa: BLE001
        d["children_error"] = str(exc)[:200]
    if children:
        d["children"] = children
    return d


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--process", required=True, help="process name prefix, e.g. Photoshop")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--label", default="idle")
    ap.add_argument("--max-depth", type=int, default=40)
    ap.add_argument("--max-nodes", type=int, default=200000)
    args = ap.parse_args()

    pids = find_pids(args.process)
    if not pids:
        raise SystemExit(f"no running process matching {args.process}")
    desktop = Desktop(backend="uia")
    started = time.time()
    windows = []
    for w in desktop.windows():
        try:
            if w.process_id() in pids:
                windows.append(w)
        except Exception:  # noqa: BLE001
            continue
    counter = {"n": 0}
    dump = {
        "crawler_id": "handshake.greenroom.uia_crawl.v1",
        "captured_at": now_iso(),
        "process": args.process,
        "pids": pids,
        "label": args.label,
        "mode": "passive",
        "top_level_window_count": len(windows),
        "windows": [],
    }
    for w in windows:
        dump["windows"].append(walk(w, 0, args.max_depth, counter, args.max_nodes))
    dump["node_count"] = counter["n"]
    dump["elapsed_s"] = round(time.time() - started, 2)
    args.out.mkdir(parents=True, exist_ok=True)
    out = args.out / f"{args.label}.json"
    out.write_text(json.dumps(dump, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    # flat index of named controls for quick grep
    flat = []

    def flatten(n, path):
        nm = n.get("name") or ""
        p = path + [f"{n.get('control_type')}:{nm[:40]}"]
        if nm or n.get("automation_id"):
            flat.append({"path": "/".join(p), "control_type": n.get("control_type"), "name": nm, "automation_id": n.get("automation_id"), "class_name": n.get("class_name"), "enabled": n.get("enabled")})
        for c in n.get("children", []):
            flatten(c, p)

    for wnode in dump["windows"]:
        flatten(wnode, [])
    (args.out / f"{args.label}.flat.jsonl").write_text("\n".join(json.dumps(r, ensure_ascii=False) for r in flat) + "\n", encoding="utf-8", newline="\n")
    print(f"[uia] windows={len(windows)} nodes={counter['n']} named={len(flat)} elapsed={dump['elapsed_s']}s -> {out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
