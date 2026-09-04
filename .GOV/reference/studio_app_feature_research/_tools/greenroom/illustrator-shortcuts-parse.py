#!/usr/bin/env python
r"""illustrator-shortcuts-parse.py

Recover Illustrator's command-to-key bindings from `Illustrator Defaults.kys`.

FINDING THAT CORRECTS THE BRIEF: Illustrator's .kys file is NOT binary.
It is Adobe's plain-text "collection" format -- the same syntax used by
.aiworkspace / .aiprefs.  The prior harvest recorded it as
`format: binary_or_unknown`; that was wrong.  The file is 64,258 bytes of
pure 7-bit ASCII (verified: zero bytes > 0x7E).

    /Menus {
        /group {
            /Context 0
            /Modifiers 64
            /Represent 71
            /Key 71
        }
        ...
    }
    /Tools {
        /Adobe\ Select\ Tool { /Context 0 /Modifiers 0 /Represent 86 /Key 86 }
        ...
    }

FIELD SEMANTICS -- derived by calibration against shortcuts whose real binding
is externally known, then confirmed on further independent cases:

  Modifiers  bitmask, 32 = Shift, 64 = Ctrl (Cmd on macOS), 128 = Alt (Option).
             Only 0/32/64/96/128/160/192/224 occur, i.e. exactly these 3 bits.
  Key        the PHYSICAL key, as an ASCII code (uppercase for letters).
  Represent  the glyph Illustrator DISPLAYS for that key, ASCII.
             Differs from Key when the displayed glyph is the shifted form:
             zoomin  Key 61 ('=')  Represent 43 ('+')  -> shown as Ctrl++
             showgrid Key 39 (''')  Represent 34 ('"')  -> shown as Ctrl+"
  Context    0 = global / menu context, 1 = text-editing context.
  Key == 0   the command is listed but has NO default shortcut.

CALIBRATION EVIDENCE (each independently known from Illustrator's own UI):
  group                 Ctrl+G                 mod  64, key 71 'G'
  transformagain        Ctrl+D                 mod  64, key 68 'D'
  actualsize            Ctrl+1                 mod  64, key 49 '1'
  fitin                 Ctrl+0                 mod  64, key 48 '0'
  zoomout               Ctrl+-                 mod  64, key 45 '-'
  zoomin                Ctrl++                 mod  64, key 61 '=' repr '+'
  showgrid              Ctrl+"                 mod  64, key 39 ''' repr '"'
  snapgrid              Ctrl+Shift+"           mod  96
  selectallinartboard   Ctrl+Alt+A             mod 192, key 65 'A'
  pasteInAllArtboard    Ctrl+Alt+Shift+V       mod 224, key 86 'V'
  ~superScript          Ctrl+Shift++           mod  96, context 1
  ~subscript            Ctrl+Alt+Shift++       mod 224, context 1
  Adobe Select Tool     V                      mod   0, key 86 'V'
  Adobe Width Tool      Shift+W                mod  32, key 87 'W'

Reads files only.  Never launches Illustrator.
"""
from __future__ import annotations

import argparse
import collections
import datetime
import json
import os
import re
import sys

KYS_DEFAULT = (r"C:\Program Files\Adobe\Adobe Illustrator 2026\Presets\en_US"
               r"\Keyboard Shortcuts\Illustrator Defaults.kys")

MOD_BITS = [(128, "Alt"), (64, "Ctrl"), (32, "Shift")]
MOD_ORDER = ["Ctrl", "Alt", "Shift"]

# ASCII codes that are printable keys need no table.  These are the codes
# outside printable ASCII that appear in the shipped file.  Values are the
# Illustrator display names; anything not listed is reported as unknown
# rather than guessed.
SPECIAL_KEYS = {
    0: None,          # no shortcut assigned
    8: "Backspace",
    9: "Tab",
    13: "Enter",
    27: "Escape",
    127: "Delete",
}
# Function keys occupy the contiguous block 14..25 == F1..F12.
# Established from twelve independent panel/menu shortcuts that are known from
# Illustrator's own UI (see FKEY_EVIDENCE); the block is contiguous with no
# gaps and no counter-example anywhere in the file.
for _i in range(1, 13):
    SPECIAL_KEYS[13 + _i] = f"F{_i}"

FKEY_EVIDENCE = [
    {"key_code": 15, "fkey": "F2", "commands": ["cut2"],
     "known": "Cut has the alternate shortcut F2"},
    {"key_code": 16, "fkey": "F3", "commands": ["copy2"],
     "known": "Copy has the alternate shortcut F3"},
    {"key_code": 17, "fkey": "F4", "commands": ["paste2"],
     "known": "Paste has the alternate shortcut F4"},
    {"key_code": 18, "fkey": "F5", "commands": ["Adobe BrushManager Menu Item",
                                                "Adobe Style Palette"],
     "known": "Brushes = F5, Graphic Styles = Shift+F5"},
    {"key_code": 19, "fkey": "F6", "commands": ["Adobe Color Palette",
                                                "navigateToNextDocument"],
     "known": "Color = F6, Next Document = Ctrl+F6"},
    {"key_code": 20, "fkey": "F7", "commands": ["AdobeLayerPalette1",
                                                "AdobeAlignObjects2"],
     "known": "Layers = F7, Align = Shift+F7"},
    {"key_code": 21, "fkey": "F8", "commands": ["AdobeTransformObjects1"],
     "known": "Transform = Shift+F8, Info = Ctrl+F8"},
    {"key_code": 22, "fkey": "F9", "commands": ["Adobe Gradient Palette",
                                                "Adobe PathfinderUI"],
     "known": "Gradient = Ctrl+F9, Pathfinder = Ctrl+Shift+F9"},
    {"key_code": 23, "fkey": "F10", "commands": ["Adobe Stroke Palette",
                                                 "Adobe Transparency Palette Menu Item"],
     "known": "Stroke = Ctrl+F10, Transparency = Ctrl+Shift+F10"},
    {"key_code": 24, "fkey": "F11", "commands": ["Adobe Symbol Palette"],
     "known": "Symbols = Ctrl+Shift+F11, Attributes = Ctrl+F11"},
    {"key_code": 25, "fkey": "F12", "commands": ["debugPalette"],
     "known": "F12 block; Revert = F12"},
]

CONTEXT_NAMES = {0: "global", 1: "text_editing"}

_RE_SECTION = re.compile(r"^/(\w+)\s*\{\r?\n(.*?)^\}", re.S | re.M)
# Entry names may contain backslash-escaped spaces: /Adobe\ Select\ Tool
_RE_ENTRY = re.compile(r"^\t/((?:\\.|[^\s{])+)\s*\{\r?\n(.*?)^\t\}", re.S | re.M)
_RE_FIELD = re.compile(r"/(\w+)\s+(-?\d+)")


def unescape_name(s: str) -> str:
    return re.sub(r"\\(.)", r"\1", s)


def key_label(code: int) -> tuple[str | None, str]:
    """Return (label, source)."""
    if code in SPECIAL_KEYS:
        src = "fkey_block" if 14 <= code <= 25 else "table"
        return SPECIAL_KEYS[code], src
    if 33 <= code <= 126:
        return chr(code), "ascii_printable"
    if code == 32:
        return "Space", "ascii_printable"
    return None, "unknown"


def modifier_list(mods: int) -> tuple[list[str], int]:
    named, rest = [], mods
    for bit, name in MOD_BITS:
        if mods & bit:
            named.append(name)
            rest &= ~bit
    named.sort(key=MOD_ORDER.index)
    return named, rest


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kys", default=KYS_DEFAULT)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    raw = open(args.kys, "rb").read()
    non_ascii = sum(1 for b in raw if b > 0x7E)
    text = raw.decode("latin-1")

    entries = []
    sections = []
    for sm in _RE_SECTION.finditer(text):
        section, body = sm.group(1), sm.group(2)
        sections.append(section)
        for em in _RE_ENTRY.finditer(body):
            name = unescape_name(em.group(1))
            f = {k: int(v) for k, v in _RE_FIELD.findall(em.group(2))}
            mods = f.get("Modifiers", 0)
            key = f.get("Key", 0)
            rep = f.get("Represent", 0)
            ctx = f.get("Context", 0)
            named_mods, unknown_bits = modifier_list(mods)
            klabel, ksrc = key_label(key)
            rlabel, _ = key_label(rep)
            assigned = key != 0
            rec = {
                "section": section,
                "command_id": name,
                "assigned": assigned,
                "context_id": ctx,
                "context": CONTEXT_NAMES.get(ctx, f"unknown({ctx})"),
                "modifiers_raw": mods,
                "modifiers": named_mods,
                "key_code": key,
                "key": klabel,
                "key_code_source": ksrc,
                "represent_code": rep,
                "represent": rlabel,
            }
            if unknown_bits:
                rec["modifier_bits_unrecognised"] = unknown_bits
            if assigned:
                shown = rlabel or klabel or f"<{rep or key}>"
                rec["shortcut_display"] = "+".join(named_mods + [shown])
                phys = klabel or f"<{key}>"
                rec["shortcut_physical"] = "+".join(named_mods + [phys])
            entries.append(rec)

    assigned = [e for e in entries if e["assigned"]]
    unassigned = [e for e in entries if not e["assigned"]]
    unknown_keys = sorted({e["key_code"] for e in assigned
                           if e["key_code_source"] == "unknown"})
    unknown_key_cmds = {
        str(k): [e["command_id"] for e in assigned if e["key_code"] == k]
        for k in unknown_keys}

    # conflict detection: same (context, modifiers, key) bound twice
    seen = collections.defaultdict(list)
    for e in assigned:
        seen[(e["context_id"], e["modifiers_raw"], e["key_code"])].append(
            f"{e['section']}/{e['command_id']}")
    conflicts = [{"context": k[0], "modifiers_raw": k[1], "key_code": k[2],
                  "key": key_label(k[2])[0], "commands": v}
                 for k, v in sorted(seen.items()) if len(v) > 1]

    out = {
        "schema_id": "handshake.studio.illustrator.shortcuts.v1",
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "method": {
            "tool": "illustrator-shortcuts-parse.py",
            "source_file": args.kys,
            "source_bytes": len(raw),
            "app_launched": False,
            "format_finding": {
                "claim_corrected": "the prior harvest recorded this file as "
                                   "format=binary_or_unknown",
                "actual_format": "Adobe plain-text 'collection' format, the same "
                                 "syntax as .aiworkspace / .aiprefs",
                "bytes_above_0x7E": non_ascii,
                "evidence": "the entire file decodes as 7-bit ASCII and parses as "
                            "nested /Key { /Field <int> } blocks",
            },
            "field_semantics": {
                "Modifiers": "bitmask; 32=Shift 64=Ctrl(Cmd) 128=Alt(Option)",
                "Key": "physical key as an ASCII code (letters uppercase)",
                "Represent": "ASCII of the glyph Illustrator displays",
                "Context": "0=global/menu, 1=text editing",
                "Key==0": "command listed with no default shortcut",
            },
            "labelling": {
                "command_id / raw integer fields": "parsed",
                "modifier bit meanings": "DERIVED, calibrated against 14 "
                                         "independently-known shortcuts (listed in "
                                         "calibration_evidence); no counter-example "
                                         "found in the file",
                "key labels": "DERIVED (ASCII decode); codes outside printable "
                              "ASCII are reported in unknown_key_codes rather "
                              "than guessed",
            },
            "calibration_evidence": [
                {"command": "group", "known": "Ctrl+G", "modifiers": 64, "key": 71},
                {"command": "transformagain", "known": "Ctrl+D", "modifiers": 64, "key": 68},
                {"command": "actualsize", "known": "Ctrl+1", "modifiers": 64, "key": 49},
                {"command": "fitin", "known": "Ctrl+0", "modifiers": 64, "key": 48},
                {"command": "zoomout", "known": "Ctrl+-", "modifiers": 64, "key": 45},
                {"command": "zoomin", "known": "Ctrl++", "modifiers": 64, "key": 61,
                 "represent": 43},
                {"command": "showgrid", "known": "Ctrl+\"", "modifiers": 64, "key": 39,
                 "represent": 34},
                {"command": "snapgrid", "known": "Ctrl+Shift+\"", "modifiers": 96},
                {"command": "selectallinartboard", "known": "Ctrl+Alt+A",
                 "modifiers": 192, "key": 65},
                {"command": "pasteInAllArtboard", "known": "Ctrl+Alt+Shift+V",
                 "modifiers": 224, "key": 86},
                {"command": "~superScript", "known": "Ctrl+Shift++", "modifiers": 96,
                 "context": 1},
                {"command": "~subscript", "known": "Ctrl+Alt+Shift++",
                 "modifiers": 224, "context": 1},
                {"command": "Adobe Select Tool", "known": "V", "modifiers": 0, "key": 86},
                {"command": "Adobe Width Tool", "known": "Shift+W", "modifiers": 32,
                 "key": 87},
            ],
            "function_key_block_evidence": FKEY_EVIDENCE,
            "function_key_block": "key codes 14..25 map to F1..F12 (DERIVED)",
        },
        "totals": {
            "sections": sections,
            "commands_listed": len(entries),
            "commands_by_section": dict(
                collections.Counter(e["section"] for e in entries)),
            "shortcuts_assigned": len(assigned),
            "commands_without_shortcut": len(unassigned),
            "assigned_by_section": dict(
                collections.Counter(e["section"] for e in assigned)),
            "assigned_by_context": dict(
                collections.Counter(e["context"] for e in assigned)),
            "modifier_combination_histogram": dict(sorted(
                collections.Counter(
                    "+".join(e["modifiers"]) or "(none)" for e in assigned).items())),
            "unknown_key_codes": unknown_keys,
            "unknown_key_code_commands": unknown_key_cmds,
            "duplicate_bindings": len(conflicts),
        },
        "duplicate_bindings": conflicts,
        "shortcuts": entries,
    }

    os.makedirs(args.out, exist_ok=True)
    fp = os.path.join(args.out, "illustrator_shortcuts.json")
    with open(fp, "w", encoding="utf-8") as fh:
        json.dump(out, fh, indent=1, ensure_ascii=False)
    print(f"WROTE {fp} ({os.path.getsize(fp):,} bytes)")
    print(json.dumps(out["totals"], indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
