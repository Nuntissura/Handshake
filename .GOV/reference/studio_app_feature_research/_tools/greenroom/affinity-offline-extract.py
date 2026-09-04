#!/usr/bin/env python3
"""Handshake Studio green room: Affinity 3 offline extraction (no app launch).

Inputs: the readable mirror of the Canva.Affinity MSIX package (or the package root itself).
Outputs under <out>:
  ui_strings_lproj_<locale>.json   parsed Apple-style .strings tables ("key" = "value";, UTF-16)
  scripting_api_surface.json       classes, methods, getters, native `affinity:*` modules, enums, examples, tests
  ui_strings_dotnet_summary.json   per-resource-set counts over the .NET extraction produced by the PowerShell pass
Reference material only.
"""
from __future__ import annotations

import argparse
import datetime as dt
import json
import re
from pathlib import Path

STRINGS_RE = re.compile(r'^\s*"((?:[^"\\]|\\.)*)"\s*=\s*"((?:[^"\\]|\\.)*)"\s*;', re.M)
CLASS_RE = re.compile(r'^\s*class\s+([A-Za-z_][\w]*)\s*(?:extends\s+([\w.]+))?\s*\{', re.M)
MEMBER_RE = re.compile(r'^\s{4}(static\s+)?(get\s+|set\s+|async\s+)?(?:\[Symbol\.[\w]+\]|#?[A-Za-z_][\w]*)\s*\(([^)]*)\)\s*\{', re.M)
MEMBER_NAME_RE = re.compile(r'^\s{4}(?:static\s+)?(?:get\s+|set\s+|async\s+)?(\[Symbol\.[\w]+\]|#?[A-Za-z_][\w]*)\s*\(')
REQUIRE_RE = re.compile(r"require\('(affinity:[\w.-]+)'\)")
DESTRUCT_RE = re.compile(r"const\s*\{([^}]*)\}\s*=\s*require\('(affinity:[\w.-]+)'\)", re.S)
EXPORT_RE = re.compile(r"module\.exports\s*=\s*\{([^}]*)\}", re.S)


def now_iso() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def read_text_any(p: Path) -> str:
    raw = p.read_bytes()
    if raw.startswith(b"\xff\xfe") or raw.startswith(b"\xfe\xff"):
        return raw.decode("utf-16")
    if b"\x00" in raw[:64]:
        try:
            return raw.decode("utf-16-le")
        except UnicodeDecodeError:
            pass
    for enc in ("utf-8-sig", "utf-8", "latin-1"):
        try:
            return raw.decode(enc)
        except UnicodeDecodeError:
            continue
    return raw.decode("latin-1", errors="replace")


def parse_lproj(root: Path, locale: str) -> dict:
    d = root / "App" / "Resources" / "Affinity" / f"{locale}.lproj"
    tables = {}
    total = 0
    for f in sorted(d.glob("*.strings")) if d.exists() else []:
        text = read_text_any(f)
        entries = {}
        for m in STRINGS_RE.finditer(text):
            entries[m.group(1)] = m.group(2)
        tables[f.stem] = {"file": str(f.relative_to(root)).replace("\\", "/"), "count": len(entries), "entries": entries}
        total += len(entries)
    return {"locale": locale, "table_count": len(tables), "total_strings": total, "tables": tables}


def parse_jslib(root: Path) -> dict:
    d = root / "App" / "Resources" / "JSLib"
    modules = {}
    native_modules: dict[str, set] = {}
    for f in sorted(d.glob("*.js")):
        src = f.read_text(encoding="utf-8", errors="replace")
        classes = []
        for cm in CLASS_RE.finditer(src):
            name, base = cm.group(1), cm.group(2)
            # slice class body until next top-level class or EOF (heuristic)
            start = cm.end()
            nxt = CLASS_RE.search(src, start)
            body = src[start : nxt.start() if nxt else len(src)]
            members = []
            for line in body.splitlines():
                mm = MEMBER_NAME_RE.match(line)
                if mm:
                    kind = "method"
                    stripped = line.strip()
                    if stripped.startswith("get "):
                        kind = "getter"
                    elif stripped.startswith("set "):
                        kind = "setter"
                    elif stripped.startswith("static "):
                        kind = "static"
                    if stripped.startswith("async ") or " async " in stripped[:20]:
                        kind = "async_" + kind
                    nm = mm.group(1)
                    if nm in ("if", "for", "while", "switch", "catch", "constructor") and nm != "constructor":
                        continue
                    members.append({"name": nm, "kind": kind})
            classes.append({"name": name, "extends": base, "members": members, "member_count": len(members)})
        for dm in DESTRUCT_RE.finditer(src):
            mod = dm.group(2)
            names = [n.strip() for n in dm.group(1).replace("\n", " ").split(",") if n.strip()]
            native_modules.setdefault(mod, set()).update(names)
        for rm in REQUIRE_RE.finditer(src):
            native_modules.setdefault(rm.group(1), set())
        exports = []
        em = EXPORT_RE.search(src)
        if em:
            exports = [n.strip() for n in em.group(1).replace("\n", " ").split(",") if n.strip()]
        modules[f.stem] = {"file": f.name, "bytes": len(src), "classes": classes, "class_count": len(classes), "exports": exports}
    examples = [p.name for p in sorted((d / "examples").glob("*"))] if (d / "examples").exists() else []
    tests = [p.name for p in sorted((d / "tests").glob("*"))] if (d / "tests").exists() else []
    license_head = ""
    any_js = next(iter(sorted(d.glob("*.js"))), None)
    if any_js:
        head = any_js.read_text(encoding="utf-8", errors="replace")[:1200]
        lm = re.search(r"/\*(.*?)\*/", head, re.S)
        license_head = (lm.group(1).strip() if lm else head[:400])
    return {
        "module_count": len(modules),
        "class_total": sum(m["class_count"] for m in modules.values()),
        "native_modules": {k: sorted(v) for k, v in sorted(native_modules.items())},
        "native_module_count": len(native_modules),
        "examples": examples,
        "tests": tests,
        "license_header": license_head,
        "modules": modules,
    }


def summarize_dotnet(path: Path) -> dict:
    if not path.exists():
        return {"missing": str(path)}
    data = json.loads(path.read_text(encoding="utf-8-sig"))
    sets = []
    for asm in data.get("assemblies", []):
        for rs in asm.get("resource_sets", []):
            sets.append({"assembly": asm["file"].split("\\")[-1], "set": rs["name"], "strings": rs.get("string_count", len(rs.get("entries", {})))})
    sets.sort(key=lambda s: -s["strings"])
    return {"total_strings": data.get("total_strings"), "set_count": len(sets), "sets": sets}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", type=Path, required=True, help="mirror or package root containing App/")
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--locales", default="en-US,Base")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    record = {"extractor_id": "handshake.affinity.offline_extract.v1", "extracted_at": now_iso(), "root": str(args.root), "outputs": {}}

    for loc in [x.strip() for x in args.locales.split(",") if x.strip()]:
        res = parse_lproj(args.root, loc)
        outp = args.out / f"ui_strings_lproj_{loc}.json"
        outp.write_text(json.dumps(res, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
        record["outputs"][outp.name] = {"tables": res["table_count"], "strings": res["total_strings"]}
        print(f"[affinity] lproj {loc}: tables={res['table_count']} strings={res['total_strings']}")

    api = parse_jslib(args.root)
    outp = args.out / "scripting_api_surface.json"
    outp.write_text(json.dumps(api, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    record["outputs"][outp.name] = {"modules": api["module_count"], "classes": api["class_total"], "native_modules": api["native_module_count"], "examples": len(api["examples"]), "tests": len(api["tests"])}
    print(f"[affinity] jslib modules={api['module_count']} classes={api['class_total']} native={api['native_module_count']}")

    summ = summarize_dotnet(args.out / "ui_strings_dotnet_en-US.json")
    (args.out / "ui_strings_dotnet_summary.json").write_text(json.dumps(summ, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    record["outputs"]["ui_strings_dotnet_summary.json"] = {"sets": summ.get("set_count"), "strings": summ.get("total_strings")}
    (args.out / "extract_record.json").write_text(json.dumps(record, indent=1), encoding="utf-8", newline="\n")
    print(f"[affinity] wrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
