#!/usr/bin/env python3
"""Handshake Studio green room: survey InDesign .idrc resource containers (offline, no app launch).

InDesign ships its UI as ~29k .idrc resource files grouped by 4-character type code
(idrc_<CODE>/<id>.idrc), both at the app root and under each plug-in's "(X Resources)" folder.
This tool inventories the type codes, extracts readable strings, and classifies which codes
carry menus, panel/dialog definitions, ZStrings, and locale text, so a later pass can parse the
codes that matter instead of all 29k files.
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

ASCII = re.compile(rb"[\x20-\x7e]{5,}")
UTF16 = re.compile(rb"(?:[\x20-\x7e]\x00){5,}")
ZSTR = re.compile(r"\$\$\$/[A-Za-z0-9_./\-]+")
MENUISH = re.compile(r"^[A-Z][A-Za-z0-9 .,'/&()\-]{2,48}(\.\.\.)?$")


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def strings_of(data: bytes) -> tuple[list[str], list[str]]:
    a = [m.group().decode("latin-1") for m in ASCII.finditer(data)]
    u = [m.group().decode("utf-16-le", errors="ignore") for m in UTF16.finditer(data)]
    return a, u


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--max-bytes-per-code", type=int, default=24 * 1024 * 1024)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    files = list(args.root.rglob("*.idrc"))
    by_code: dict[str, dict] = collections.defaultdict(lambda: {"files": 0, "bytes": 0, "owners": set(), "read_bytes": 0, "ascii": 0, "utf16": 0, "zstrings": set(), "samples": [], "menuish": set()})
    for p in files:
        code = p.parent.name.replace("idrc_", "") or "?"
        owner = "APP_ROOT"
        for part in p.parts:
            if part.startswith("(") and part.endswith("Resources)"):
                owner = part.strip("()").replace(" Resources", "")
        rec = by_code[code]
        try:
            size = p.stat().st_size
        except OSError:
            continue
        rec["files"] += 1
        rec["bytes"] += size
        rec["owners"].add(owner)
        if rec["read_bytes"] < args.max_bytes_per_code and size < 4 * 1024 * 1024:
            try:
                data = p.read_bytes()
            except OSError:
                continue
            rec["read_bytes"] += size
            a, u = strings_of(data)
            rec["ascii"] += len(a)
            rec["utf16"] += len(u)
            for s in a + u:
                for z in ZSTR.findall(s):
                    rec["zstrings"].add(z)
                t = s.strip()
                if MENUISH.match(t) and not t.startswith("$$$"):
                    rec["menuish"].add(t)
            if len(rec["samples"]) < 30:
                rec["samples"].extend((a + u)[:6])

    codes = []
    for code, rec in sorted(by_code.items(), key=lambda kv: -kv[1]["bytes"]):
        zs = sorted(rec["zstrings"])
        mn = sorted(rec["menuish"])
        codes.append({
            "code": code, "files": rec["files"], "bytes": rec["bytes"], "scanned_bytes": rec["read_bytes"],
            "owner_count": len(rec["owners"]), "owners_sample": sorted(rec["owners"])[:8],
            "ascii_strings": rec["ascii"], "utf16_strings": rec["utf16"],
            "zstring_key_count": len(zs), "zstring_samples": zs[:40],
            "label_candidate_count": len(mn), "label_samples": mn[:60],
            "raw_samples": [s[:100] for s in rec["samples"][:12]],
        })
    all_z = set()
    all_labels = set()
    for c in by_code.values():
        all_z |= c["zstrings"]
        all_labels |= c["menuish"]
    doc = {
        "scanner_id": "handshake.indesign.idrc_scan.v1",
        "scanned_at": now(),
        "root": str(args.root),
        "method": "Heuristic string survey. Extracts printable ASCII and UTF-16LE runs, collects $$$/ ZString keys and title-case label candidates, groups by idrc type code and owning plug-in. Not a format parser; identifies which type codes are worth parsing properly.",
        "totals": {"idrc_files": len(files), "type_codes": len(by_code), "zstring_keys": len(all_z), "label_candidates": len(all_labels)},
        "zstring_keys": sorted(all_z),
        "label_candidates": sorted(all_labels),
        "type_codes": codes,
    }
    out = args.out / "indesign_idrc_survey.json"
    out.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[idrc] files={len(files)} codes={len(by_code)} zstring_keys={len(all_z)} labels={len(all_labels)} -> {out}")
    for c in codes[:18]:
        print(f"   {c['code']:8s} files={c['files']:6d} MB={c['bytes']/1048576:7.1f} owners={c['owner_count']:3d} z={c['zstring_key_count']:5d} labels={c['label_candidate_count']:5d}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
