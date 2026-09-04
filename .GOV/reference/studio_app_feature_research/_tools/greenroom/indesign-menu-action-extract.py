#!/usr/bin/env python3
"""Handshake Studio green room: extract InDesign's real command surface (offline).

The idrc survey showed where the clean surface lives:
  idrc_MENR  menu resources   -> full menu paths, e.g. "Main:&Edit:Preferences", "Main:&View:Grids && Guides"
  idrc_ACTD  action resources -> action/command names, e.g. "Actual Size", "Add All Unnamed Colors", "Composite Font..."
             plus KBSCE shortcut-category rows
  idrc_PMST  string tables    -> panel/dialog strings, but in all 33 locales (noise); English-only filtered here

Output: indesign_command_surface.json  (menus tree + actions + panels), reference material only.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

ASCII = re.compile(rb"[\x20-\x7e]{3,}")
UTF16 = re.compile(rb"(?:[\x20-\x7e]\x00){3,}")
MENUPATH = re.compile(r"^(Main|RtMouse[A-Za-z]*|[A-Za-z]+Popup|[A-Za-z]+Menu|Panel[A-Za-z]*)(:[^:]{0,60})+$")
IDENT = re.compile(r"^[A-Z][A-Za-z0-9]*([A-Z][a-z0-9]+)+\d*$")  # CamelCaseIdentifier
SENTENCE = re.compile(r"\b(the|is|are|was|were|will|would|cannot|could|please|because|your|you|this document|not be)\b", re.I)
ACTIONISH = re.compile(r"^[A-Z][A-Za-z0-9][A-Za-z0-9 .,'/&()\-…]{1,60}(\.\.\.)?$")
KBSCE = re.compile(r"^KBSCE\s+(.*)$")


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def strings(data: bytes) -> list[str]:
    out = [m.group().decode("latin-1") for m in ASCII.finditer(data)]
    out += [m.group().decode("utf-16-le", errors="ignore") for m in UTF16.finditer(data)]
    return out


def is_english(s: str) -> bool:
    return all(ord(c) < 128 for c in s)


def clean_menu_label(part: str) -> str:
    p = part.replace("&&", "\x00").replace("&", "").replace("\x00", "&").strip()
    return p


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    menu_paths: set[str] = set()
    actions: set[str] = set()
    shortcut_cats: set[str] = set()
    panel_strings: set[str] = set()
    owners: dict[str, set] = collections.defaultdict(set)
    counts = collections.Counter()

    for p in args.root.rglob("*.idrc"):
        code = p.parent.name.replace("idrc_", "")
        if code not in ("MENR", "ACTD", "PMST"):
            continue
        owner = "APP_ROOT"
        for part in p.parts:
            if part.startswith("(") and part.endswith("Resources)"):
                owner = part.strip("()").replace(" Resources", "")
        try:
            data = p.read_bytes()
        except OSError:
            continue
        counts[code] += 1
        for s in strings(data):
            s = s.strip()
            if not s or not is_english(s):
                continue
            if code == "MENR":
                if MENUPATH.match(s) and ":" in s:
                    menu_paths.add(s)
                    owners[s].add(owner)
            elif code == "ACTD":
                m = KBSCE.match(s)
                if m:
                    v = m.group(1).strip(": ")
                    if v:
                        shortcut_cats.add(v)
                    continue
                if IDENT.match(s) or SENTENCE.search(s):
                    continue
                if ACTIONISH.match(s) and 2 < len(s) <= 60:
                    actions.add(s)
                    owners[s].add(owner)
            else:  # PMST, English-only panel/dialog vocabulary
                if IDENT.match(s) or SENTENCE.search(s):
                    continue
                if ACTIONISH.match(s) and 2 < len(s) <= 48 and s.count(" ") <= 5:
                    panel_strings.add(s)

    # build menu tree
    tree: dict = {}
    leaves = []
    for mp in menu_paths:
        parts = [clean_menu_label(x) for x in mp.split(":")]
        parts = [p for p in parts if p and p != "-"]
        node = tree
        for part in parts:
            node = node.setdefault(part, {})
        if len(parts) > 1:
            leaves.append({"path": " > ".join(parts), "root": parts[0], "depth": len(parts), "leaf": parts[-1], "owners": sorted(owners.get(mp, []))[:4]})

    roots = collections.Counter(l["root"] for l in leaves)
    doc = {
        "schema_id": "handshake.reference.indesign_command_surface@1",
        "generated_at": now(),
        "source": str(args.root),
        "method": "Offline extraction from idrc_MENR (menu resources), idrc_ACTD (action resources) and idrc_PMST (string tables, English-only). No app launched. Heuristic filters drop CamelCase internal identifiers, sentences, and non-ASCII locale strings.",
        "files_scanned": dict(counts),
        "totals": {"menu_paths": len(menu_paths), "menu_leaves": len(leaves), "actions": len(actions), "shortcut_categories": len(shortcut_cats), "panel_strings": len(panel_strings)},
        "menu_roots": dict(roots.most_common()),
        "menu_tree": tree,
        "menu_leaves": sorted(leaves, key=lambda l: l["path"]),
        "actions": sorted(actions),
        "shortcut_categories": sorted(shortcut_cats),
        "panel_strings": sorted(panel_strings),
    }
    out = args.out / "indesign_command_surface.json"
    out.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[indesign] menu_paths={len(menu_paths)} leaves={len(leaves)} actions={len(actions)} shortcut_cats={len(shortcut_cats)} panel_strings={len(panel_strings)}")
    print(f"[indesign] menu roots: {dict(roots.most_common(12))}")
    print(f"[indesign] sample menu leaves: {[l['path'] for l in sorted(leaves, key=lambda x: x['path'])[:10]]}")
    print(f"[indesign] sample actions: {sorted(actions)[:15]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
