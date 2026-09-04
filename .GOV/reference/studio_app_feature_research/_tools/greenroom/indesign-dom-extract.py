#!/usr/bin/env python3
"""Handshake Studio green room: extract InDesign's scripting object model from disk.

InDesign registers no type library until its scripting bridge initialises, and driving that
bridge over COM crashes the app. The model is however compiled into the plug-in resources:
idrc_SCE2 holds the scripting element tables (class descriptions, property and method names,
enumerator names) across ~98 plug-ins.

This reads those resources directly. No app is launched.

Output: indesign_dom.json
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
# "A cell style.", "A book.", "A behavior object that jumps to a URL."
DESC = re.compile(r"^(An?|The)\s+[a-z].{2,120}\.$")
# scripting identifiers are lowerCamelCase members or UpperCamelCase types
MEMBER = re.compile(r"^[a-z][a-z0-9]*(?:[A-Z][a-z0-9]+)+$")  # lowerCamelCase with a real word boundary
TYPE = re.compile(r"^[A-Z][a-z0-9]+(?:[A-Z][a-z0-9]+)*$")  # UpperCamelCase words, not 4CC tags
ENUMISH = re.compile(r"^[a-z][A-Za-z0-9]*([A-Z][A-Za-z0-9]*)+$")
NOISE = re.compile(r"^(k[A-Z]|D::|RESS|TSMP|APLN|RPLN)|33u?$|[0-9]{3,}$")
SENTENCE = re.compile(r"\b(cannot|could not|unable|please|error|failed|invalid|warning)\b", re.I)


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def strings(data: bytes) -> list[str]:
    out = [m.group().decode("latin-1") for m in ASCII.finditer(data)]
    out += [m.group().decode("utf-16-le", errors="ignore") for m in UTF16.finditer(data)]
    return [s.strip() for s in out if s.strip()]


def owner_of(p: Path) -> str:
    for part in p.parts:
        if part.startswith("(") and part.endswith("Resources)"):
            return part.strip("()").replace(" Resources", "")
    return "APP_ROOT"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--codes", default="SCE2")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    codes = {c.strip() for c in args.codes.split(",")}

    descriptions: dict[str, dict] = {}
    members: dict[str, set] = collections.defaultdict(set)
    types: dict[str, set] = collections.defaultdict(set)
    enums: dict[str, set] = collections.defaultdict(set)
    files = 0
    per_owner: dict[str, dict] = collections.defaultdict(lambda: {"descriptions": 0, "members": set(), "types": set()})

    for p in args.root.rglob("*.idrc"):
        code = p.parent.name.replace("idrc_", "")
        if code not in codes:
            continue
        files += 1
        owner = owner_of(p)
        try:
            data = p.read_bytes()
        except OSError:
            continue
        seq = strings(data)
        for i, s in enumerate(seq):
            if not all(ord(c) < 128 for c in s) or NOISE.search(s) or SENTENCE.search(s):
                continue
            if DESC.match(s):
                # the identifier for a description is usually the preceding short string
                subject = None
                for back in range(1, 4):
                    if i - back < 0:
                        break
                    cand = seq[i - back].strip()
                    if TYPE.match(cand) or MEMBER.match(cand):
                        subject = cand
                        break
                key = s
                d = descriptions.setdefault(key, {"description": s, "subjects": set(), "owners": set()})
                if subject:
                    d["subjects"].add(subject)
                d["owners"].add(owner)
                per_owner[owner]["descriptions"] += 1
            elif MEMBER.match(s) and len(s) >= 6:
                members[s].add(owner)
                per_owner[owner]["members"].add(s)
            elif TYPE.match(s) and len(s) >= 5:
                types[s].add(owner)
                per_owner[owner]["types"].add(s)
            if ENUMISH.match(s) and len(s) > 6:
                enums[s].add(owner)

    doc = {
        "schema_id": "handshake.reference.indesign_dom@1",
        "generated_at": now(),
        "source": str(args.root),
        "resource_codes": sorted(codes),
        "method": "Read directly from idrc scripting-element resources inside the installed plug-ins. No application launched and no COM scripting bridge used, because that bridge crashes the app (EXCEPTION_ACCESS_VIOLATION in ExtendScript.dll, 2026-09-04). Identifier classification is heuristic: sentence-form strings are treated as object descriptions, lowerCamelCase as properties/methods, UpperCamelCase as types.",
        "totals": {
            "resource_files": files,
            "object_descriptions": len(descriptions),
            "member_identifiers": len(members),
            "type_identifiers": len(types),
            "enum_candidates": len(enums),
            "plugins": len(per_owner),
        },
        "object_descriptions": sorted(({"description": v["description"], "subjects": sorted(v["subjects"])[:6], "owners": sorted(v["owners"])[:6]} for v in descriptions.values()), key=lambda x: x["description"].lower()),
        "types": sorted(({"name": k, "owners": sorted(v)[:6], "owner_count": len(v)} for k, v in types.items()), key=lambda x: x["name"]),
        "members": sorted(({"name": k, "owners": sorted(v)[:6], "owner_count": len(v)} for k, v in members.items()), key=lambda x: x["name"]),
        "enum_candidates": sorted(enums),
        "by_plugin": sorted(({"plugin": k, "descriptions": v["descriptions"], "member_count": len(v["members"]), "type_count": len(v["types"])} for k, v in per_owner.items()), key=lambda x: -x["descriptions"]),
    }
    outp = args.out / "indesign_dom.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    t = doc["totals"]
    print(f"[dom] files={t['resource_files']} descriptions={t['object_descriptions']} types={t['type_identifiers']} members={t['member_identifiers']} enums={t['enum_candidates']} plugins={t['plugins']}")
    print("[dom] sample descriptions:", [d["description"] for d in doc["object_descriptions"][:10]])
    print("[dom] sample types:", [x["name"] for x in doc["types"][:25]])
    print("[dom] sample members:", [x["name"] for x in doc["members"][:25]])
    print(f"[dom] -> {outp}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
