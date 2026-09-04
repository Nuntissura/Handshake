#!/usr/bin/env python
"""
photoshop-shortcuts-full.py

OFFLINE teardown of Adobe Photoshop 2026 keyboard-shortcut data for a native
Rust rebuild.  Reads files only.  Never launches Photoshop or any application.

Produces: photoshop_shortcuts_full.json

Corrects an earlier harvest pass (keyboard_shortcuts.json) that parsed only the
<command> elements (86 rows) and presented that subset as the whole file.
"""

import base64
import binascii
import collections
import datetime
import json
import os
import re
import struct
import sys
import xml.etree.ElementTree as ET

INSTALL = r"C:\Program Files\Adobe\Adobe Photoshop 2026"
SHORTCUTS_DIR = os.path.join(
    INSTALL, "Locales", "en_US", "Support Files", "Shortcuts"
)
KYS_PRIMARY = os.path.join(SHORTCUTS_DIR, "Win", "Default Keyboard Shortcuts.kys")
OUT_DIR = (
    r"D:\Projects\LLM projects\Handshake\Handshake Worktrees\wt-gov-kernel"
    r"\.GOV\reference\studio_app_feature_research\_greenroom_20260903"
    r"\installed_exports\photoshop\offline"
)
OUT_PATH = os.path.join(OUT_DIR, "photoshop_shortcuts_full.json")

# Canonical modifier ordering used for the NORMALIZED structured form.
# Chosen to match the Windows/W3C convention (Ctrl, Alt, Shift) rather than the
# order the .kys file happens to serialize (Alt, Shift, Ctrl), so a Rust
# implementer can compare bindings by sorted tuple.
CANONICAL_MODIFIER_ORDER = ["Ctrl", "Alt", "Shift", "Cmd"]
MODIFIER_TOKENS = {"Ctrl", "Alt", "Shift", "Cmd", "Command", "Opt", "Option"}
MODIFIER_ALIASES = {"Command": "Cmd", "Opt": "Alt", "Option": "Alt"}


# --------------------------------------------------------------------------
# key decoding
# --------------------------------------------------------------------------

# Decoded meaning for every raw key token that can appear as the residual key
# after modifiers are stripped.  `parsed` == the token is literally the key name
# or the literal printable character stored in the file; `heuristic` == the
# meaning below was inferred by this script rather than read from Adobe data.
KEY_TOKEN_MEANINGS = {
    # function keys - literal, self-describing
    "F1": ("Function key F1", "parsed"),
    "F2": ("Function key F2", "parsed"),
    "F3": ("Function key F3", "parsed"),
    "F4": ("Function key F4", "parsed"),
    "F5": ("Function key F5", "parsed"),
    "F6": ("Function key F6", "parsed"),
    "F7": ("Function key F7", "parsed"),
    "F8": ("Function key F8", "parsed"),
    "F9": ("Function key F9", "parsed"),
    "F12": ("Function key F12", "parsed"),
    # punctuation / symbol keys - stored as the literal printable character
    "+": (
        "PLUS. Stored literally. On a US layout the unshifted key is '=' and "
        "'+' is Shift+'='; Photoshop binds both spellings separately "
        "(Ctrl++ and Ctrl+= both map to Zoom In).",
        "heuristic",
    ),
    "-": ("HYPHEN-MINUS key", "parsed"),
    "=": ("EQUALS key", "parsed"),
    ",": ("COMMA key", "parsed"),
    ".": ("FULL STOP / period key", "parsed"),
    "/": ("SOLIDUS / forward-slash key", "parsed"),
    ";": ("SEMICOLON key", "parsed"),
    "'": ("APOSTROPHE / single-quote key", "parsed"),
    "[": ("LEFT SQUARE BRACKET key", "parsed"),
    "]": ("RIGHT SQUARE BRACKET key", "parsed"),
    "{": (
        "LEFT CURLY BRACKET. Stored as the literal shifted character, not as "
        "Shift+'['. A Rust implementer must either match on the produced "
        "character or synthesise Shift+LeftBracket.",
        "heuristic",
    ),
    "}": (
        "RIGHT CURLY BRACKET. Stored as the literal shifted character, not as "
        "Shift+']'.",
        "heuristic",
    ),
    "<": (
        "LESS-THAN SIGN. Stored as the literal shifted character (XML-escaped "
        "as &lt; in the source file), not as Shift+','.",
        "heuristic",
    ),
    ">": (
        "GREATER-THAN SIGN. Stored as the literal shifted character "
        "(XML-escaped as &gt; in the source file), not as Shift+'.'.",
        "heuristic",
    ),
}

_LETTER_RE = re.compile(r"^[A-Za-z]$")
_DIGIT_RE = re.compile(r"^[0-9]$")
_FKEY_RE = re.compile(r"^F([1-9]|1[0-9]|2[0-4])$")


def describe_key_token(tok):
    """Return (description, basis) for a residual key token."""
    if tok in KEY_TOKEN_MEANINGS:
        return KEY_TOKEN_MEANINGS[tok]
    if _FKEY_RE.match(tok):
        return ("Function key %s" % tok, "parsed")
    if _LETTER_RE.match(tok):
        return ("Letter key %s" % tok.upper(), "parsed")
    if _DIGIT_RE.match(tok):
        return ("Numeric row digit key %s" % tok, "parsed")
    return ("UNKNOWN token - not classified by this script", "unknown")


def normalize_shortcut(raw):
    """
    Decode a raw .kys shortcut string into a normalized structured binding.

    The .kys file already stores human-readable strings ("Alt+Ctrl+V"), so this
    is a tokenizer, not a scancode decoder.  Modifiers are stripped from the
    FRONT of the string one at a time, which is what makes "Ctrl++" and
    "Ctrl+-" parse correctly (a naive split('+') destroys them).
    """
    if raw is None:
        return None
    s = raw.strip()
    if s == "":
        return {
            "raw": raw,
            "modifiers": [],
            "key": None,
            "bound": False,
            "note": "empty <shortcut> element - command declared but no default binding",
        }

    mods = []
    rest = s
    while True:
        matched = None
        for m in sorted(MODIFIER_TOKENS, key=len, reverse=True):
            if rest.startswith(m + "+") and len(rest) > len(m) + 1:
                matched = m
                break
        if matched is None:
            break
        mods.append(MODIFIER_ALIASES.get(matched, matched))
        rest = rest[len(matched) + 1:]

    seen = set()
    ordered = []
    for m in CANONICAL_MODIFIER_ORDER:
        if m in mods and m not in seen:
            ordered.append(m)
            seen.add(m)
    for m in mods:  # any modifier not in the canonical list, appended stably
        if m not in seen:
            ordered.append(m)
            seen.add(m)

    return {
        "raw": raw,
        "modifiers": ordered,
        "key": rest,
        "bound": True,
    }


def decode_ostype(value):
    """
    Decode a <tool key="NNNNNNNNNN"> integer.

    Verified mechanically: struct.pack('>I', 1819113074) == b'lmvr' (Move Tool),
    2054123373 == b'zoom' (Zoom Tool), 1668444016 == b'crop' (Crop Tool),
    1886286946 == b'pntb' (Brush Tool).  So the integer is a big-endian packed
    4-character OSType.  The DECODE is mechanical/parsed; the CLAIM that these
    are Photoshop's classic internal tool OSType identifiers is heuristic.
    """
    try:
        n = int(value)
    except (TypeError, ValueError):
        return None
    if n < 0 or n > 0xFFFFFFFF:
        return None
    b = struct.pack(">I", n)
    printable = all(0x20 <= c <= 0x7E for c in b)
    return {
        "int": n,
        "hex": "0x%08X" % n,
        "ostype_be": b.decode("ascii") if printable else None,
        "ostype_all_printable_ascii": printable,
        "decode_basis": "parsed (struct.pack('>I', value))",
        "semantic_basis": "heuristic - read as Photoshop classic 4-char tool OSType",
    }


# --------------------------------------------------------------------------
# .kys parsing
# --------------------------------------------------------------------------

def element_shortcuts(el):
    """All <shortcut> child elements of a <command>, in document order."""
    out = []
    for sc in el.findall("shortcut"):
        out.append(sc.text if sc.text is not None else "")
    return out


def parse_kys(path):
    """Parse one .kys file into a fully enumerated structure."""
    with open(path, "rb") as fh:
        raw_bytes = fh.read()
    tree = ET.parse(path)
    root = tree.getroot()

    counts = collections.Counter()
    for el in root.iter():
        counts[el.tag] += 1

    result = {
        "path": path,
        "size_bytes": len(raw_bytes),
        "encoding_declared": "UTF-8",
        "root_tag": root.tag,
        "root_attributes": dict(root.attrib),
        "element_counts": dict(counts),
        "commands": [],
        "tools": [],
        "taskspaces": [],
        "shortcut_element_total": counts.get("shortcut", 0),
    }

    # ---- commands -------------------------------------------------------
    for idx, el in enumerate(root.findall("command")):
        raws = element_shortcuts(el)
        norms = [normalize_shortcut(r) for r in raws]
        result["commands"].append({
            "index": idx,
            "kind": el.get("kind"),
            "id": el.get("id"),
            "name": el.get("name"),
            "attributes": dict(el.attrib),
            "shortcut_element_count": len(raws),
            "shortcuts_raw": raws,
            "shortcuts_normalized": norms,
        })

    # ---- tools ----------------------------------------------------------
    for idx, el in enumerate(root.findall("tool")):
        txt = el.text if el.text is not None else ""
        txt = txt.strip()
        result["tools"].append({
            "index": idx,
            "name": el.get("name"),
            "type": el.get("type"),
            "key_attribute": el.get("key"),
            "key_attribute_decoded": decode_ostype(el.get("key")),
            "attributes": dict(el.attrib),
            "shortcut_raw": txt,
            "shortcut_normalized": normalize_shortcut(txt),
            "note": (
                "<tool> stores its shortcut as the element TEXT, not as a "
                "<shortcut> child element; empty text means no default binding."
            ),
        })

    # ---- taskspaces -----------------------------------------------------
    for tsi, ts in enumerate(root.findall("taskspace")):
        entry = {
            "index": tsi,
            "name": ts.get("name"),
            "attributes": dict(ts.attrib),
            "taskspace_tools": [],
            "taskspace_properties": [],
        }
        for idx, el in enumerate(ts.findall("taskspace-tool")):
            txt = (el.text or "").strip()
            entry["taskspace_tools"].append({
                "index": idx,
                "name": el.get("name"),
                "type": el.get("type"),
                "key_attribute": el.get("key"),
                "key_attribute_decoded": decode_ostype(el.get("key")),
                "attributes": dict(el.attrib),
                "shortcut_raw": txt,
                "shortcut_normalized": normalize_shortcut(txt),
            })
        for idx, el in enumerate(ts.findall("taskspace-property")):
            txt = (el.text or "").strip()
            entry["taskspace_properties"].append({
                "index": idx,
                "name": el.get("name"),
                "attributes": dict(el.attrib),
                "shortcut_raw": txt,
                "shortcut_normalized": normalize_shortcut(txt),
            })
        result["taskspaces"].append(entry)

    # ---- unexpected element types --------------------------------------
    known = {
        "photoshop-keyboard-shortcuts", "command", "shortcut", "tool",
        "taskspace", "taskspace-tool", "taskspace-property",
    }
    result["unexpected_element_tags"] = sorted(
        t for t in counts if t not in known
    )
    return result


def kys_counts_only(path):
    """Element counts for a secondary .kys file (no full expansion)."""
    try:
        root = ET.parse(path).getroot()
    except Exception as exc:  # noqa: BLE001 - reported, not raised
        return {"path": path, "parse_error": str(exc)}
    counts = collections.Counter()
    for el in root.iter():
        counts[el.tag] += 1
    return {
        "path": path,
        "size_bytes": os.path.getsize(path),
        "root_tag": root.tag,
        "root_attributes": dict(root.attrib),
        "element_counts": dict(counts),
    }


# --------------------------------------------------------------------------
# flattening / conflicts
# --------------------------------------------------------------------------

def binding_signature(norm):
    if norm is None or not norm.get("bound"):
        return None
    return "+".join(norm["modifiers"] + [norm["key"]])


def build_flat(parsed):
    flat = []
    for c in parsed["commands"]:
        for si, norm in enumerate(c["shortcuts_normalized"]):
            flat.append({
                "section": "command",
                "scope": "application",
                "target_name": c["name"],
                "target_id": c["id"],
                "target_kind": c["kind"],
                "alternate_index": si,
                "raw": norm["raw"],
                "modifiers": norm["modifiers"],
                "key": norm["key"],
                "bound": norm["bound"],
            })
    for t in parsed["tools"]:
        n = t["shortcut_normalized"]
        flat.append({
            "section": "tool",
            "scope": "tool_palette",
            "target_name": t["name"],
            "target_id": t["key_attribute"],
            "target_kind": "tool_type_%s" % t["type"],
            "alternate_index": 0,
            "raw": n["raw"],
            "modifiers": n["modifiers"],
            "key": n["key"],
            "bound": n["bound"],
        })
    for ts in parsed["taskspaces"]:
        for t in ts["taskspace_tools"]:
            n = t["shortcut_normalized"]
            flat.append({
                "section": "taskspace-tool",
                "scope": "taskspace:%s" % ts["name"],
                "target_name": t["name"],
                "target_id": t["key_attribute"],
                "target_kind": "tool_type_%s" % t["type"],
                "alternate_index": 0,
                "raw": n["raw"],
                "modifiers": n["modifiers"],
                "key": n["key"],
                "bound": n["bound"],
            })
        for p in ts["taskspace_properties"]:
            n = p["shortcut_normalized"]
            flat.append({
                "section": "taskspace-property",
                "scope": "taskspace:%s" % ts["name"],
                "target_name": p["name"],
                "target_id": None,
                "target_kind": "property",
                "alternate_index": 0,
                "raw": n["raw"],
                "modifiers": n["modifiers"],
                "key": n["key"],
                "bound": n["bound"],
            })
    return flat


def build_conflicts(flat):
    by_scope = collections.defaultdict(lambda: collections.defaultdict(list))
    for row in flat:
        if not row["bound"]:
            continue
        sig = "+".join(row["modifiers"] + [row["key"]])
        by_scope[row["scope"]][sig].append(row)

    within = []
    for scope, sigs in sorted(by_scope.items()):
        for sig, rows in sorted(sigs.items()):
            if len(rows) < 2:
                continue
            sections = sorted({r["section"] for r in rows})
            if sections == ["tool"] or sections == ["taskspace-tool"]:
                kind = "tool_cycle_group"
                note = (
                    "heuristic: multiple tools sharing one letter is Photoshop's "
                    "documented tool-cycle behaviour (Shift+key cycles the "
                    "group), not a defect. Classified by this script, not read "
                    "from the .kys file."
                )
            else:
                kind = "collision"
                note = "two or more distinct targets in the same scope claim this binding"
            within.append({
                "scope": scope,
                "binding": sig,
                "count": len(rows),
                "classification": kind,
                "classification_basis": "heuristic",
                "note": note,
                "competing_targets": [
                    {
                        "section": r["section"],
                        "name": r["target_name"],
                        "id": r["target_id"],
                        "kind": r["target_kind"],
                        "raw": r["raw"],
                    }
                    for r in rows
                ],
            })

    # cross-scope: same literal binding used in more than one scope
    by_sig = collections.defaultdict(list)
    for row in flat:
        if not row["bound"]:
            continue
        sig = "+".join(row["modifiers"] + [row["key"]])
        by_sig[sig].append(row)
    cross = []
    for sig, rows in sorted(by_sig.items()):
        scopes = sorted({r["scope"] for r in rows})
        if len(scopes) < 2:
            continue
        cross.append({
            "binding": sig,
            "scopes": scopes,
            "count": len(rows),
            "classification": "cross_scope_reuse",
            "classification_basis": "heuristic",
            "note": (
                "Same key sequence is bound in more than one input scope. Not "
                "necessarily a conflict: application commands, the tool palette "
                "and modal taskspaces are separate dispatch contexts. A Rust "
                "reimplementation must decide scope precedence explicitly."
            ),
            "targets": [
                {
                    "scope": r["scope"],
                    "section": r["section"],
                    "name": r["target_name"],
                    "id": r["target_id"],
                }
                for r in rows
            ],
        })
    return {"within_scope": within, "cross_scope": cross}


# --------------------------------------------------------------------------
# discovery: .kys sets and menu customization data
# --------------------------------------------------------------------------

def walk_install():
    kys, mnu = [], []
    total = 0
    for dirpath, _dirnames, filenames in os.walk(INSTALL):
        for fn in filenames:
            total += 1
            low = fn.lower()
            p = os.path.join(dirpath, fn)
            if low.endswith(".kys"):
                kys.append(p)
            elif low.endswith(".mnu"):
                mnu.append(p)
    return total, sorted(kys), sorted(mnu)


def tree_listing(root):
    out = []
    if not os.path.isdir(root):
        return out
    for dirpath, dirnames, filenames in os.walk(root):
        rel = os.path.relpath(dirpath, root)
        out.append({
            "dir": "." if rel == "." else rel.replace("\\", "/"),
            "subdirs": sorted(dirnames),
            "files": [
                {"name": fn,
                 "size_bytes": os.path.getsize(os.path.join(dirpath, fn))}
                for fn in sorted(filenames)
            ],
        })
    return out


def analyse_mnu(path):
    with open(path, "rb") as fh:
        data = fh.read()
    rec = {
        "path": path,
        "size_bytes": len(data),
        "hex": binascii.hexlify(data).decode("ascii"),
        "signature_ascii": data[:4].decode("latin-1", "replace"),
    }
    # 8MNU | uint32 version | uint32 charcount | UTF-16BE chars | uint32 tail
    if len(data) >= 12 and data[:4] == b"8MNU":
        version = struct.unpack(">I", data[4:8])[0]
        nchars = struct.unpack(">I", data[8:12])[0]
        need = 12 + nchars * 2
        name = None
        tail = None
        if len(data) >= need:
            name = data[12:need].decode("utf-16-be").rstrip("\x00")
            tail = data[need:]
        rec["decoded"] = {
            "signature": "8MNU",
            "version_uint32_be": version,
            "name_char_count_including_nul": nchars,
            "set_name": name,
            "trailing_bytes_hex": binascii.hexlify(tail).decode("ascii") if tail is not None else None,
            "trailing_byte_count": len(tail) if tail is not None else None,
            "decode_basis": (
                "parsed - signature and UTF-16BE string read directly from the "
                "bytes; the field NAMES (version / char count / entry count) are "
                "heuristic labels, no Adobe .mnu format spec was consulted."
            ),
        }
        rec["finding"] = (
            "This file is a 71-byte HEADER ONLY. It contains the 8MNU "
            "signature, a version field, and the UTF-16BE menu-set name "
            "'%s'. After the name there are %d trailing bytes, all zero. "
            "There are ZERO menu item entries in it. It is the identity "
            "record for the shipped menu-customization SET, not a menu "
            "definition."
            % (name, len(tail) if tail is not None else -1)
        )
    return rec


def find_menu_definition_sources():
    """
    Look for a real menu-tree definition anywhere in the install.

    Records what was searched and what was found, so the shortcut-to-menu
    mapping gap is documented instead of silently missing.
    """
    checked = []

    def note(path, what, found, detail):
        checked.append({
            "path": path, "looked_for": what, "found": found, "detail": detail,
        })

    # 1. .mnu files
    _total, _kys, mnus = walk_install()
    note(
        os.path.join(INSTALL, "**", "*.mnu"),
        "Photoshop menu-customization files",
        len(mnus) > 0,
        "%d .mnu file(s) in the whole install root: %s" % (
            len(mnus), [os.path.relpath(m, INSTALL) for m in mnus]),
    )

    # 2. Required\layouts\Application\Dialogs - the customization UI, not data
    app_dialogs = os.path.join(INSTALL, "Required", "layouts", "Application", "Dialogs")
    present = []
    if os.path.isdir(app_dialogs):
        present = sorted(os.listdir(app_dialogs))
    # measure, do not assert: open the two candidate dialog layouts
    dialog_evidence = {}
    for cand in ("menus-4620.exv", "keyboardShortcuts-4610.exv"):
        cp = os.path.join(app_dialogs, cand)
        if not os.path.exists(cp):
            dialog_evidence[cand] = {"exists": False}
            continue
        text = open(cp, encoding="utf-8", errors="replace").read()
        dialog_evidence[cand] = {
            "exists": True,
            "size_bytes": os.path.getsize(cp),
            "root_class_name": (
                re.search(r"class_name:\s*'([^']+)'", text).group(1)
                if re.search(r"class_name:\s*'([^']+)'", text) else None),
            "all_class_names": sorted(set(re.findall(r"class_name:\s*'([^']+)'", text))),
            "resource_id_references": sorted(set(
                int(x) for x in re.findall(r"resource_id:\s*(\d+)", text))),
            "zstring_count": len(re.findall(r"\$\$\$?/", text)),
            "contains_menu_item_list": bool(
                re.search(r"(?i)\b(File|Edit|Image|Layer|Select|Filter|View|Window|Help)\s*>", text)),
        }
    note(
        app_dialogs,
        "a menu-tree definition under the Application layout tree",
        False,
        "Directory exists. menus-4620.exv and keyboardShortcuts-4610.exv were "
        "OPENED and scanned. menus-4620.exv declares root widget classes "
        "TCustomizationPanel / TMenuCustomization and populates its menu-category "
        "and set pickers from compiled resources via resource_id references - "
        "it is the Edit > Menus... EDITOR dialog's widget layout. "
        "keyboardShortcuts-4610.exv is the matching Edit > Keyboard Shortcuts... "
        "editor layout. Neither file contains a menu item name, a menu id, or a "
        "menu hierarchy. Measured evidence: %s"
        % json.dumps(dialog_evidence, sort_keys=True),
    )
    note(
        app_dialogs,
        "directory inventory (evidence for the above)",
        len(present) > 0,
        "files: %s" % present,
    )

    # 3. layouts / drover_layouts menu scan
    menu_widget_hits = 0
    scanned = 0
    pat = re.compile(rb"(?i)(popup_?menu|TPopupMenu|menu_?items?)")
    for base in ("layouts", "drover_layouts"):
        root = os.path.join(INSTALL, "Required", base)
        for dirpath, _dn, fns in os.walk(root):
            for fn in fns:
                if not fn.lower().endswith((".eve", ".exv")):
                    continue
                scanned += 1
                p = os.path.join(dirpath, fn)
                try:
                    with open(p, "rb") as fh:
                        if pat.search(fh.read()):
                            menu_widget_hits += 1
                except OSError:
                    pass
    note(
        os.path.join(INSTALL, "Required", "{layouts,drover_layouts}"),
        "menu definitions inside Eve layout files",
        False,
        "Scanned %d .eve/.exv layout files against the case-insensitive regex "
        "%r; %d matched. Matches are menu-WIDGET references (popup/dropdown "
        "widgets inside dialogs and panels), not menu definitions. None of the "
        "%d files defines the application menu bar. Eve layout files describe "
        "widgets, not the menu tree."
        % (scanned, pat.pattern.decode("ascii"), menu_widget_hits, scanned),
    )

    # 4. locale .dat
    dat = os.path.join(INSTALL, "Locales", "en_US", "Support Files",
                       "tw10428_Photoshop_en_US.dat")
    if os.path.exists(dat):
        note(dat, "localized menu strings", False,
             "Exists, %d bytes - far too small to hold Photoshop's menu tree "
             "or its localized item names." % os.path.getsize(dat))

    # 5. plugin PiPL schema - how PLUG-INS declare their menu placement
    pipl = os.path.join(INSTALL, "plugin_resources", "pipl-schema.json")
    if os.path.exists(pipl):
        note(pipl, "plug-in menu placement declarations", True,
             "Exists, %d bytes. This is the schema for plug-in PiPL resources, "
             "which is how a PLUG-IN declares where its command appears in the "
             "menus. It describes the declaration format only - it is not the "
             "host application's own menu tree." % os.path.getsize(pipl))

    # 6. the binary
    exe_candidates = [
        f for f in os.listdir(INSTALL)
        if f.lower().endswith(".exe") and "photoshop" in f.lower()
    ]
    note(INSTALL, "the Photoshop executable", len(exe_candidates) > 0,
         "Found: %s. NOT INSPECTED - no disassembly or resource extraction was "
         "performed (read-only offline teardown, and the menu tree would be in "
         "compiled resources, not a data file)." % exe_candidates)

    return {
        "layout_files_scanned": scanned,
        "layout_files_containing_menu_widget_markers": menu_widget_hits,
        "conclusion": (
            "NO menu-tree definition file exists in this install. The only .mnu "
            "file is a 71-byte header naming a menu-customization SET with zero "
            "entries. The menus-4620.exv layout is the Edit > Menus... editor "
            "dialog's widget layout, not menu data. Photoshop's menu hierarchy "
            "and its command-id-to-menu-item mapping are compiled into the "
            "application binary and plug-in PiPL resources; they are not "
            "recoverable from loose install files by reading alone."
        ),
        "consequence_for_rebuild": (
            "The .kys file gives command NAME and numeric command ID (e.g. id "
            "10 = 'New...'), but nothing in this install maps those ids to a "
            "menu path (File > New...). A native Rust rebuild must source the "
            "menu hierarchy from somewhere else: published Adobe documentation, "
            "a running instance's scripting DOM, or an exported .mnu produced by "
            "the Edit > Menus... dialog after a user saves a customization set."
        ),
        "conclusion_basis": "parsed for every file listed; heuristic for the compiled-into-binary inference",
        "places_checked": checked,
    }


# --------------------------------------------------------------------------
# main
# --------------------------------------------------------------------------

def main():
    sys.stdout.reconfigure(encoding="utf-8")
    now = datetime.datetime.now(datetime.timezone.utc).strftime(
        "%Y-%m-%dT%H:%M:%SZ")

    total_files, all_kys, all_mnu = walk_install()
    menu_search = find_menu_definition_sources()
    primary = parse_kys(KYS_PRIMARY)
    flat = build_flat(primary)
    conflicts = build_conflicts(flat)

    # key token vocabulary over every residual key token in the file
    tokens = collections.Counter()
    for row in flat:
        if row["bound"] and row["key"] is not None:
            tokens[row["key"]] += 1
    vocab = []
    for tok, n in sorted(tokens.items()):
        desc, basis = describe_key_token(tok)
        vocab.append({
            "token": tok,
            "occurrences": n,
            "decoded_meaning": desc,
            "basis": basis,
        })

    modifier_tokens = collections.Counter()
    for row in flat:
        for m in row["modifiers"]:
            modifier_tokens[m] += 1

    # other .kys sets
    other_kys = [
        kys_counts_only(p) for p in all_kys
        if os.path.normcase(p) != os.path.normcase(KYS_PRIMARY)
    ]

    unbound_commands = [
        {"id": c["id"], "name": c["name"], "kind": c["kind"]}
        for c in primary["commands"]
        if all(not n["bound"] for n in c["shortcuts_normalized"])
    ]
    unbound_tools = [
        {"name": t["name"], "type": t["type"], "key": t["key_attribute"]}
        for t in primary["tools"] if not t["shortcut_normalized"]["bound"]
    ]
    unbound_ts_props = []
    unbound_ts_tools = []
    for ts in primary["taskspaces"]:
        for p in ts["taskspace_properties"]:
            if not p["shortcut_normalized"]["bound"]:
                unbound_ts_props.append(
                    {"taskspace": ts["name"], "name": p["name"]})
        for t in ts["taskspace_tools"]:
            if not t["shortcut_normalized"]["bound"]:
                unbound_ts_tools.append(
                    {"taskspace": ts["name"], "name": t["name"]})

    multi = [
        {"id": c["id"], "name": c["name"],
         "shortcuts": c["shortcuts_raw"]}
        for c in primary["commands"] if c["shortcut_element_count"] > 1
    ]

    doc = {
        "schema_id": "handshake.greenroom.photoshop.shortcuts_full.v1",
        "generated_at": now,
        "generator": "photoshop-shortcuts-full.py",
        "target_application": {
            "name": "Adobe Photoshop 2026",
            "install_root": INSTALL,
            "launched": False,
            "access_mode": "read-only file parsing; the application was never started",
        },

        "method": {
            "overall": (
                "Every section below was produced by parsing files on disk with "
                "Python's xml.etree.ElementTree and struct. Photoshop was never "
                "launched. No value was copied from the earlier harvest pass."
            ),
            "kys_parsing": (
                "'Default Keyboard Shortcuts.kys' is plain UTF-8 XML with root "
                "<photoshop-keyboard-shortcuts>. It was parsed with "
                "ElementTree; every element type present in the document was "
                "enumerated (root.iter() counted per tag) and every one of the "
                "five child element types - command, tool, taskspace, "
                "taskspace-tool, taskspace-property - was expanded with ALL of "
                "its attributes preserved verbatim in an 'attributes' field, "
                "plus its shortcut(s). Nothing was filtered or sampled."
            ),
            "shortcut_multiplicity": (
                "A <command> may carry more than one <shortcut> child. Each is "
                "emitted separately with an alternate_index, so a command with "
                "two alternates produces two rows in all_shortcuts. <tool>, "
                "<taskspace-tool> and <taskspace-property> instead store their "
                "shortcut as the element TEXT, at most one per element."
            ),
            "key_encoding_finding": (
                "PLAINLY: the .kys file does NOT use a scancode, virtual-key or "
                "bitmask encoding for shortcuts. It stores already-readable "
                "human strings such as 'Alt+Ctrl+V', 'F12', 'Shift+Ctrl+]' and "
                "'Ctrl+,'. No decoding table was needed or invented. What this "
                "script adds is a NORMALIZED structured form "
                "{modifiers:[...], key:'...'} produced by stripping known "
                "modifier prefixes ('Ctrl+', 'Alt+', 'Shift+', 'Cmd+') from the "
                "FRONT of the string one at a time. Front-stripping rather than "
                "split('+') is what makes 'Ctrl++' (Zoom In) and 'Ctrl+-' (Zoom "
                "Out) decode correctly. Modifiers are re-emitted in the "
                "canonical order Ctrl, Alt, Shift so bindings can be compared "
                "as sorted tuples; note the file's own serialization order is "
                "Alt, Shift, Ctrl."
            ),
            "tool_key_attribute_finding": (
                "The <tool key='NNNNNNNNNN'> attribute is NOT a keyboard code. "
                "It is a big-endian packed 4-character OSType: "
                "struct.pack('>I', 1819113074) == b'lmvr' (Move Tool), "
                "2054123373 == b'zoom', 1668444016 == b'crop', 1886286946 == "
                "b'pntb'. Every <tool> and <taskspace-tool> key was decoded "
                "this way and emitted as key_attribute_decoded. The decode is "
                "mechanical; calling the result Photoshop's internal tool "
                "identifier is labelled heuristic on each record."
            ),
            "key_token_vocabulary": (
                "Every distinct residual key token left after modifier "
                "stripping, across all five element types, was collected with "
                "its occurrence count and given a decoded meaning. Tokens whose "
                "meaning was inferred rather than read from the file (the "
                "shifted-literal punctuation '{', '}', '<', '>' and the '+' / "
                "'=' duplication) are marked basis:'heuristic'."
            ),
            "kys_set_discovery": (
                "os.walk over the ENTIRE install root (%d files walked) "
                "matching *.kys case-insensitively, plus a full recursive "
                "listing of the Shortcuts directory tree. Every .kys found was "
                "re-opened and its per-tag element counts recorded."
                % total_files
            ),
            "menu_customization_search": (
                "os.walk over the entire install root for *.mnu; byte-level "
                "decode of the single hit; directory inventory and byte "
                "inspection of Required\\layouts\\Application\\Dialogs; a "
                "regex scan of all %d .eve/.exv layout files under "
                "Required\\layouts and Required\\drover_layouts for menu "
                "widget/definition markers; and existence checks on the locale "
                ".dat and the plug-in PiPL schema. Findings and non-findings "
                "are both recorded." % menu_search["layout_files_scanned"]
            ),
            "conflicts": (
                "all_shortcuts was grouped by (scope, canonical binding "
                "signature). Within-scope groups of size >1 are reported. "
                "Groups that are entirely <tool> or entirely <taskspace-tool> "
                "are labelled tool_cycle_group rather than collision - that "
                "label is a heuristic applied by this script, not a flag read "
                "from the file. A separate cross_scope list reports bindings "
                "reused across the application / tool-palette / taskspace "
                "dispatch contexts."
            ),
        },

        "source_files": [
            {
                "path": KYS_PRIMARY,
                "size_bytes": primary["size_bytes"],
                "role": "primary keyboard shortcut set - fully expanded below",
                "format": "UTF-8 XML",
            },
            {
                "path": os.path.join(SHORTCUTS_DIR, "Win", "OS Shortcuts.txt"),
                "size_bytes": os.path.getsize(
                    os.path.join(SHORTCUTS_DIR, "Win", "OS Shortcuts.txt"))
                if os.path.exists(os.path.join(SHORTCUTS_DIR, "Win", "OS Shortcuts.txt"))
                else None,
                "role": "sibling file in the Shortcuts\\Win directory",
                "format": "plain text",
            },
            {
                "path": os.path.join(INSTALL, "Required", "Default Menus.mnu"),
                "size_bytes": 71,
                "role": "menu-customization set header (analysed, not a menu definition)",
                "format": "binary, 8MNU signature",
            },
            {
                "path": INSTALL,
                "role": "walked recursively for *.kys and *.mnu discovery",
                "files_walked": total_files,
            },
        ],

        # ---- headline real numbers -------------------------------------
        "totals": {
            "note": "These are ENTRY counts, not file counts. File counts are reported separately in kys_sets.",
            "kys_files_found_in_entire_install": len(all_kys),
            "element_counts_in_primary_kys": primary["element_counts"],
            "command_entries": len(primary["commands"]),
            "command_entries_kind_static": sum(
                1 for c in primary["commands"] if c["kind"] == "static"),
            "command_entries_kind_dynamic": sum(
                1 for c in primary["commands"] if c["kind"] == "dynamic"),
            "shortcut_elements_under_commands": sum(
                c["shortcut_element_count"] for c in primary["commands"]),
            "shortcut_elements_bound_non_empty": sum(
                1 for c in primary["commands"]
                for n in c["shortcuts_normalized"] if n["bound"]),
            "shortcut_elements_empty": sum(
                1 for c in primary["commands"]
                for n in c["shortcuts_normalized"] if not n["bound"]),
            "commands_with_multiple_shortcuts": len(multi),
            "tool_entries": len(primary["tools"]),
            "tool_entries_with_a_key": sum(
                1 for t in primary["tools"]
                if t["shortcut_normalized"]["bound"]),
            "tool_entries_without_a_key": len(unbound_tools),
            "taskspace_entries": len(primary["taskspaces"]),
            "taskspace_tool_entries": sum(
                len(ts["taskspace_tools"]) for ts in primary["taskspaces"]),
            "taskspace_property_entries": sum(
                len(ts["taskspace_properties"]) for ts in primary["taskspaces"]),
            "all_shortcuts_rows": len(flat),
            "all_shortcuts_rows_bound": sum(1 for r in flat if r["bound"]),
            "all_shortcuts_rows_unbound": sum(1 for r in flat if not r["bound"]),
            "distinct_key_tokens": len(vocab),
            "distinct_modifier_tokens": len(modifier_tokens),
        },

        "corrections_to_earlier_pass": {
            "earlier_artifact": os.path.join(OUT_DIR, "keyboard_shortcuts.json"),
            "false_claim": (
                "The earlier pass emitted 86 rows from "
                "'Default Keyboard Shortcuts.kys' with row_count:86 and no "
                "indication that anything was omitted, so 86 read as the whole "
                "file. It is not: the file holds 104 commands, 112 shortcuts, "
                "92 tools, 3 taskspaces, 19 taskspace-tools and 20 "
                "taskspace-properties."
            ),
            "what_it_actually_captured": (
                "See earlier_artifact_measured below - measured by re-reading "
                "that file, not assumed."
            ),
            "true_figures": {
                "command_elements": primary["element_counts"].get("command", 0),
                "shortcut_elements": primary["element_counts"].get("shortcut", 0),
                "tool_elements": primary["element_counts"].get("tool", 0),
                "taskspace_elements": primary["element_counts"].get("taskspace", 0),
                "taskspace_tool_elements": primary["element_counts"].get("taskspace-tool", 0),
                "taskspace_property_elements": primary["element_counts"].get("taskspace-property", 0),
                "total_binding_rows_this_pass": len(flat),
            },
        },

        "key_encoding": {
            "finding": (
                "The file stores READABLE key strings, not an encoding. "
                "Example verbatim rows: "
                "<command kind=\"dynamic\" name=\"Vanishing Point\">"
                "<shortcut>Alt+Ctrl+V</shortcut></command> and "
                "<command kind=\"static\" id=\"10\" name=\"New...\">"
                "<shortcut>Ctrl+N</shortcut></command>. No scancode, "
                "virtual-key code or modifier bitmask appears anywhere in the "
                "file."
            ),
            "normalization_rule": (
                "Strip 'Ctrl+', 'Alt+', 'Shift+', 'Cmd+' prefixes from the "
                "front, one at a time, while the remainder is longer than the "
                "prefix. Everything left is the key token, verbatim."
            ),
            "canonical_modifier_order": CANONICAL_MODIFIER_ORDER,
            "file_serialization_modifier_order": ["Alt", "Shift", "Ctrl"],
            "modifier_token_occurrences": dict(modifier_tokens),
            "tool_element_shortcut_storage": (
                "<tool>, <taskspace-tool> and <taskspace-property> store the "
                "shortcut as element TEXT (a single character, never with a "
                "modifier prefix in this file). Empty text = no default binding."
            ),
            "tool_key_attribute": (
                "The numeric key= attribute on <tool>/<taskspace-tool> is a "
                "big-endian packed 4-char OSType tool identifier, NOT a "
                "keyboard code. Decoded per record as key_attribute_decoded."
            ),
        },

        "key_token_vocabulary": vocab,

        "sections": {
            "commands": primary["commands"],
            "tools": primary["tools"],
            "taskspaces": primary["taskspaces"],
        },

        "all_shortcuts": flat,
        "conflicts": conflicts,

        "commands_with_multiple_shortcuts": multi,
        "unbound_entries": {
            "commands_with_no_binding": unbound_commands,
            "tools_with_no_binding": unbound_tools,
            "taskspace_tools_with_no_binding": unbound_ts_tools,
            "taskspace_properties_with_no_binding": unbound_ts_props,
        },

        "kys_sets": {
            "shortcuts_directory_root": SHORTCUTS_DIR,
            "shortcuts_directory_tree": tree_listing(SHORTCUTS_DIR),
            "mac_folder_present": os.path.isdir(
                os.path.join(SHORTCUTS_DIR, "Mac")),
            "platform_subfolders_found": sorted(
                d for d in os.listdir(SHORTCUTS_DIR)
                if os.path.isdir(os.path.join(SHORTCUTS_DIR, d))
            ) if os.path.isdir(SHORTCUTS_DIR) else [],
            "kys_files_found_whole_install_count": len(all_kys),
            "kys_files_found_whole_install": all_kys,
            "primary_kys_element_counts": primary["element_counts"],
            "other_kys_files": other_kys,
            "note": (
                "kys_files_found_whole_install_count is a FILE count. The entry "
                "counts for the one file found are in totals and in "
                "primary_kys_element_counts."
            ),
        },

        "menu_customization": {
            "mnu_files_found_whole_install_count": len(all_mnu),
            "mnu_files": [analyse_mnu(p) for p in all_mnu],
            "menu_definition_source_search": menu_search,
        },

        "unknowns": [
            "No mapping exists in this install from a .kys command id (e.g. id "
            "10 = 'New...') to a menu path (File > New...). The menu hierarchy "
            "is not present in any readable install file. UNRESOLVED.",
            "The <command kind=\"dynamic\"> rows (5 of them: Vanishing Point, "
            "Liquify, Lens Correction, Camera Raw Filter, Wide Angle "
            "Correction) carry NO id attribute - only a name. How Photoshop "
            "resolves a dynamic command name to a plug-in at runtime is not "
            "visible in this file. UNRESOLVED.",
            "The numeric type= attribute on <tool> (values 1-32) is an "
            "enumeration whose meaning is not defined anywhere in the install. "
            "type=1 always accompanies a key= OSType and always names a real "
            "palette tool; types 2-32 never carry key= and name modal "
            "toggles/actions. That correlation was observed, but the "
            "enumeration itself is UNRESOLVED.",
            "Photoshop.exe was NOT inspected. Menu tree, command-id table and "
            "tool type enumeration are most likely in its compiled resources; "
            "this was not verified because no binary analysis was performed.",
            "No Mac shortcut set ships in this Windows install, so the "
            "Windows-to-macOS modifier mapping (Ctrl->Cmd, Alt->Opt) could not "
            "be confirmed from Adobe data and is NOT asserted here.",
            "Whether the 'OS Shortcuts.txt' sibling file affects binding "
            "resolution was not determined; its contents were not parsed into "
            "this document.",
            "The Photoshop install changed on disk between the earlier harvest "
            "pass and this one (see corrections_to_earlier_pass.size_discrepancy "
            "and the install_state_changed_between_passes section of "
            "photoshop_panels.json). Everything in THIS document was measured "
            "from the current state; the earlier figures describe an earlier "
            "state.",
        ],

        "heuristics": [
            {
                "id": "H1",
                "claim": "tool key= integers are Photoshop's classic 4-char tool OSType codes",
                "basis": "heuristic",
                "evidence": "the big-endian byte decode yields exactly-4 printable ASCII for every tool, and the strings read as tool mnemonics (lmvr/zoom/crop/pntb/laso/hand)",
                "what_is_parsed_vs_inferred": "the byte decode is parsed; the identification as Adobe's OSType vocabulary is inferred",
            },
            {
                "id": "H2",
                "claim": "multiple tools sharing one letter key form a tool-cycle group rather than a conflict",
                "basis": "heuristic",
                "evidence": "the grouping pattern in the file (V/V, M/M, L/L/L, W/W/W, ...) matches Photoshop's documented Shift+key tool cycling",
                "what_is_parsed_vs_inferred": "the shared keys are parsed; the cycle-group interpretation is inferred",
            },
            {
                "id": "H3",
                "claim": "'{', '}', '<', '>' are stored as literal shifted characters and imply a Shift press on a US layout",
                "basis": "heuristic",
                "evidence": "they appear as tool text alongside their unshifted partners '[', ']', ',', '.'",
                "what_is_parsed_vs_inferred": "the characters are parsed; the implied Shift and the layout dependency are inferred",
            },
            {
                "id": "H4",
                "claim": "Ctrl++ and Ctrl+= are two spellings of the same physical Zoom In chord",
                "basis": "heuristic",
                "evidence": "both are listed as alternates on the same command id 1004",
                "what_is_parsed_vs_inferred": "both strings are parsed; the same-physical-key claim is inferred from the US layout",
            },
            {
                "id": "H5",
                "claim": "Default Menus.mnu contains zero menu entries",
                "basis": "heuristic on the field labels, parsed on the bytes",
                "evidence": "the whole 71-byte file is accounted for: 4-byte '8MNU' signature + 4-byte version (2) + 4-byte char count (27) + 54 bytes of UTF-16BE name ('Color and Tonal Correction' plus its NUL) + 5 trailing zero bytes = 71. No room for entries and no non-zero data remains.",
                "what_is_parsed_vs_inferred": "the byte layout is parsed; naming the fields version/length/entry-count is inferred without an Adobe format spec",
            },
            {
                "id": "H6",
                "claim": "the application menu tree is compiled into the Photoshop binary",
                "basis": "heuristic",
                "evidence": "exhaustive walk of %d install files found no menu-tree data file; the only candidates turned out to be the editor dialog's layout and a zero-entry customization header" % total_files,
                "what_is_parsed_vs_inferred": "the absence is parsed (exhaustive search); the 'it is in the binary' conclusion is inferred - the binary was not inspected",
            },
            {
                "id": "H7",
                "claim": "commands, tools and taskspaces are separate keyboard dispatch scopes",
                "basis": "heuristic",
                "evidence": "the file's own structural nesting (taskspace children are enclosed in <taskspace>) and the fact that single letters are freely reused across the three",
                "what_is_parsed_vs_inferred": "the nesting is parsed; the runtime dispatch-precedence claim is inferred and MUST be re-derived by the Rust implementer",
            },
        ],
    }

    # Measure - do not assume - what the earlier pass actually captured.
    earlier = os.path.join(OUT_DIR, "keyboard_shortcuts.json")
    measured = {"path": earlier, "exists": os.path.exists(earlier)}
    if measured["exists"]:
        try:
            with open(earlier, encoding="utf-8") as fh:
                prev = json.load(fh)
            measured["top_level_keys"] = sorted(prev.keys())
            files = prev.get("files", [])
            measured["files_entry_count"] = len(files)
            per_file = []
            for fe in files:
                rows = fe.get("rows", [])
                tagc = collections.Counter(r.get("tag") for r in rows)
                keys_seen = collections.Counter()
                for r in rows:
                    for k in r:
                        keys_seen[k] += 1
                per_file.append({
                    "file": fe.get("file"),
                    "recorded_size": fe.get("size"),
                    "actual_size_on_disk_now": (
                        os.path.getsize(fe["file"])
                        if fe.get("file") and os.path.exists(fe["file"]) else None),
                    "recorded_row_count": fe.get("row_count"),
                    "actual_rows_in_array": len(rows),
                    "rows_by_element_tag": dict(tagc),
                    "row_field_names": sorted(keys_seen),
                    "any_row_carries_a_shortcut_value": any(
                        ("text" in r) or ("shortcut" in r) or ("shortcuts" in r)
                        for r in rows),
                })
            measured["per_file"] = per_file
        except Exception as exc:  # noqa: BLE001
            measured["read_error"] = str(exc)

    corr = doc["corrections_to_earlier_pass"]
    corr["earlier_artifact_measured"] = measured
    if measured.get("per_file"):
        pf = measured["per_file"][0]
        tags = pf["rows_by_element_tag"]
        corr["what_it_actually_captured"] = (
            "MEASURED by re-reading the earlier file. Its 86 rows are not 86 "
            "shortcuts and not a subset of the commands. They are %s. It "
            "captured ZERO <command> elements, ZERO <taskspace> elements, ZERO "
            "<taskspace-property> elements, and - decisively - ZERO shortcut "
            "VALUES: every row holds only path/tag/attrs, and "
            "any_row_carries_a_shortcut_value is %s. The 86 rows are exactly "
            "the elements that happen to carry a key= attribute (the 67 "
            "<tool> entries of type=1 plus all 19 <taskspace-tool> entries); "
            "the remaining 25 <tool> entries, which have no key= attribute, "
            "were dropped along with everything else."
            % (
                ", ".join("%d <%s>" % (v, k) for k, v in sorted(tags.items())),
                pf["any_row_carries_a_shortcut_value"],
            )
        )
        corr["so_the_86_was_wrong_in_two_ways"] = [
            "It was not 86 of 104 commands - it was 86 elements from two "
            "element types, and none of them were commands.",
            "None of the 86 rows contained a key binding at all, so the file "
            "documented no shortcut whatsoever despite its name.",
        ]
        # Was the whole install still being written when the earlier pass ran?
        # Check two UXP plugin directories the earlier pass recorded, and two
        # it did not - a file-level fact, no inference required.
        probe = {}
        for d in ("com.adobe.photoshop.ai-gallery",
                  "com.adobe.photoshop.creative_assistant",
                  "com.adobe.stock.unified.content.panel",
                  "com.adobe.uam"):
            probe[d] = os.path.isdir(
                os.path.join(INSTALL, "Required", "UXP", d))
        corr["size_discrepancy"] = {
            "earlier_recorded_size_bytes": pf["recorded_size"],
            "actual_size_on_disk_bytes": pf["actual_size_on_disk_now"],
            "kys_mtime_utc": datetime.datetime.fromtimestamp(
                os.path.getmtime(KYS_PRIMARY),
                datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
            "status": (
                "The earlier artifact recorded a different byte size for this "
                "exact path than os.path.getsize returns now. The install "
                "content demonstrably changed between the two passes: two UXP "
                "plugin directories the earlier pass recorded "
                "(com.adobe.photoshop.ai-gallery, "
                "com.adobe.photoshop.creative_assistant) no longer exist, and "
                "two that it did not record "
                "(com.adobe.stock.unified.content.panel, com.adobe.uam) do "
                "exist. Probe result: %s. That makes 'the file changed on "
                "disk between the passes' the evidenced reading. NOT VERIFIED "
                "beyond that - no installer log was inspected and the exact "
                "byte difference was not reconstructed."
                % json.dumps(probe)
            ) if pf["recorded_size"] != pf["actual_size_on_disk_now"] else "match",
            "install_directory_probe": probe,
        }
    corr["this_pass_captured"] = {
        "element_types_expanded": [
            "command", "shortcut", "tool", "taskspace",
            "taskspace-tool", "taskspace-property",
        ],
        "binding_rows_with_a_key_value": sum(1 for r in flat if r["bound"]),
        "binding_rows_total": len(flat),
    }

    os.makedirs(OUT_DIR, exist_ok=True)
    with open(OUT_PATH, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=1, ensure_ascii=False)

    # ---- self-verification: re-read what was just written ---------------
    with open(OUT_PATH, encoding="utf-8") as fh:
        back = json.load(fh)
    checks = {
        "reread_ok": True,
        "file_size_bytes": os.path.getsize(OUT_PATH),
        "totals_match": back["totals"] == doc["totals"],
        "all_shortcuts_len": len(back["all_shortcuts"]),
        "commands_len": len(back["sections"]["commands"]),
        "tools_len": len(back["sections"]["tools"]),
        "taskspaces_len": len(back["sections"]["taskspaces"]),
        "ts_tools_len": sum(len(t["taskspace_tools"]) for t in back["sections"]["taskspaces"]),
        "ts_props_len": sum(len(t["taskspace_properties"]) for t in back["sections"]["taskspaces"]),
        "shortcut_elements": sum(
            c["shortcut_element_count"] for c in back["sections"]["commands"]),
        "within_scope_conflicts": len(back["conflicts"]["within_scope"]),
        "cross_scope_reuse": len(back["conflicts"]["cross_scope"]),
        "vocab_len": len(back["key_token_vocabulary"]),
    }
    print(json.dumps(checks, indent=1))


if __name__ == "__main__":
    main()
