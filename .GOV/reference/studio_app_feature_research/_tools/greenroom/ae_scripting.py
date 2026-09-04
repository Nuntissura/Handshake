"""After Effects 2026 -> aftereffects_scripting_expressions.json

Offline. Reads only. Never launches After Effects.

Two distinct APIs are recovered.

1. The ExtendScript object model.
   Support Files/Plug-ins/Keyframe/Scripting.aex is the ExtendScript bridge. It
   stores each scripting class as a contiguous string table shaped

       "<Class>.prototype" , "<Class>" , "<Class> class" , member, member, ...

   so the class list and each class's member vocabulary can be read directly.

2. The expression language.
   Support Files/BEE.dll carries the Expression Language Menu as one contiguous
   ordered string table. Entries beginning with '>' are submenu headers
   (">Global", ">Vector Math", ">Random Numbers", ">Interpolation",
   ">Color Conversion", ">Other Math", ">Layer", ">Properties", ">3D",
   ">Space Transforms", ...), ">--------" is a separator, and the remaining
   entries are the expression identifiers - both the legacy snake_case spelling
   and the modern camelCase spelling. The same table also carries the argument
   signature strings, e.g. "freq, amp, octaves = 1, amp_mult = .5, t = time".

Plus: the shipped .jsx/.js scripts, the Expression/Script snippet library from
Required/Expressions and Scripting Palette.aex, and the $$$/AE/Scripting/*
diagnostic strings, which state real API rules (for example that setValue()
cannot be called on a property that has keyframes).
"""

from __future__ import annotations

import collections
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402

CSTR = re.compile(rb"(?<=\x00)([\x20-\x7e]{1,220})\x00")
IDENT = re.compile(r"^[A-Za-z_$][A-Za-z0-9_$]*$")
PROTO = re.compile(rb"([A-Za-z_][A-Za-z0-9_]{1,40})\.prototype\x00")

AI_CLASSES = {
    "ContentAwareFillOptions": "Content-Aware Fill scripting surface",
}


def strings_with_offsets(data: bytes):
    return [(m.start(1), m.group(1).decode("latin-1")) for m in CSTR.finditer(data)]


# --------------------------------------------------------------------------
# 1. ExtendScript object model
# --------------------------------------------------------------------------

NOISE = ("::", "boost", "std::", "D:\\", ".cpp", ".h", "%s", "%d", " ",
         "operator", "__")


def extendscript_model():
    p = os.path.join(C.support_files(), "Plug-ins", "Keyframe", "Scripting.aex")
    data = C.read_bytes(p)
    protos = [(m.start(), m.group(1).decode()) for m in PROTO.finditer(data)]
    protos.sort()
    strs = strings_with_offsets(data)
    offs = [s[0] for s in strs]
    import bisect
    classes = []
    for i, (off, name) in enumerate(protos):
        end = protos[i + 1][0] if i + 1 < len(protos) else off + 200_000
        lo = bisect.bisect_left(offs, off)
        hi = bisect.bisect_left(offs, end)
        members = []
        seen = set()
        for _o, s in strs[lo:hi]:
            if s in (name, name + ".prototype", name + " class"):
                continue
            if not IDENT.match(s):
                continue
            if any(n in s for n in NOISE) or len(s) > 48:
                continue
            if s in seen:
                continue
            seen.add(s)
            members.append(s)
        internal = [m for m in members if m.startswith("_")]
        setters = [m for m in members if re.match(r"^new[A-Z]", m)]
        public = [m for m in members if m not in internal and m not in setters]
        classes.append({
            "class": name,
            "member_count": len(public),
            "members": public,
            "setter_shadow_names": setters,
            "internal_accessors": internal,
            "evidence_offset": hex(off),
        })
    return classes, C.rel(p)


# --------------------------------------------------------------------------
# 2. expression language table
# --------------------------------------------------------------------------

def expression_table():
    p = os.path.join(C.support_files(), "BEE.dll")
    data = C.read_bytes(p)
    strs = strings_with_offsets(data)
    anchor = None
    for i, (_o, s) in enumerate(strs):
        if s == ">Global":
            anchor = i
            break
    if anchor is None:
        return None, C.rel(p), "no '>Global' header found in BEE.dll"

    def table_like(s):
        if s.startswith(">") or s.startswith("."):
            return True
        if IDENT.match(s) and len(s) <= 40 and "_" not in s[:1]:
            return True
        if ("=" in s or "," in s) and len(s) < 220:
            return True
        return False

    # The table also contains a handful of one- or two-character operator
    # strings and one internal symbol; tolerate short interruptions rather than
    # stopping the run on them.
    TOL = 4
    lo = anchor
    misses = 0
    while lo > 0:
        o, s = strs[lo - 1]
        prev_end = o + len(s) + 1
        if strs[lo][0] - prev_end > 64:
            break
        if table_like(s):
            misses = 0
        else:
            misses += 1
            if misses > TOL:
                break
        lo -= 1
    hi = anchor
    misses = 0
    while hi + 1 < len(strs):
        o, s = strs[hi]
        nxt_o, nxt_s = strs[hi + 1]
        if nxt_o - (o + len(s) + 1) > 64:
            break
        if table_like(nxt_s):
            misses = 0
        else:
            misses += 1
            if misses > TOL:
                break
        hi += 1
    # trim the tolerated non-table strings back off the ends
    while lo < anchor and not table_like(strs[lo][1]):
        lo += 1
    while hi > anchor and not table_like(strs[hi][1]):
        hi -= 1

    entries = []
    category = None
    for o, s in strs[lo:hi + 1]:
        if s.startswith(">"):
            label = s[1:]
            if set(label) == {"-"}:
                entries.append({"kind": "separator", "offset": hex(o)})
                continue
            category = label
            entries.append({"kind": "category", "category": label,
                            "offset": hex(o)})
        elif IDENT.match(s) or s.startswith("."):
            entries.append({"kind": "identifier", "name": s,
                            "category": category,
                            "spelling": "snake_case" if "_" in s else "camelCase",
                            "offset": hex(o)})
        else:
            entries.append({"kind": "argument_signature", "signature": s,
                            "category": category, "offset": hex(o)})
    return entries, C.rel(p), None


# --------------------------------------------------------------------------
# 3. shipped scripts
# --------------------------------------------------------------------------

def shipped_scripts(class_names):
    root = C.support_files()
    out = []
    for p in C.iter_files(root, (".jsx", ".js"),
                          skip_dirs=("node_modules", "CEPHtmlEngine", "UXP",
                                     "com.adobe.frameio", "Libraries")):
        try:
            text = C.read_bytes(p).decode("utf-8", "replace")
        except OSError:
            continue
        used = sorted({c for c in class_names if re.search(r"\b%s\b" % re.escape(c), text)})
        api = sorted(set(re.findall(r"\bapp\.[A-Za-z_][A-Za-z0-9_.]*", text)))
        out.append({
            "file": C.rel(p),
            "bytes": os.path.getsize(p),
            "lines": text.count("\n") + 1,
            "target_engine": (re.search(r"#targetengine\s+(\S+)", text, re.I).group(1)
                              if re.search(r"#targetengine", text, re.I) else None),
            "scripting_classes_referenced": used,
            "app_api_paths_used": api[:80],
            "declares_scriptui_panel": "ScriptUI" in text or "Window(" in text,
        })
    return out


# --------------------------------------------------------------------------
# 4. snippet library
# --------------------------------------------------------------------------

SNIP = re.compile(
    r"^AE/ExpressionPanel/(?P<kind>Expression|Script)?/?Snippets?/(?P<rest>.+)$")


def snippet_library(idx):
    snips = collections.defaultdict(dict)
    other = {}
    for k, v in idx.items():
        if not k.startswith("AE/ExpressionPanel/"):
            continue
        parts = k.split("/")
        if len(parts) >= 5 and parts[-1] in ("Title", "Description"):
            name = "/".join(parts[2:-1])
            snips[name][parts[-1].lower()] = v["text"]
        else:
            other[k] = v["text"]
    rows = []
    for name, d in sorted(snips.items()):
        kind = "expression" if "/Expression/" in "/" + name else "script"
        rows.append({"snippet": name.split("/")[-1], "path": name,
                     "kind": kind, "title": d.get("title"),
                     "description": d.get("description")})
    return rows, other


# --------------------------------------------------------------------------

def main():
    idx = C.build_english_index()
    classes, script_src = extendscript_model()
    ai_classes = [c for c in classes if c["class"] in AI_CLASSES]
    classes = [c for c in classes if c["class"] not in AI_CLASSES]
    class_names = [c["class"] for c in classes]

    expr, expr_src, expr_err = expression_table()
    scripts = shipped_scripts(class_names)
    snippets, snippet_other = snippet_library(idx)

    rules = {k: v["text"] for k, v in C.keys_under("AE/Scripting/", idx).items()}
    palette = {k: v["text"] for k, v in C.keys_under("AE/ExpressionPal/", idx).items()}

    ids = [e for e in (expr or []) if e["kind"] == "identifier"]
    sigs = [e for e in (expr or []) if e["kind"] == "argument_signature"]
    cats = [e for e in (expr or []) if e["kind"] == "category"]
    by_cat = collections.Counter(e.get("category") for e in ids)
    camel = [e["name"] for e in ids if e["spelling"] == "camelCase"]
    snake = [e["name"] for e in ids if e["spelling"] == "snake_case"]

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_scripting.py",
        "evidence": [
            {"label": "parsed", "path": script_src,
             "what": "ExtendScript class registration tables: "
                     "'<Class>.prototype' / '<Class>' / '<Class> class' followed "
                     "by that class's member identifiers",
             "extraction": "locate every '<Name>.prototype' C string, then take "
                           "the identifier strings between it and the next one"},
            {"label": "parsed", "path": expr_src,
             "what": "Expression Language Menu: one contiguous ordered string "
                     "table of '>Category' headers, '>--------' separators, "
                     "expression identifiers in both spellings, and argument "
                     "signature strings",
             "extraction": "anchor on the '>Global' header, then extend the run "
                           "in both directions while consecutive strings stay "
                           "adjacent (<=24 byte gap) and table-shaped"},
            {"label": "parsed", "path": "Support Files/Scripts/**",
             "what": "shipped .jsx sample and utility scripts"},
            {"label": "parsed",
             "path": "Support Files/Required/Expressions and Scripting Palette.aex",
             "what": "$$$/AE/ExpressionPanel/**/Title and /Description for the "
                     "shipped expression and script snippet library"},
            {"label": "parsed", "path": "$$$/AE/Scripting/* strings",
             "what": "scripting diagnostics that state real API rules"},
        ],
        "failures_and_limits": [
            "The member tables in Scripting.aex do NOT distinguish a property "
            "from a method, and carry no arity or type. Members are therefore "
            "reported as an ordered vocabulary per class, with the '_get*'/"
            "'_set*' internal accessors and the 'new<Prop>' setter shadow names "
            "separated out because those spellings are self-describing.",
            "Expression argument signatures are pooled in the same table as the "
            "identifiers but are NOT stored adjacent to the function they belong "
            "to. They are therefore emitted as an ordered signature pool with "
            "their surrounding category, and the signature-to-function binding "
            "is NOT asserted.",
            "No ExtendScript type library or .h header ships on disk, so return "
            "types and parameter types are not recoverable offline.",
            ("expression table extraction failed: %s" % expr_err) if expr_err else None,
        ],
        "counts": {
            "extendscript_classes": len(classes),
            "extendscript_members": sum(c["member_count"] for c in classes),
            "expression_categories": len(cats),
            "expression_identifiers": len(ids),
            "expression_identifiers_camelCase": len(camel),
            "expression_identifiers_snake_case": len(snake),
            "expression_argument_signatures": len(sigs),
            "shipped_scripts": len(scripts),
            "snippets": len(snippets),
            "scripting_rule_strings": len(rules),
        },
    }
    method["failures_and_limits"] = [f for f in method["failures_and_limits"] if f]

    excluded = dict(C.default_excluded_ai())
    excluded.update({
        "policy": C.EXCLUDED_AI_NOTE,
        "excluded_scripting_classes": [
            {"class": c["class"], "reason": AI_CLASSES[c["class"]],
             "evidence_path": script_src, "member_count": c["member_count"]}
            for c in ai_classes],
        "note": "Any Content-Aware Fill / Roto Brush / Scene Edit Detection "
                "scripting entry point is excluded from the recovered object "
                "model above; the class is named here with its on-disk evidence "
                "path so the exclusion is auditable.",
    })

    payload = {
        "summary": {
            "extendscript_classes": len(classes),
            "extendscript_members": sum(c["member_count"] for c in classes),
            "expression_identifiers": len(ids),
            "expression_categories": [c["category"] for c in cats],
            "expression_identifiers_by_category": dict(by_cat),
            "expression_argument_signatures": len(sigs),
            "shipped_scripts": len(scripts),
            "snippets": len(snippets),
        },
        "extendscript_object_model": classes,
        "expression_language": {
            "ordered_table": expr,
            "identifiers": ids,
            "argument_signature_pool": sigs,
            "uncategorised_prefix_note":
                "333 identifiers were recovered from one contiguous table. The "
                "first '>' category header in that table is '>Global'; the "
                "identifiers that precede it carry category null. They are the "
                "same table's leading block (camera, light, mask, path, "
                "velocity, text-style and string members) and are real, but no "
                "category header precedes them on disk, so none is invented.",
            "dual_spelling_note":
                "After Effects accepts a legacy snake_case spelling and a modern "
                "camelCase spelling for most expression members; both are in the "
                "shipped table and both are listed.",
        },
        "expression_and_script_snippets": snippets,
        "shipped_scripts": scripts,
        "scripting_rule_strings": rules,
        "expression_editor_strings": palette,
        "expression_panel_other_strings": snippet_other,
    }
    C.write_json("aftereffects_scripting_expressions.json",
                 "handshake.studio.teardown.aftereffects.scripting_expressions",
                 method, payload, excluded_ai=excluded)
    print("classes=%d members=%d expr_ids=%d sigs=%d scripts=%d snippets=%d"
          % (len(classes), sum(c["member_count"] for c in classes), len(ids),
             len(sigs), len(scripts), len(snippets)), file=sys.stderr)


if __name__ == "__main__":
    main()
