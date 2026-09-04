#!/usr/bin/env python
"""illustrator-library-parse.py

Recover the ENTRIES inside Adobe Illustrator's shipped preset libraries.

A library is a *file*; the thing a Rust reimplementation must reproduce is the
set of entries inside it.  This tool never reports a file count as an entry
count.

Four container formats are parsed:

  .ai   PDF container -> /Private -> AIPrivateDataN blocks -> zstd
        -> legacy PostScript-flavoured AI stream.  Entries are recovered from
        the AI stream's own section markers (see GRAMMAR below).
  .ase  Adobe Swatch Exchange -- documented binary, parsed in full
        (group structure, colour model, components, colour type).
  .acb  Adobe Color Book -- documented binary, parsed in full
        (book id, page size, colour space, per-colour code + components,
        and the trailing spfl spot/process ink-type record).
  .acbl Adobe Swatchbook -- the XML legacy colour books under
        Swatches/Color Books/Legacy, parsed in full.

GRAMMAR (all PARSED from Adobe's own markers, no guessing):
  symbols         %AI24_BeginSymbolList  ..  %AI24_EndSymbolList   one (Name)/line
  graphic_styles  %AI9_BeginArtStyleList ..  %AI9_EndArtStyleList  one (Name)/line
  swatches        %AI5_BeginPalette .. %AI5_EndPalette; a (Name) directly
                  followed by the `Pc` operator closes one palette entry.  The
                  operator that precedes it gives the swatch kind
                  (Xa process, Xs/Xx spot, Xz registration, BB gradient).
  gradients       %AI5_BeginGradient: (Name)
  patterns        %AI3_BeginPattern: (Name)
  brushes         %AI8_BeginPluginObject / (ToolType) / (Name) / (params)
                  plus the (Adobe Brush Manager Order) roster string.
  svg_filters     %AI10_BeginSVGFilter .. %AI10_EndSVGFilter

Reads files only.  Never launches Illustrator.
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import struct
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ai_private  # noqa: E402

PRESET_ROOT_DEFAULT = r"C:\Program Files\Adobe\Adobe Illustrator 2026\Presets\en_US"

# ---------------------------------------------------------------------------
# PostScript string reader
# ---------------------------------------------------------------------------
_OCTAL = re.compile(rb"\\([0-7]{1,3})")


def ps_unescape(raw: bytes) -> str:
    """Resolve PostScript string escapes, then decode (utf-8 first)."""
    out = bytearray()
    i = 0
    n = len(raw)
    while i < n:
        c = raw[i]
        if c == 0x5C and i + 1 < n:  # backslash
            nxt = raw[i + 1]
            if 0x30 <= nxt <= 0x37:
                j = i + 1
                digits = b""
                while j < n and 0x30 <= raw[j] <= 0x37 and len(digits) < 3:
                    digits += raw[j:j + 1]
                    j += 1
                out.append(int(digits, 8) & 0xFF)
                i = j
                continue
            mapping = {0x6E: 0x0A, 0x72: 0x0D, 0x74: 0x09, 0x62: 0x08,
                       0x66: 0x0C, 0x28: 0x28, 0x29: 0x29, 0x5C: 0x5C}
            if nxt in mapping:
                out.append(mapping[nxt])
                i += 2
                continue
            if nxt in (0x0A, 0x0D):  # line continuation
                i += 2
                continue
            out.append(nxt)
            i += 2
            continue
        out.append(c)
        i += 1
    b = bytes(out)
    for enc in ("utf-8", "cp1252", "latin-1"):
        try:
            return b.decode(enc)
        except UnicodeDecodeError:
            continue
    return b.decode("latin-1", "replace")


_RE_PSSTR = re.compile(rb"\((?:[^()\\]|\\.)*\)", re.S)


def read_ps_string(data: bytes, pos: int) -> tuple[str, int] | None:
    """Read a balanced PostScript ( ... ) string starting at data[pos]=='('."""
    if pos >= len(data) or data[pos:pos + 1] != b"(":
        return None
    depth = 0
    i = pos
    n = len(data)
    start = pos + 1
    while i < n:
        c = data[i]
        if c == 0x5C:
            i += 2
            continue
        if c == 0x28:
            depth += 1
        elif c == 0x29:
            depth -= 1
            if depth == 0:
                return ps_unescape(data[start:i]), i + 1
        i += 1
    return None


def strings_in(block: bytes) -> list[str]:
    """All top-level ( ... ) strings inside a block, in order."""
    out = []
    i = 0
    n = len(block)
    while i < n:
        if block[i:i + 1] == b"(":
            r = read_ps_string(block, i)
            if r:
                out.append(r[0])
                i = r[1]
                continue
        i += 1
    return out


def join_continued(parts: list[str]) -> str:
    """AI splits a long string over several ( .. ) - continued fragments."""
    return "".join(parts)


# ---------------------------------------------------------------------------
# AI private-stream entry extraction
# ---------------------------------------------------------------------------
BRUSH_TOOL_KINDS = {
    "Adobe Calligraphic Brush Tool": "calligraphic",
    "Adobe Scatter Brush Tool": "scatter",
    "Adobe ArtOnPath Brush Tool": "art",
    "Adobe PatternOnPath Brush Tool": "pattern",
    # "dBrush" is Illustrator's internal name for the bristle brush; it is the
    # tool string used throughout Bristle Brush Library.ai.
    "Adobe dBrush Brush Tool": "bristle",
    "Adobe Bristle Brush Tool": "bristle",
    "Adobe Image Brush Tool": "image",
}
BRUSH_ROSTER_KEY = "Adobe Brush Manager Order"

# AI palette (swatch list) operators, mapped to the swatch kind they close.
# The verbatim operator is also preserved per entry as `palette_operator`.
PALETTE_OPS = {
    b"Xa": "process", b"XA": "process",   # AI process colour
    b"k": "process_cmyk", b"K": "process_cmyk",
    b"g": "process_gray", b"G": "process_gray",
    b"Xs": "spot", b"Xx": "spot", b"x": "spot", b"X": "spot",
    b"Xz": "registration",
    b"BB": "gradient",
    b"p": "pattern",
}


def _list_block(payload: bytes, begin: bytes, end: bytes) -> list[str]:
    names = []
    for m in re.finditer(re.escape(begin), payload):
        e = payload.find(end, m.end())
        if e < 0:
            continue
        names.extend(strings_in(payload[m.end():e]))
    return names


def _colon_named(payload: bytes, marker: bytes) -> list[str]:
    """`%AI5_BeginGradient: (Name)` style markers."""
    out = []
    for m in re.finditer(re.escape(marker) + rb"\s*:?\s*", payload):
        r = read_ps_string(payload, m.end())
        if r:
            out.append(r[0])
    return out


_RE_PC = re.compile(rb"\)\s*Pc\b")


def _palette_entries(payload: bytes) -> list[dict]:
    """Swatch entries: a ( Name ) immediately followed by the Pc operator."""
    a = payload.find(b"%AI5_BeginPalette")
    if a < 0:
        return []
    b = payload.find(b"%AI5_EndPalette", a)
    block = payload[a:b if b > 0 else len(payload)]
    out = []
    for m in _RE_PC.finditer(block):
        # walk back to the opening paren of this string
        close = m.start()
        i = close
        depth = 0
        while i >= 0:
            ch = block[i]
            if ch == 0x29 and (i == 0 or block[i - 1] != 0x5C):
                depth += 1
            elif ch == 0x28 and (i == 0 or block[i - 1] != 0x5C):
                depth -= 1
                if depth == 0:
                    break
            i -= 1
        if i < 0:
            continue
        name = ps_unescape(block[i + 1:close])
        # The swatch kind is the LAST AI palette operator emitted before the
        # entry's closing name.  Observed forms:
        #   <cmyk> <rgb> Xa \n (Name) \n Pc                 process colour
        #   <...> ([Registration]) 0 1 Xz ([Registration]) Pc   registration
        #   Bb 2 (Grad) ... Bg 0 BB \n (Name) \n Pc          gradient
        #   (Pat) 0 0 1 1 0 0 0 0 0 [matrix] p \n (Name) \n Pc  pattern
        pre = block[max(0, i - 200):i]
        kind = "unknown"
        op_tok = None
        for tok in re.findall(rb"(?<![A-Za-z])([A-Za-z]{1,3})(?![A-Za-z])", pre):
            if tok in PALETTE_OPS:
                op_tok = tok
        if op_tok:
            kind = PALETTE_OPS[op_tok]
        out.append({"name": name, "swatch_kind": kind,
                    "palette_operator": op_tok.decode() if op_tok else None})
    return out


def _plugin_objects(payload: bytes) -> list[dict]:
    """(ToolType) (Name) (params) triples from %AI8_BeginPluginObject blocks."""
    out = []
    for m in re.finditer(rb"%AI8_BeginPluginObject", payload):
        e = payload.find(b"%AI8_EndPluginObject", m.end())
        seg = payload[m.end():e if e > 0 else min(len(payload), m.end() + 20000)]
        strs = strings_in(seg)
        if not strs:
            continue
        tool = strs[0]
        name = strs[1] if len(strs) > 1 else None
        params = join_continued(strs[2:]) if len(strs) > 2 else ""
        out.append({"tool": tool, "name": name, "params": params,
                    "truncated": e < 0})
    return out


def parse_ai_library(path: str) -> dict:
    res = ai_private.extract(path)
    rec = {
        "container": "ai_pdf",
        "private_stream": {
            "compression": res.compression,
            "num_block": res.num_block,
            "creator_version": res.creator_version,
            "container_version": res.container_version,
            "decompressed_bytes": len(res.payload),
        },
        "parse_error": res.error,
        "entries": {},
        "entry_total": 0,
    }
    if res.error or not res.payload:
        return rec
    p = res.payload

    symbols = _list_block(p, b"%AI24_BeginSymbolList", b"%AI24_EndSymbolList")
    styles = _list_block(p, b"%AI9_BeginArtStyleList", b"%AI9_EndArtStyleList")
    gradients = _colon_named(p, b"%AI5_BeginGradient")
    patterns = _colon_named(p, b"%AI3_BeginPattern")
    swatches = _palette_entries(p)
    plugins = _plugin_objects(p)

    brushes, roster, other_plugins = [], None, []
    for po in plugins:
        if po["tool"] == BRUSH_ROSTER_KEY:
            roster = po
            continue
        if po["tool"] in BRUSH_TOOL_KINDS or "Brush Tool" in (po["tool"] or ""):
            brushes.append({
                "name": po["name"],
                "brush_kind": BRUSH_TOOL_KINDS.get(po["tool"], "unknown"),
                "tool": po["tool"],
                "params": po["params"],
            })
        else:
            other_plugins.append(po)

    roster_entries = []
    if roster:
        blob = (roster.get("name") or "") + roster.get("params", "")
        # "/ <tool>/ <name>/ <tool>/ <name>/ ..."
        toks = [t.strip() for t in blob.split("/") if t.strip()]
        for i in range(0, len(toks) - 1, 2):
            roster_entries.append({"tool": toks[i], "name": toks[i + 1]})

    svg_filters = []
    for m in re.finditer(rb"%AI10_BeginSVGFilter", p):
        e = p.find(b"%AI10_EndSVGFilter", m.end())
        seg = p[m.end():e if e > 0 else m.end() + 4000]
        ss = strings_in(seg)
        if ss:
            svg_filters.append(ss[0])

    def put(key, items, kind):
        if items:
            rec["entries"][key] = {
                "count": len(items),
                "unique_count": len({(i["name"] if isinstance(i, dict) else i)
                                     for i in items}),
                "extraction": kind,
                "items": items,
            }

    put("symbols", symbols, "parsed:%AI24_BeginSymbolList")
    put("graphic_styles", styles, "parsed:%AI9_BeginArtStyleList")
    put("gradients", gradients, "parsed:%AI5_BeginGradient")
    put("patterns", patterns, "parsed:%AI3_BeginPattern")
    put("swatches", swatches, "parsed:%AI5_BeginPalette/Pc")
    put("brushes", brushes, "parsed:%AI8_BeginPluginObject")
    put("brush_manager_order", roster_entries,
        "parsed:(Adobe Brush Manager Order)")
    put("svg_filters", svg_filters, "parsed:%AI10_BeginSVGFilter")
    if other_plugins:
        rec["entries"]["other_plugin_objects"] = {
            "count": len(other_plugins),
            "unique_count": len({p_["tool"] for p_ in other_plugins}),
            "extraction": "parsed:%AI8_BeginPluginObject (non-brush)",
            "items": other_plugins,
        }

    # brush_manager_order is a roster over the same brushes -> not additive
    rec["entry_total"] = sum(v["count"] for k, v in rec["entries"].items()
                             if k != "brush_manager_order")
    return rec


# ---------------------------------------------------------------------------
# .ase -- Adobe Swatch Exchange
# ---------------------------------------------------------------------------
ASE_MODEL_COMPONENTS = {"RGB ": 3, "CMYK": 4, "LAB ": 3, "GRAY": 1}
ASE_COLOR_TYPE = {0: "global", 1: "spot", 2: "normal"}


def parse_ase(path: str) -> dict:
    d = open(path, "rb").read()
    rec = {"container": "ase", "parse_error": None, "entries": {},
           "entry_total": 0}
    if d[:4] != b"ASEF":
        rec["parse_error"] = "bad_magic"
        return rec
    major, minor, nblocks = struct.unpack_from(">HHI", d, 4)
    rec["format"] = {"version": f"{major}.{minor}", "declared_blocks": nblocks}
    o = 12
    colors, groups, stack = [], [], []
    read_blocks = 0
    try:
        while o < len(d) and read_blocks < nblocks:
            btype, blen = struct.unpack_from(">HI", d, o)
            body = d[o + 6:o + 6 + blen]
            o += 6 + blen
            read_blocks += 1
            if btype in (0x0001, 0xC001):
                nlen = struct.unpack_from(">H", body, 0)[0]
                name = body[2:2 + nlen * 2].decode("utf-16-be").rstrip("\x00")
                q = 2 + nlen * 2
                if btype == 0xC001:
                    groups.append(name)
                    stack.append(name)
                    continue
                model = body[q:q + 4].decode("latin-1")
                q += 4
                ncomp = ASE_MODEL_COMPONENTS.get(model.upper(), 0)
                comps = list(struct.unpack_from(">" + "f" * ncomp, body, q)) \
                    if ncomp else []
                q += 4 * ncomp
                ctype = struct.unpack_from(">H", body, q)[0] if q + 2 <= len(body) else None
                colors.append({
                    "name": name,
                    "color_model": model.strip(),
                    "components": [round(c, 6) for c in comps],
                    "color_type": ASE_COLOR_TYPE.get(ctype, ctype),
                    "group": stack[-1] if stack else None,
                })
            elif btype == 0xC002:
                if stack:
                    stack.pop()
            else:
                rec.setdefault("unknown_block_types", []).append(hex(btype))
    except Exception as exc:
        rec["parse_error"] = f"{type(exc).__name__}:{exc}"
    rec["entries"]["swatches"] = {
        "count": len(colors), "unique_count": len({c["name"] for c in colors}),
        "extraction": "parsed:ASEF_binary", "items": colors,
    }
    if groups:
        rec["entries"]["swatch_groups"] = {
            "count": len(groups), "unique_count": len(set(groups)),
            "extraction": "parsed:ASEF_binary_group_blocks", "items": groups,
        }
    rec["entry_total"] = len(colors)
    rec["blocks_read"] = read_blocks
    return rec


# ---------------------------------------------------------------------------
# .acb -- Adobe Color Book
# ---------------------------------------------------------------------------
ACB_SPACE = {0: "RGB", 1: "HSB", 2: "CMYK", 7: "Lab", 8: "Grayscale"}
ACB_SPACE_COMPONENTS = {0: 3, 1: 3, 2: 4, 7: 3, 8: 1}


def _acb_str(d: bytes, o: int) -> tuple[str, int]:
    n = struct.unpack_from(">I", d, o)[0]
    s = d[o + 4:o + 4 + n * 2].decode("utf-16-be", "replace")
    return s, o + 4 + n * 2


def _zstring_value(s: str) -> str:
    """Adobe ZString `$$$/colorbook/X/title=TRUMATCH` -> `TRUMATCH`."""
    return s.split("=", 1)[1] if s.startswith("$$$/") and "=" in s else s


def parse_acb(path: str) -> dict:
    d = open(path, "rb").read()
    rec = {"container": "acb", "parse_error": None, "entries": {},
           "entry_total": 0}
    if d[:4] != b"8BCB":
        rec["parse_error"] = "bad_magic"
        return rec
    version, book_id = struct.unpack_from(">HH", d, 4)
    o = 8
    try:
        title, o = _acb_str(d, o)
        prefix, o = _acb_str(d, o)
        suffix, o = _acb_str(d, o)
        desc, o = _acb_str(d, o)
        count, page_size, page_sel, space = struct.unpack_from(">HHHH", d, o)
        o += 8
        ncomp = ACB_SPACE_COMPONENTS.get(space)
        rec["format"] = {
            "version": version, "book_id": book_id,
            "title": _zstring_value(title),
            "title_zstring": title,
            "color_name_prefix": _zstring_value(prefix),
            "color_name_suffix": _zstring_value(suffix),
            "description": _zstring_value(desc),
            "declared_color_count": count,
            "page_size": page_size,
            "page_selector_offset": page_sel,
            "color_space_id": space,
            "color_space": ACB_SPACE.get(space, f"unknown({space})"),
            "components_per_color": ncomp,
        }
        if ncomp is None:
            rec["parse_error"] = f"unknown_color_space_{space}"
            return rec
        colors = []
        for _ in range(count):
            name, o = _acb_str(d, o)
            code = d[o:o + 6].decode("latin-1").strip()
            o += 6
            comps = list(d[o:o + ncomp])
            o += ncomp
            colors.append({
                "name": _zstring_value(name).strip(),
                "code": code,
                "components_raw": comps,
            })
        rec["entries"]["colors"] = {
            "count": len(colors), "unique_count": len({c["code"] for c in colors}),
            "extraction": "parsed:8BCB_binary", "items": colors,
        }
        rec["entry_total"] = len(colors)
        # Every shipped .acb ends with an 8-byte record: 'spfl' + 'spot'|'proc'
        # declaring whether the book's colours are spot or process inks.
        tail = d[o:o + 8]
        rec["trailing_bytes"] = tail.decode("latin-1", "replace") if tail else ""
        if tail[:4] == b"spfl":
            ink = tail[4:8].decode("latin-1")
            rec["format"]["ink_type"] = {"spot": "spot", "proc": "process"}.get(
                ink, ink)
            rec["format"]["ink_type_source"] = "parsed:spfl trailer record"
        rec["bytes_consumed"] = o
        rec["file_size"] = len(d)
        rec["fully_consumed"] = (o + len(tail) == len(d))
    except Exception as exc:
        rec["parse_error"] = f"{type(exc).__name__}:{exc}"
    return rec


# ---------------------------------------------------------------------------
# .acbl -- Adobe Swatchbook (XML legacy colour book)
# ---------------------------------------------------------------------------
def parse_acbl(path: str) -> dict:
    import xml.etree.ElementTree as ET
    rec = {"container": "acbl", "parse_error": None, "entries": {},
           "entry_total": 0}
    try:
        root = ET.parse(path).getroot()
    except Exception as exc:
        rec["parse_error"] = f"{type(exc).__name__}:{exc}"
        return rec
    fmt = root.find("Formats/Format")
    pp = root.find("PrefixPostfixPairs/PrefixPostfixPair")
    rec["format"] = {
        "version": root.get("Version"),
        "book_id": root.get("BookID"),
        "color_space": fmt.get("ColorSpace") if fmt is not None else None,
        "encoding": fmt.get("Encoding") if fmt is not None else None,
        "channels": int(fmt.get("Channels")) if fmt is not None and
        fmt.get("Channels") else None,
        "color_name_prefix": pp.get("Prefix") if pp is not None else None,
        "color_name_postfix": pp.get("Postfix") if pp is not None else None,
    }
    colors = []
    for sp in root.findall("Swatches/Sp"):
        c = sp.find("C")
        alt = sp.find("A")
        comps = []
        if c is not None and c.text:
            for tok in c.text.split():
                try:
                    comps.append(int(tok))
                except ValueError:
                    try:
                        comps.append(float(tok))
                    except ValueError:
                        pass
        colors.append({
            "name": sp.get("N"),
            "code": (alt.get("N") if alt is not None else None) or sp.get("N"),
            "components_raw": comps,
        })
    rec["entries"]["colors"] = {
        "count": len(colors), "unique_count": len({c["name"] for c in colors}),
        "extraction": "parsed:AdobeSwatchbook_xml", "items": colors,
    }
    rec["entry_total"] = len(colors)
    return rec


# ---------------------------------------------------------------------------
def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--root", default=PRESET_ROOT_DEFAULT)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    targets = []
    for dp, _dn, fn in os.walk(args.root):
        for f in fn:
            if f.lower().endswith((".ai", ".ase", ".acb", ".acbl")):
                targets.append(os.path.join(dp, f))
    targets.sort()

    # Which entry kind a library of this family actually publishes to its
    # panel.  Everything else a file contains (the gradients a symbol's
    # artwork happens to use, the [Default] style every AI document carries)
    # is present in the file but is NOT an entry the library offers.
    PRIMARY_BY_FAMILY = {
        "Brushes": "brushes",
        "Symbols": "symbols",
        "Graphic Styles": "graphic_styles",
        "Swatches": None,  # decided per-file below
    }
    SWATCH_SUBFAMILY = {
        "Gradients": "gradients",
        "Patterns": "patterns",
        "Color Books": "colors",
    }
    BUILTIN_NAMES = {"[Default]", "[Registration]", "[None]"}

    libs = []
    for p in targets:
        rel = os.path.relpath(p, args.root).replace("\\", "/")
        ext = os.path.splitext(p)[1].lower()
        parts = rel.split("/")
        family = parts[0]
        if ext == ".ai":
            rec = parse_ai_library(p)
        elif ext == ".ase":
            rec = parse_ase(p)
        elif ext == ".acbl":
            rec = parse_acbl(p)
        else:
            rec = parse_acb(p)
        rec["path"] = rel
        rec["family"] = family
        rec["file_bytes"] = os.path.getsize(p)

        primary = PRIMARY_BY_FAMILY.get(family)
        if primary is None:
            primary = "swatches"
            for seg in parts[1:]:
                if seg in SWATCH_SUBFAMILY:
                    primary = SWATCH_SUBFAMILY[seg]
                    break
            if ext in (".acb", ".acbl"):
                primary = "colors"
            elif ext == ".ase":
                primary = "swatches"
        rec["primary_entry_kind"] = primary
        rec["primary_entry_kind_basis"] = (
            "DERIVED from the library's family folder and file extension; the "
            "entry names/counts themselves are parsed")

        block = rec["entries"].get(primary)
        items = block["items"] if block else []
        names = [(i["name"] if isinstance(i, dict) else i) for i in items]
        builtin = [n for n in names if n in BUILTIN_NAMES]
        rec["primary_entry_count"] = len(names)
        rec["primary_entry_count_excluding_builtins"] = len(names) - len(builtin)
        rec["builtin_entries_present"] = sorted(set(builtin))
        rec["incidental_entry_kinds"] = sorted(
            k for k in rec["entries"]
            if k not in (primary, "brush_manager_order"))
        libs.append(rec)

    # ------------------------------------------------------------- rollups
    by_family = {}
    by_entry_kind = {}
    by_primary_kind = {}
    for lib in libs:
        fam = by_family.setdefault(lib["family"], {
            "library_files": 0,
            "primary_entries": 0,
            "primary_entries_excluding_builtins": 0,
            "all_definitions_in_files": 0,
            "parse_failures": 0,
            "primary_entry_kinds": {},
            "all_definitions_by_kind": {}})
        fam["library_files"] += 1
        fam["primary_entries"] += lib["primary_entry_count"]
        fam["primary_entries_excluding_builtins"] += \
            lib["primary_entry_count_excluding_builtins"]
        fam["all_definitions_in_files"] += lib["entry_total"]
        pk = lib["primary_entry_kind"]
        fam["primary_entry_kinds"][pk] = \
            fam["primary_entry_kinds"].get(pk, 0) + lib["primary_entry_count"]
        by_primary_kind[pk] = by_primary_kind.get(pk, 0) + \
            lib["primary_entry_count_excluding_builtins"]
        if lib["parse_error"]:
            fam["parse_failures"] += 1
        for k, v in lib["entries"].items():
            fam["all_definitions_by_kind"][k] = \
                fam["all_definitions_by_kind"].get(k, 0) + v["count"]
            by_entry_kind[k] = by_entry_kind.get(k, 0) + v["count"]

    failures = [{"path": l["path"], "error": l["parse_error"]}
                for l in libs if l["parse_error"]]

    out = {
        "schema_id": "handshake.studio.illustrator.library_contents.v1",
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "method": {
            "tool": "illustrator-library-parse.py",
            "preset_root": args.root,
            "app_launched": False,
            "channels": {
                ".ai": "PDF container -> /Private -> AIPrivateDataN concat -> "
                       "zstd (%AI24_ZStandard_Data) -> legacy AI PostScript stream; "
                       "entries read from Adobe's own section markers",
                ".ase": "Adobe Swatch Exchange binary, fully parsed",
                ".acb": "Adobe Color Book (8BCB) binary, fully parsed",
                ".acbl": "Adobe Swatchbook XML (legacy colour books), fully parsed",
            },
            "labelling": {
                "entry_names": "parsed",
                "entry_counts": "parsed",
                "swatch_kind": "parsed (AI palette operator preceding the Pc close)",
                "brush_kind": "parsed (AI8 plugin-object tool string)",
                "brush params string": "parsed verbatim, NOT decoded into fields",
                "acb components_raw": "parsed raw bytes; not scaled to a colour space",
            },
            "explicit_warning": "file counts and entry counts are different numbers; "
                                "every count in this file is an ENTRY count unless the "
                                "key says library_files",
        },
        "totals": {
            "library_files_scanned": len(libs),
            "library_files_by_extension": {
                ext: sum(1 for l in libs if l["path"].lower().endswith(ext))
                for ext in (".ai", ".ase", ".acb", ".acbl")},
            "library_files_parsed_ok": len(libs) - len(failures),
            "library_files_failed": len(failures),
            "_note": "library_* keys count FILES. Every other count below is an "
                     "ENTRY count.",
            "primary_entries_total": sum(l["primary_entry_count"] for l in libs),
            "primary_entries_total_excluding_builtins":
                sum(l["primary_entry_count_excluding_builtins"] for l in libs),
            "primary_entries_by_kind": dict(sorted(by_primary_kind.items())),
            "all_definitions_in_files_total":
                sum(l["entry_total"] for l in libs),
            "all_definitions_by_kind": dict(sorted(by_entry_kind.items())),
            "_definition_of_terms": {
                "primary_entries": "entries the library actually publishes to its "
                                   "panel (a Brushes library's brushes, a Symbols "
                                   "library's symbols, ...)",
                "all_definitions_in_files": "every definition of any kind found "
                                            "inside the files, including the "
                                            "gradients/patterns a library's own "
                                            "artwork happens to reference; larger "
                                            "than primary_entries and NOT the "
                                            "user-visible library size",
            },
        },
        "by_family": dict(sorted(by_family.items())),
        "parse_failures": failures,
        "libraries": libs,
    }

    os.makedirs(args.out, exist_ok=True)
    fp = os.path.join(args.out, "illustrator_library_contents.json")
    with open(fp, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print(f"WROTE {fp} ({os.path.getsize(fp):,} bytes)")
    print(json.dumps(out["totals"], indent=1))
    for fam, v in out["by_family"].items():
        print(f"  {fam}: files={v['library_files']} "
              f"primary_entries={v['primary_entries']} "
              f"(excl builtins {v['primary_entries_excluding_builtins']}) "
              f"all_defs={v['all_definitions_in_files']} "
              f"fail={v['parse_failures']} {v['primary_entry_kinds']}")
    if failures:
        print("FAILURES:")
        for f in failures:
            print("  ", f["path"], "->", f["error"][:120])
    return 0


if __name__ == "__main__":
    sys.exit(main())
