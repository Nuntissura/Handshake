#!/usr/bin/env python3
"""Handshake Studio green room: read InDesign preset CONTENTS offline (no app launch).

The install-tree harvest counted preset files; this reads what is inside them:

  Resources/Adobe PDF/settings/**/*.joboptions   PostScript-dict PDF export presets (every key/value)
  Resources/Adobe PDF/settings/Res/*.zdct        localized preset display names
  Presets/autocorrect/*.xml                      autocorrect word pairs per language
  Presets/Find-Change Queries/**/*.xml           stock find/change + GREP queries
  Presets/button library/*.indl                  stock button/sample library (binary; names scraped)
  Presets/default/*.iddx                         per-locale default document definitions (binary; names scraped)
  Presets/**/*.idms                              snippet libraries
  Resources/Dictionaries                         hyphenation/spelling dictionary inventory

Output: indesign_preset_contents.json
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
import xml.etree.ElementTree as ET
from pathlib import Path

JOB_KV = re.compile(r"/([A-Za-z0-9_]+)\s+(<<.*?>>|\[[^\]]*\]|\([^)]*\)|/[A-Za-z0-9_.]+|-?[\d.]+|true|false)", re.S)
ASCII = re.compile(rb"[\x20-\x7e]{4,}")
UTF16 = re.compile(rb"(?:[\x20-\x7e]\x00){4,}")


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def read_text(p: Path) -> str:
    raw = p.read_bytes()
    for enc in ("utf-8-sig", "utf-16", "utf-8", "latin-1"):
        try:
            return raw.decode(enc)
        except UnicodeDecodeError:
            continue
    return raw.decode("latin-1", errors="replace")


def parse_joboptions(p: Path) -> dict:
    text = read_text(p)
    settings = {}
    for m in JOB_KV.finditer(text):
        key, val = m.group(1), m.group(2).strip()
        if val.startswith("(") and val.endswith(")"):
            val = val[1:-1]
        elif val.startswith("/"):
            val = val[1:]
        elif val.startswith("<<"):
            val = re.sub(r"\s+", " ", val)[:400]
        settings[key] = val
    return {"preset": p.stem, "locale_dir": p.parent.name, "setting_count": len(settings), "settings": settings}


def scrape_names(p: Path, min_len: int = 3) -> list[str]:
    raw = p.read_bytes()
    out = []
    for m in ASCII.finditer(raw):
        s = m.group().decode("latin-1").strip()
        if min_len < len(s) <= 60 and re.match(r"^[A-Za-z][A-Za-z0-9 .,'/&()\-]*$", s):
            out.append(s)
    for m in UTF16.finditer(raw):
        s = m.group().decode("utf-16-le", errors="ignore").strip()
        if min_len < len(s) <= 60 and re.match(r"^[A-Za-z][A-Za-z0-9 .,'/&()\-]*$", s):
            out.append(s)
    seen, uniq = set(), []
    for s in out:
        if s not in seen:
            seen.add(s)
            uniq.append(s)
    return uniq


def parse_zdct(p: Path) -> dict:
    text = read_text(p)
    d = {}
    for m in re.finditer(r'"\$\$\$/([^=]+)=([^"]*)"', text):
        d[m.group(1)] = m.group(2)
    return d


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    root = args.root
    doc = {
        "schema_id": "handshake.reference.indesign_preset_contents@1",
        "generated_at": now(),
        "source": str(root),
        "method": "Offline read of InDesign preset files. PDF export presets are PostScript dictionaries and parsed key by key; XML presets are parsed; binary libraries have their readable names scraped. No app launched.",
        "sections": {},
    }

    # PDF export presets
    jobs = sorted((root / "Resources" / "Adobe PDF" / "settings").rglob("*.joboptions"))
    parsed = [parse_joboptions(p) for p in jobs]
    keys = collections.Counter()
    for j in parsed:
        keys.update(j["settings"].keys())
    doc["pdf_export_presets"] = {"count": len(parsed), "distinct_settings": len(keys), "setting_frequency": dict(keys.most_common()), "presets": parsed}
    doc["sections"]["pdf_export_presets"] = {"presets": len(parsed), "distinct_settings": len(keys)}

    # localized preset names
    zd = {}
    for p in sorted((root / "Resources" / "Adobe PDF" / "settings" / "Res").glob("*.zdct")):
        zd[p.stem] = parse_zdct(p)
    doc["pdf_preset_localised_names"] = {"locales": len(zd), "entries_per_locale": {k: len(v) for k, v in zd.items()}, "en": zd.get("ENU", {})}
    doc["sections"]["pdf_preset_localised_names"] = {"locales": len(zd)}

    # autocorrect
    auto = {}
    for p in sorted((root / "Presets" / "autocorrect").glob("*.xml")):
        try:
            tree = ET.parse(p)
            pairs = {}
            for el in tree.iter():
                a = el.attrib
                if len(a) >= 2:
                    vals = list(a.values())
                    pairs[vals[0]] = vals[1]
            auto[p.stem] = {"file": p.name, "pair_count": len(pairs), "sample": dict(list(pairs.items())[:12])}
        except ET.ParseError as e:
            auto[p.stem] = {"file": p.name, "error": str(e)[:100]}
    doc["autocorrect"] = auto
    doc["sections"]["autocorrect"] = {"languages": len(auto)}

    # find/change queries
    queries = []
    qroot = root / "Presets" / "Find-Change Queries"
    for p in sorted(qroot.rglob("*.xml")):
        text = read_text(p)
        entry = {"name": p.stem, "kind": p.parent.name, "file": str(p.relative_to(root)).replace("\\", "/")}
        for attr in ("findWhat", "changeTo", "appliedParagraphStyle", "appliedCharacterStyle"):
            m = re.search(rf'{attr}="([^"]*)"', text)
            if m:
                entry[attr] = m.group(1)
        if len(text) < 4000:
            entry["raw"] = text.strip()[:1500]
        queries.append(entry)
    doc["find_change_queries"] = {"count": len(queries), "by_kind": dict(collections.Counter(q["kind"] for q in queries)), "queries": queries}
    doc["sections"]["find_change_queries"] = {"count": len(queries)}

    # binary libraries: names scraped
    libs = {}
    for pattern, label in (("Presets/button library/*.indl", "button_library"), ("Presets/default/*.iddx", "default_documents"), ("Presets/**/*.idms", "snippets")):
        files = sorted(root.glob(pattern))
        rows = []
        for p in files:
            names = scrape_names(p)
            rows.append({"file": str(p.relative_to(root)).replace("\\", "/"), "bytes": p.stat().st_size, "name_count": len(names), "names": names[:150]})
        libs[label] = {"files": len(rows), "entries": rows}
        doc["sections"][label] = {"files": len(rows)}
    doc["binary_libraries"] = libs

    # dictionaries
    dic_root = root / "Plug-Ins" / "Dictionaries"
    dics = collections.Counter()
    dic_files = 0
    for p in dic_root.rglob("*"):
        if p.is_file():
            dic_files += 1
            dics[p.suffix.lower() or "<none>"] += 1
    doc["dictionaries"] = {"file_count": dic_files, "by_extension": dict(dics.most_common()), "languages": sorted({d.name for d in dic_root.iterdir() if d.is_dir()}) if dic_root.exists() else []}
    doc["sections"]["dictionaries"] = {"files": dic_files}

    outp = args.out / "indesign_preset_contents.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    print(f"[presets] PDF export presets: {len(parsed)} with {len(keys)} distinct settings")
    print(f"[presets] localised name locales: {len(zd)}")
    print(f"[presets] autocorrect languages: {len(auto)}")
    print(f"[presets] find/change queries: {len(queries)} {dict(collections.Counter(q['kind'] for q in queries))}")
    for k, v in libs.items():
        print(f"[presets] {k}: {v['files']} files")
    print(f"[presets] dictionaries: {dic_files} files, {len(doc['dictionaries']['languages'])} languages")
    print(f"[presets] -> {outp}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
