#!/usr/bin/env python3
"""Deep extraction of the Affinity 3.x JavaScript scripting API from JSLib.

Reads the shipped BSD-3 licensed sources under App/Resources/JSLib and produces
a per-member specification: class hierarchy, method and accessor signatures with
parameter names and defaults, the native `affinity:*` calls each member makes,
the native module import surface, and native enum member usage.

Everything here is read from the shipped .js sources.  Where a fact could only
be corroborated against the native binary (libscriptingjs.dll) that is recorded
as a separate boolean and never used to invent a name.

Usage:
  python affinity-jslib-extract.py --jslib <JSLib dir> --out <json path>
                                   [--native <libscriptingjs.dll>]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import re
from collections import Counter, OrderedDict

TOOL_ID = "handshake.affinity.jslib_extract.v1"

RE_REQ_DESTRUCT = re.compile(
    r"const\s*\{([^}]*)\}\s*=\s*require\(\s*['\"]([^'\"]+)['\"]\s*\)")
RE_REQ_PLAIN = re.compile(
    r"const\s+(\w+)\s*=\s*require\(\s*['\"]([^'\"]+)['\"]\s*\)")
RE_CLASS = re.compile(r"\bclass\s+(\w+)\s*(?:extends\s+([\w.]+)\s*)?\{")
RE_EXPORT = re.compile(r"module\.exports(?:\.(\w+))?\s*=\s*([\w.]+)\s*;")
RE_APICALL = re.compile(r"\b([A-Z]\w*Api)\.(\w+)\s*\(")
RE_NEWCALL = re.compile(r"\bnew\s+([A-Z]\w*)\s*\(")
RE_ENUMUSE = re.compile(r"\b([A-Z]\w*)\.([A-Za-z_]\w*)\b")
RE_MEMBER = re.compile(
    r"""^[ \t]*
        (?P<mods>(?:static\s+|async\s+|get\s+|set\s+|\*\s*)*)
        (?P<name>\#?[A-Za-z_$][\w$]*|\[[^\]]+\])
        \s*\((?P<args>[^)]*)\)\s*\{""",
    re.VERBOSE | re.M)
RE_FIELD = re.compile(r"^[ \t]*(?P<mods>(?:static\s+)*)(?P<name>#?[A-Za-z_$][\w$]*)\s*(?:=|;)")
RE_FUNC = re.compile(r"^function\s+(\w+)\s*\(([^)]*)\)\s*\{", re.M)



ENUM_MACHINERY = {"entries", "parse", "keys", "values", "equals", "isEnumValue",
                  "isEnum", "toString", "value", "string"}
RE_IDENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]{0,63}$")


def native_string_pool(blob):
    """Ordered (offset, text) list of the binary's NUL-terminated ASCII pool."""
    pool = []
    pat = re.compile(b"[\\x20-\\x7e]{2,120}\\x00")
    for m in pat.finditer(blob):
        pool.append((m.start(), m.group()[:-1].decode("ascii")))
    return pool


def enum_member_candidates(pool, index, enum_names, known_names, span=48):
    """Adjacency scan around each enum name in the native string pool.

    HEURISTIC. The binary stores enum member names as plain strings next to the
    enum's own name, but the pool is deduplicated and its ordering is not part
    of any documented contract, so the association below is positional evidence,
    not a parsed mapping.
    """
    out = []
    for name in sorted(enum_names):
        hits = index.get(name)
        if not hits:
            continue
        i = hits[0]
        before, after = [], []
        j = i - 1
        while j >= 0 and len(before) < span:
            off, txt = pool[j]
            if not RE_IDENT.match(txt) or txt in known_names or txt in ENUM_MACHINERY:
                break
            before.append(txt)
            j -= 1
        k = i + 1
        while k < len(pool) and len(after) < span:
            off, txt = pool[k]
            if not RE_IDENT.match(txt) or txt in known_names or txt in ENUM_MACHINERY:
                break
            after.append(txt)
            k += 1
        if not before and not after:
            continue
        out.append(OrderedDict([
            ("enum", name),
            ("pool_offset", pool[i][0]),
            ("adjacent_before_reversed", list(reversed(before))),
            ("adjacent_after", after),
            ("candidate_member_count", len(before) + len(after)),
        ]))
    return out


def now():
    return _dt.datetime.now(_dt.timezone.utc).isoformat(timespec="seconds")


def strip_for_scan(src):
    """Blanks out comments and string literals so brace counting is reliable."""
    out = list(src)
    i, n = 0, len(src)
    while i < n:
        c = src[i]
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            j = n if j < 0 else j
            for k in range(i, j):
                out[k] = " "
            i = j
        elif c == "/" and i + 1 < n and src[i + 1] == "*":
            j = src.find("*/", i + 2)
            j = n if j < 0 else j + 2
            for k in range(i, j):
                if src[k] != "\n":
                    out[k] = " "
            i = j
        elif c in "'\"`":
            q, j = c, i + 1
            while j < n:
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j] == q:
                    break
                j += 1
            j = min(j + 1, n)
            for k in range(i + 1, min(j, n)):
                if src[k] != "\n":
                    out[k] = " "
            i = j
        else:
            i += 1
    return "".join(out)


def match_brace(scan, start):
    """Returns the index just past the block whose '{' is at `start`."""
    depth = 0
    for i in range(start, len(scan)):
        if scan[i] == "{":
            depth += 1
        elif scan[i] == "}":
            depth -= 1
            if depth == 0:
                return i + 1
    return len(scan)


def split_params(argtxt):
    """Splits a parameter list, keeping defaults and rest/destructuring intact."""
    out, depth, cur = [], 0, ""
    for ch in argtxt:
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        if ch == "," and depth == 0:
            out.append(cur.strip())
            cur = ""
        else:
            cur += ch
    if cur.strip():
        out.append(cur.strip())
    params = []
    for p in out:
        if not p:
            continue
        name, default = p, None
        if "=" in p and not p.startswith("{"):
            name, default = p.split("=", 1)
            name, default = name.strip(), default.strip()
        params.append(OrderedDict([
            ("name", name),
            ("default", default),
            ("rest", name.startswith("...")),
            ("destructured", name.startswith(("{", "["))),
            ("optional", default is not None),
        ]))
    return params


def leading_doc(src, pos):
    """Returns the JSDoc/line comment block immediately above `pos`, if any."""
    head = src[:pos].rstrip()
    if head.endswith("*/"):
        s = head.rfind("/*")
        if s >= 0:
            return src[s:len(head)].strip()
    lines, out = head.split("\n"), []
    for ln in reversed(lines):
        t = ln.strip()
        if t.startswith("//"):
            out.append(t[2:].strip())
        elif t == "":
            continue
        else:
            break
    return "\n".join(reversed(out)) or None


def parse_members(src, scan, body_start, body_end):
    members = []
    i = body_start
    text = scan[body_start:body_end]
    for m in RE_MEMBER.finditer(text):
        abs_start = body_start + m.start()
        # only members at the class-body top level
        depth = scan.count("{", body_start, abs_start) - scan.count("}", body_start, abs_start)
        if depth != 1:
            continue
        mods = (m.group("mods") or "").split()
        name = m.group("name")
        args = m.group("args")
        open_brace = body_start + m.end() - 1
        end = match_brace(scan, open_brace)
        body = src[open_brace:end]
        api = sorted({"%s.%s" % (a, b) for a, b in RE_APICALL.findall(body)})
        news = sorted(set(RE_NEWCALL.findall(body)))
        kind = "method"
        if "get" in mods:
            kind = "getter"
        elif "set" in mods:
            kind = "setter"
        elif name == "constructor":
            kind = "constructor"
        members.append(OrderedDict([
            ("name", name),
            ("kind", kind),
            ("static", "static" in mods),
            ("async", "async" in mods),
            ("private", name.startswith("#")),
            ("parameters", split_params(args)),
            ("parameter_count", len(split_params(args))),
            ("native_api_calls", api),
            ("constructs", news),
            ("doc", leading_doc(src, abs_start)),
            ("body_lines", body.count("\n") + 1),
        ]))
    for m in re.finditer(r"^[ \t]{4}(static\s+)?(#?[A-Za-z_$][\w$]*)\s*(=[^;\n]*)?;",
                         text, re.M):
        members.append(OrderedDict([
            ("name", m.group(2)),
            ("kind", "field"),
            ("static", bool(m.group(1))),
            ("private", m.group(2).startswith("#")),
            ("initialiser", (m.group(3) or "").lstrip("=").strip() or None),
        ]))
    return members


def parse_file(path, rel):
    with open(path, encoding="utf-8", errors="replace") as fh:
        src = fh.read()
    scan = strip_for_scan(src)
    lic = None
    if src.lstrip().startswith("/*"):
        e = src.find("*/")
        lic = src[src.find("/*"):e + 2]
    requires = []
    for m in RE_REQ_DESTRUCT.finditer(src):
        syms = [s.strip() for s in m.group(1).split(",") if s.strip()]
        requires.append({"module": m.group(2), "symbols": syms,
                         "form": "destructured"})
    for m in RE_REQ_PLAIN.finditer(src):
        requires.append({"module": m.group(2), "symbols": [m.group(1)],
                         "form": "namespace"})
    classes = []
    for m in RE_CLASS.finditer(scan):
        name, base = m.group(1), m.group(2)
        body_start = m.end() - 1
        body_end = match_brace(scan, body_start)
        classes.append(OrderedDict([
            ("name", name),
            ("extends", base),
            ("doc", leading_doc(src, m.start())),
            ("members", parse_members(src, scan, body_start, body_end)),
        ]))
    funcs = []
    for m in RE_FUNC.finditer(scan):
        open_brace = m.end() - 1
        end = match_brace(scan, open_brace)
        body = src[open_brace:end]
        funcs.append(OrderedDict([
            ("name", m.group(1)),
            ("parameters", split_params(m.group(2))),
            ("native_api_calls",
             sorted({"%s.%s" % (a, b) for a, b in RE_APICALL.findall(body)})),
            ("doc", leading_doc(src, m.start())),
        ]))
    exports = []
    for m in RE_EXPORT.finditer(src):
        exports.append({"exported_as": m.group(1) or "<module>",
                        "value": m.group(2)})
    return OrderedDict([
        ("file", rel),
        ("bytes", len(src.encode("utf-8"))),
        ("licence_header_present", lic is not None),
        ("requires", requires),
        ("classes", classes),
        ("functions", funcs),
        ("exports", exports),
        ("class_count", len(classes)),
        ("member_count", sum(len(c["members"]) for c in classes)),
    ]), src, lic


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--jslib", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--native", default=None)
    a = ap.parse_args()

    modules, examples, tests = [], [], []
    licence = None
    licence_hashes = Counter()
    all_src = []
    for root, _dirs, files in os.walk(a.jslib):
        for f in sorted(files):
            p = os.path.join(root, f)
            rel = os.path.relpath(p, a.jslib).replace("\\", "/")
            if not f.endswith(".js"):
                continue
            rec, src, lic = parse_file(p, rel)
            all_src.append(src)
            if lic:
                licence_hashes[hashlib.sha256(
                    lic.encode("utf-8")).hexdigest()[:16]] += 1
                if licence is None:
                    licence = lic
            if rel.startswith("examples/"):
                examples.append(rec)
            elif rel.startswith("tests/"):
                tests.append(rec)
            else:
                modules.append(rec)

    corpus = "\n".join(all_src)

    # native module surface: union of symbols imported from affinity:*
    native = {}
    for rec in modules + examples + tests:
        for r in rec["requires"]:
            if not r["module"].startswith("affinity:"):
                continue
            d = native.setdefault(r["module"], {"symbols": set(), "importers": set()})
            d["symbols"].update(r["symbols"])
            d["importers"].add(rec["file"])
    native_out = []
    for mod in sorted(native):
        syms = sorted(native[mod]["symbols"])
        native_out.append(OrderedDict([
            ("module", mod),
            ("symbol_count", len(syms)),
            ("symbols", syms),
            ("imported_by", sorted(native[mod]["importers"])),
        ]))

    # native API method inventory, with the argument counts seen at call sites
    api_calls = {}
    for m in re.finditer(r"\b([A-Z]\w*Api)\.(\w+)\s*\(([^;]{0,400})", corpus):
        cls, meth, tail = m.group(1), m.group(2), m.group(3)
        depth, argtxt = 0, ""
        for ch in tail:
            if ch in "([{":
                depth += 1
            elif ch in ")]}":
                if depth == 0:
                    break
                depth -= 1
            argtxt += ch
        n = len(split_params(argtxt))
        d = api_calls.setdefault((cls, meth), Counter())
        d[n] += 1
    api_out = {}
    for (cls, meth), counts in sorted(api_calls.items()):
        api_out.setdefault(cls, []).append(OrderedDict([
            ("method", meth),
            ("call_sites", sum(counts.values())),
            ("argument_counts_observed", dict(sorted(counts.items()))),
        ]))
    native_api = [OrderedDict([("api_class", c),
                               ("method_count", len(v)),
                               ("methods", v)])
                  for c, v in sorted(api_out.items())]

    # enum member usage: Symbol.Member where Symbol is a native import
    native_symbols = set()
    for n_ in native_out:
        native_symbols.update(n_["symbols"])
    enum_use = {}
    for m in RE_ENUMUSE.finditer(corpus):
        sym, member = m.group(1), m.group(2)
        if sym not in native_symbols or sym.endswith("Api"):
            continue
        if member in ("prototype", "constructor", "call", "apply", "bind"):
            continue
        enum_use.setdefault(sym, Counter())[member] += 1
    enum_out = []
    for sym in sorted(enum_use):
        mem = enum_use[sym]
        enum_out.append(OrderedDict([
            ("symbol", sym),
            ("member_count", len(mem)),
            ("members", [{"name": k, "references": v}
                         for k, v in sorted(mem.items())]),
        ]))

    native_present = None
    enum_adjacency = None
    if a.native and os.path.exists(a.native):
        blob = open(a.native, "rb").read()
        checks = {}
        for n_ in native_out:
            checks[n_["module"]] = n_["module"].encode() in blob
        for row in native_api:
            checks[row["api_class"]] = row["api_class"].encode() in blob
        pool = native_string_pool(blob)
        index = {}
        for i, (_off, txt) in enumerate(pool):
            index.setdefault(txt, []).append(i)
        known = set()
        for row in native_api:
            known.add(row["api_class"])
            for meth in row["methods"]:
                known.add(meth["method"])
        for rec in modules:
            for c in rec["classes"]:
                known.add(c["name"])
        enum_syms = {sy for sy in native_symbols
                     if not sy.endswith("Api") and RE_IDENT.match(sy)}
        enum_adjacency = OrderedDict([
            ("method", "heuristic"),
            ("method_detail",
             "String-pool adjacency scan in libscriptingjs.dll. The native enums "
             "are compiled away, so their member names can only be located by "
             "position next to the enum's own name in the binary's string pool. "
             "Runs are cut at any string that is a known API class, a known API "
             "method, a known JS class, shared enum machinery (keys/values/"
             "entries/parse/...), or is not an identifier. This is positional "
             "evidence, NOT a parsed mapping: treat every entry as a candidate "
             "requiring confirmation before use."),
            ("enums_scanned", len(enum_syms)),
            ("string_pool_entries", len(pool)),
            ("candidates", enum_member_candidates(pool, index, enum_syms, known)),
        ])
        native_present = OrderedDict([
            ("binary", os.path.basename(a.native)),
            ("binary_bytes", len(blob)),
            ("method", "literal ASCII string presence test in the native binary"),
            ("names_checked", len(checks)),
            ("names_found", sum(1 for v in checks.values() if v)),
            ("detail", checks),
        ])

    doc = OrderedDict([
        ("schema_id", "handshake.affinity.scripting_api_detail.v1"),
        ("generated_at", now()),
        ("generator", TOOL_ID),
        ("method", "parsed"),
        ("method_detail",
         "Structural parse of every shipped JSLib .js source: comments and "
         "string literals are blanked before brace matching, then classes, "
         "their inheritance, and every member (constructor, method, getter, "
         "setter, static, private, field) are read with their real parameter "
         "names and defaults.  Each member also records which native "
         "affinity:* Api calls its body makes, which is the behavioural bridge "
         "between the scriptable surface and the native engine."),
        ("provenance_notes", [
            "JSDoc is almost absent in the shipped sources; the 'doc' field is "
            "populated only where a real comment exists. It is never invented.",
            "Native affinity:* modules are compiled into libscriptingjs.dll and "
            "ship no JS source. Their surface is therefore reported as the union "
            "of symbols the shipped JS actually imports, plus the Api methods the "
            "shipped JS actually calls - both parsed, both lower bounds on the "
            "true native surface.",
            "Enum members are recovered from real member accesses in the shipped "
            "JS (including examples and tests). This is a lower bound: an enum "
            "member never referenced by shipped JS cannot be recovered this way.",
        ]),
        ("licence", OrderedDict([
            ("spdx", "BSD-3-Clause"),
            ("copyright", "Copyright (c) 2026, Canva Pty Ltd."),
            ("files_carrying_header", sum(licence_hashes.values())),
            ("distinct_header_texts", len(licence_hashes)),
            ("header_text", licence),
            ("attribution_requirement",
             "Redistribution in source or binary form must retain the above "
             "copyright notice, this list of conditions and the disclaimer. "
             "The copyright holder's name and contributor names may not be used "
             "to endorse or promote derived products without prior written "
             "permission. Any Rust reimplementation that copies or adapts these "
             "sources must carry this notice; a clean-room reimplementation from "
             "the extracted behavioural specification does not, but the "
             "provenance is recorded here deliberately."),
        ])),
        ("counts", OrderedDict([
            ("library_modules", len(modules)),
            ("example_scripts", len(examples)),
            ("test_scripts", len(tests)),
            ("classes", sum(m["class_count"] for m in modules)),
            ("class_members", sum(m["member_count"] for m in modules)),
            ("native_modules", len(native_out)),
            ("native_symbols_imported", len(native_symbols)),
            ("native_api_classes", len(native_api)),
            ("native_api_methods", sum(r["method_count"] for r in native_api)),
            ("native_enums_with_recovered_members", len(enum_out)),
        ])),
        ("native_modules", native_out),
        ("native_api_methods", native_api),
        ("native_enum_members_recovered", enum_out),
        ("native_binary_corroboration", native_present),
        ("native_enum_members_adjacency_heuristic", enum_adjacency),
        ("modules", modules),
        ("examples", examples),
        ("tests", tests),
    ])
    with open(a.out, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)
    print("wrote", a.out)
    print(json.dumps(doc["counts"], indent=1))


if __name__ == "__main__":
    main()
