#!/usr/bin/env python3
"""Handshake Studio green room: Figma object model from its public TypeScript declarations.

Figma has no installed binary to tear apart, but it publishes its complete document model as
TypeScript declarations on npm (@figma/plugin-typings, @figma/widget-typings). Those declare
every node type, every property with its type and mutability, every method with its signature,
and every enumerated string union. That is the same parameter surface recovered from type
libraries and plug-in resources for the installed applications, already in readable form and
obtainable without a Figma account.

Parses the .d.ts sources into a typed model. No browser, no login, no app.

Output: figma_object_model.json
"""
from __future__ import annotations

import argparse
import collections
import datetime as dt
import json
import re
from pathlib import Path

IFACE = re.compile(r"^(?:export\s+)?interface\s+([A-Za-z0-9_]+)(?:<[^>]*>)?(?:\s+extends\s+([^{]+))?\s*\{", re.M)
TYPEALIAS = re.compile(r"^(?:export\s+)?(?:declare\s+)?type\s+([A-Za-z0-9_]+)(?:<[^>]*>)?\s*=\s*(.+?)(?=^(?:export\s+)?(?:declare\s+)?(?:type|interface|const|function)\s|\Z)", re.M | re.S)
MEMBER = re.compile(r"^\s{2,}(readonly\s+)?([A-Za-z_][A-Za-z0-9_]*)(\?)?\s*(\??:|\()", re.M)
DOCBLOCK = re.compile(r"/\*\*(.*?)\*/\s*$", re.S)


def now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def strip_comments(src: str) -> str:
    src = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group().count("\n"), src, flags=re.S)
    src = re.sub(r"//[^\n]*", "", src)
    return src


def block_of(src: str, open_idx: int) -> str:
    depth = 0
    for i in range(open_idx, len(src)):
        c = src[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return src[open_idx : i + 1]
    return src[open_idx:]


def parse_members(body: str) -> list[dict]:
    out = []
    depth = 0
    for raw in body.splitlines():
        line = raw.rstrip()
        stripped = line.strip()
        if not stripped or stripped in ("{", "}"):
            depth += line.count("{") - line.count("}")
            continue
        if depth <= 1 and re.match(r"^(readonly\s+)?[A-Za-z_$][\w$]*\??\s*[:(<]", stripped):
            ro = bool(re.match(r"^readonly\s", stripped))
            m = re.match(r"^(?:readonly\s+)?([A-Za-z_$][\w$]*)(\?)?\s*([:(<])(.*)$", stripped)
            if m:
                name, opt, sig, rest = m.group(1), bool(m.group(2)), m.group(3), m.group(4)
                kind = "method" if sig == "(" or sig == "<" else "property"
                typ = rest.strip().rstrip(",;")
                if kind == "property":
                    typ = typ.lstrip(":").strip().rstrip(",;")
                literals = re.findall(r"'([^']+)'", typ)
                out.append({
                    "name": name, "kind": kind, "readonly": ro, "optional": opt,
                    "type": typ[:400],
                    "enumerated_values": sorted(set(literals)) if literals and kind == "property" else [],
                })
        depth += line.count("{") - line.count("}")
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--src", type=Path, required=True, help="directory holding the extracted npm packages")
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    files = [p for p in sorted(args.src.rglob("*.d.ts")) if "standalone" not in p.name]
    interfaces: dict[str, dict] = {}
    aliases: dict[str, dict] = {}
    per_file = []
    for p in files:
        raw = p.read_text(encoding="utf-8", errors="replace")
        src = strip_comments(raw)
        n_i = n_t = 0
        for m in IFACE.finditer(src):
            name, extends = m.group(1), (m.group(2) or "").strip()
            body = block_of(src, src.index("{", m.end() - 1))
            members = parse_members(body)
            interfaces[name] = {
                "name": name,
                "extends": [e.strip() for e in extends.split(",") if e.strip()],
                "source": p.name,
                "member_count": len(members),
                "property_count": sum(1 for x in members if x["kind"] == "property"),
                "method_count": sum(1 for x in members if x["kind"] == "method"),
                "members": members,
            }
            n_i += 1
        for m in TYPEALIAS.finditer(src):
            name, rhs = m.group(1), m.group(2).strip().rstrip(";")
            lits = re.findall(r"'([^']+)'", rhs)
            aliases[name] = {
                "name": name, "source": p.name, "definition": rhs[:600],
                "is_string_union": bool(lits) and "|" in rhs,
                "values": sorted(set(lits)),
            }
            n_t += 1
        per_file.append({"file": p.name, "bytes": len(raw), "interfaces": n_i, "type_aliases": n_t})

    node_types = {k: v for k, v in interfaces.items() if k.endswith("Node")}
    mixins = {k: v for k, v in interfaces.items() if k.endswith("Mixin")}
    enums = {k: v for k, v in aliases.items() if v["is_string_union"]}

    # resolve each node type's full property surface through its mixin chain
    def resolve(name: str, seen: set | None = None) -> list[dict]:
        seen = seen or set()
        if name in seen or name not in interfaces:
            return []
        seen.add(name)
        out = list(interfaces[name]["members"])
        for base in interfaces[name]["extends"]:
            base = re.sub(r"<.*?>", "", base).strip()
            out.extend(resolve(base, seen))
        return out

    resolved_nodes = []
    for name in sorted(node_types):
        members = resolve(name)
        dedup = {}
        for m in members:
            dedup.setdefault(m["name"], m)
        props = [m for m in dedup.values() if m["kind"] == "property"]
        meths = [m for m in dedup.values() if m["kind"] == "method"]
        resolved_nodes.append({
            "node_type": name,
            "extends": interfaces[name]["extends"],
            "resolved_property_count": len(props),
            "resolved_method_count": len(meths),
            "properties": sorted(props, key=lambda x: x["name"]),
            "methods": sorted(meths, key=lambda x: x["name"]),
        })

    total_props = sum(v["property_count"] for v in interfaces.values())
    total_meths = sum(v["method_count"] for v in interfaces.values())
    doc = {
        "schema_id": "handshake.reference.figma_object_model@1",
        "generated_at": now(),
        "app": "figma",
        "app_launched": False,
        "source": {
            "packages": [str(p.parent.name) for p in files],
            "files": per_file,
            "provenance": "Public npm packages @figma/plugin-typings and @figma/widget-typings. Obtained without a Figma account, browser session or desktop install.",
            "licence_note": "Figma publishes these declarations for plugin authors. They are read here as an interface description; no Figma code is copied into Studio. Studio ships Handshake-native names per STU-SECTION-003.",
        },
        "method": "TypeScript declaration parse. Interfaces, their inheritance chains and their members are read directly; string-literal unions are treated as enumerations; node types resolve their full property surface through the mixin chain.",
        "totals": {
            "interfaces": len(interfaces), "type_aliases": len(aliases),
            "declared_properties": total_props, "declared_methods": total_meths,
            "node_types": len(node_types), "mixins": len(mixins), "string_union_enums": len(enums),
            "enumerated_values": sum(len(v["values"]) for v in enums.values()),
        },
        "node_types": resolved_nodes,
        "mixins": sorted(mixins.values(), key=lambda x: x["name"]),
        "enums": sorted(enums.values(), key=lambda x: x["name"]),
        "interfaces": sorted(interfaces.values(), key=lambda x: x["name"]),
        "type_aliases": sorted(aliases.values(), key=lambda x: x["name"]),
    }
    outp = args.out / "figma_object_model.json"
    outp.write_text(json.dumps(doc, indent=1, ensure_ascii=False), encoding="utf-8", newline="\n")
    t = doc["totals"]
    print(f"[figma] interfaces={t['interfaces']} aliases={t['type_aliases']} props={t['declared_properties']} methods={t['declared_methods']}")
    print(f"[figma] node types={t['node_types']} mixins={t['mixins']} enums={t['string_union_enums']} enum values={t['enumerated_values']}")
    print(f"[figma] richest nodes: {[(n['node_type'], n['resolved_property_count']) for n in sorted(resolved_nodes, key=lambda x: -x['resolved_property_count'])[:8]]}")
    print(f"[figma] -> {outp}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
