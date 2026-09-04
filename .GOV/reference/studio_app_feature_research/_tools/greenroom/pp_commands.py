"""pp_commands.py -- Premiere's command surface, menus, shortcuts, control surfaces.

Streams:
  K1  Keyboard Shortcuts/<lang>/*.kys   every shipped shortcut set. A .kys is
      PremiereData: <shortcuts Version=6><platform>windows</platform>
      <mode.Edit><context.timeline><item.N><commandname/>
      <modifier.shift/><modifier.alt/><modifier.ctrl/><virtualkey/>.
  K2  Settings/EveScripts/NewMenus/*.eve and Menus/*.eve   the menu trees. Each
      item carries a localizable label and, in NewMenus, the command it fires
      as idName:'cmd.*'. This is the command-to-label mapping.
  K3  executable command literals       the contiguous table of "cmd.*"
      NUL-terminated literals inside Adobe Premiere Pro.exe: the full command
      surface, including commands with no default binding and no menu item.
  K4  xml/Audition.xml, MackieConfig, ControlSurface*   hardware control-surface
      bindings (EuCon AppSet and Mackie configurations).

KEY ENCODING (see key_encoding in the output for the verification record)
  <virtualkey> is a 32-bit value despite the field name; it is NOT a Windows
  VK code.
    bit 31 set  -> the low byte is the ASCII code of the key's unshifted
                   character: 0x41..0x5A A-Z, 0x30..0x39 0-9, and the
                   punctuation keys ' , - . / ; = [ \\ ] `
    bit 31 clear-> the low byte indexes a small Adobe special-key enum
                   (Space, Backspace, Tab, Return, Delete, Home, End,
                   Page Up/Down, the four arrows, F1).
  Verified by binding each code back to commands whose own names state the key
  they imply -- cmd.timeline.slide.left.one carries 0x8000002C and ',' is
  Premiere's Slide Clip Left; cmd.timeline.nudge.up carries 0x0000002C and
  Nudge Up is the Up arrow; cmd.project.move.home carries 0x00000024.
  The discriminator against a Windows VK reading is the punctuation: Windows
  VK_OEM_COMMA is 0xBC, not 0x2C, so the high-bit form cannot be VK codes.
"""
import collections
import os
import re
import sys
import traceback
import xml.etree.ElementTree as ET

HERE = os.path.dirname(os.path.abspath(__file__))
if HERE not in sys.path:
    sys.path.insert(0, HERE)

import pp_common as C
import dw_eve

SCRATCH = os.environ.get("PP_SCRATCH") or os.path.join(HERE, "_cache")

CHAR_KEY_FLAG = 0x80000000          # bit 31: the low bits are a character
NUMPAD_FLAG = 0x40000000            # bit 30, always with bit 31: numeric keypad

SPECIAL_KEYS = {
    0x01: "Space",
    0x02: "Backspace",
    0x03: "Tab",
    0x04: "Return / Enter",
    0x05: "Numeric keypad Enter",
    0x07: "F1", 0x08: "F2", 0x09: "F3", 0x0A: "F4", 0x0B: "F5", 0x0C: "F6",
    0x0D: "F7", 0x0E: "F8", 0x0F: "F9", 0x10: "F10", 0x11: "F11", 0x12: "F12",
    0x23: "Delete (forward delete)",
    0x24: "Home",
    0x25: "End",
    0x26: "Page Up",
    0x27: "Page Down",
    0x2A: "Left Arrow",
    0x2B: "Right Arrow",
    0x2C: "Up Arrow",
    0x2D: "Down Arrow",
    0x2F: "Alt",
    0x30: "Windows / Command",
    0x31: "Ctrl",
    0x32: "Shift",
    0x33: "Caps Lock",
    0xFFFF: "(no key -- physical layout spacer)",
}

# Two independent evidence streams back the table above.
#  (a) the command each code is bound to in the shipped default set
#  (b) the physical position each code occupies in the shipped keyboard
#      visualiser layouts, ksvlayout/xml/*.ksvlayout
SPECIAL_KEY_EVIDENCE = {
    0x01: "cmd.transport.toggleplay is Space | ksvlayout: the wide key in the bottom row",
    0x02: "cmd.edit.clear / cmd.graphics.clear | ksvlayout: end of the number row",
    0x03: "cmd.project.nextcolumnfield / previouscolumnfield | ksvlayout: start of the QWERTY row",
    0x04: "cmd.sequence.preview, cmd.project.nextrowfield | ksvlayout: end of the home row",
    0x05: "ksvlayout: sits immediately after the numeric keypad 1/2/3 cluster in every shipped layout",
    0x07: "cmd.help.contents is F1 | ksvlayout: 0x07..0x12 form the twelve-key function row in order",
    0x23: "cmd.edit.rippledelete, cmd.project.deletewithoptions | ksvlayout: navigation cluster",
    0x24: "cmd.transport.sequence.start, cmd.project.move.home | ksvlayout: navigation cluster",
    0x25: "cmd.transport.sequence.end, cmd.project.move.end | ksvlayout: navigation cluster",
    0x26: "cmd.timeline.show.previous.screen, cmd.project.move.pageup | ksvlayout: navigation cluster",
    0x27: "cmd.timeline.show.next.screen, cmd.project.move.pagedown | ksvlayout: navigation cluster",
    0x2A: "cmd.timeline.nudge.left.one, cmd.capture.step.back | ksvlayout: arrow cluster, left",
    0x2B: "cmd.timeline.nudge.right.one, cmd.capture.step.forward | ksvlayout: arrow cluster, right",
    0x2C: "cmd.timeline.nudge.up | ksvlayout: arrow cluster, above the arrow row",
    0x2D: "cmd.timeline.nudge.down | ksvlayout: arrow cluster, bottom row centre",
    0x2F: "ksvlayout: third key of the bottom row, mirrored on both sides of Space",
    0x30: "ksvlayout: second key of the bottom row, mirrored on both sides of Space",
    0x31: "ksvlayout: outermost key of the bottom row, mirrored on both sides",
    0x32: "ksvlayout: first and last key of the ZXCV row",
    0x33: "ksvlayout: first key of the ASDF row",
    0xFFFF: "ksvlayout: carries width_multiplier but no key; a physical gap",
}


def decode_key(value):
    try:
        v = int(value)
    except (TypeError, ValueError):
        return {"raw": value, "key": None, "encoding": "unrecognised"}
    if v < 0:
        v &= 0xFFFFFFFF
    low = v & 0xFFFFFF
    if v & CHAR_KEY_FLAG:
        numpad = bool(v & NUMPAD_FLAG)
        try:
            ch = chr(low) if low >= 0x20 else None
        except ValueError:
            ch = None
        return {"raw": v, "key": (("Numpad " + ch) if (numpad and ch) else ch),
                "key_code": low, "numeric_keypad": numpad,
                "encoding": ("unicode code point of the key's unshifted character"
                             + (", numeric keypad" if numpad else "")),
                "confidence": "parsed"}
    return {"raw": v, "key": SPECIAL_KEYS.get(low), "key_code": low,
            "numeric_keypad": False,
            "encoding": "Adobe special-key enum",
            "confidence": ("derived and cross-checked" if low in SPECIAL_KEYS
                           else "unresolved"),
            "evidence": SPECIAL_KEY_EVIDENCE.get(low)}


def parse_ksvlayout(path):
    """A shipped physical keyboard layout for the keyboard visualiser."""
    with open(path, "rb") as fh:
        raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", fh.read())
    # the file has several sibling roots and no document element
    root = ET.fromstring(b"<ksv>" + raw.split(b"?>", 1)[-1] + b"</ksv>")

    def txt(tag):
        e = root.find(tag)
        return (e.text or "").strip() if e is not None else None

    key, disp = C.split_localized(txt("layout_displayname") or "")
    rows = []
    kb = root.find("layout_keyboard")
    if kb is not None:
        for r in kb:
            if C._strip_ns(r.tag) != "layout_row":
                continue
            keys = []
            for b in r:
                a = dict(b.attrib)
                code = a.get("button_code")
                d = decode_key(code)
                keys.append({
                    "button_code": int(code) if code and code.isdigit() else code,
                    "key": d.get("key"),
                    "key_encoding": d.get("encoding"),
                    "is_modifier": a.get("modifier_key") == "true",
                    "width_multiplier": a.get("width_multiplier"),
                    "height_multiplier": a.get("height_multiplier"),
                    "width_adjustment": a.get("width_adjustment"),
                    "height_adjustment": a.get("height_adjustment"),
                    "expanded_layout_only": a.get("expanded") == "true",
                })
            rows.append(keys)
    return {
        "layout_name": txt("layout_name"),
        "display_name": disp,
        "display_name_string_key": key,
        "layout_version": txt("layout_version"),
        "layout_margin": txt("layout_margin"),
        "rows": rows,
        "row_count": len(rows),
        "key_count": sum(len(r) for r in rows),
    }


def format_binding(shift, alt, ctrl, key):
    mods = []
    if ctrl:
        mods.append("Ctrl")
    if alt:
        mods.append("Alt")
    if shift:
        mods.append("Shift")
    return "+".join(mods + [key or "?"])


def parse_kys(path):
    with open(path, "rb") as fh:
        raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", fh.read())
    root = ET.fromstring(raw)
    sc = root.find("shortcuts")
    if sc is None:
        raise ValueError("no <shortcuts> element")
    platform = None
    pe = sc.find("platform")
    if pe is not None:
        platform = (pe.text or "").strip()
    version = sc.get("Version")

    bindings = []
    modes = []

    def read_context(ctx_el, mode_name):
        ctx = C._strip_ns(ctx_el.tag)
        ctx_name = ctx.split(".", 1)[1] if "." in ctx else ctx
        n = 0
        for item in ctx_el:
            tag = C._strip_ns(item.tag)
            if not tag.startswith("item."):
                continue
            f = C.flat_fields(item)
            key = decode_key(f.get("virtualkey"))
            shift = f.get("modifier.shift") == "true"
            alt = f.get("modifier.alt") == "true"
            ctrl = f.get("modifier.ctrl") == "true"
            bindings.append({
                "command": f.get("commandname"),
                "mode": mode_name,
                "context": ctx_name,
                "item_index": tag.split(".", 1)[1],
                "modifier_shift": shift,
                "modifier_alt": alt,
                "modifier_ctrl": ctrl,
                "key": key.get("key"),
                "key_code": key.get("key_code"),
                "key_encoding": key.get("encoding"),
                "key_confidence": key.get("confidence"),
                "raw_virtualkey": key.get("raw"),
                "binding": format_binding(shift, alt, ctrl, key.get("key")),
            })
            n += 1
        return ctx_name, n

    contexts = collections.Counter()
    for el in sc:
        tag = C._strip_ns(el.tag)
        if tag.startswith("mode."):
            mode_name = tag.split(".", 1)[1]
            modes.append(mode_name)
            for ctx_el in el:
                if C._strip_ns(ctx_el.tag).startswith("context."):
                    name, n = read_context(ctx_el, mode_name)
                    contexts[name] += n
        elif tag.startswith("context."):
            name, n = read_context(el, None)
            contexts[name] += n

    return {"platform": platform, "shortcuts_version": version,
            "modes": modes, "contexts": dict(contexts),
            "bindings": bindings, "binding_count": len(bindings)}


# ---------------------------------------------------------------------------
def walk_menu(nodes, path=(), out=None, depth=0):
    if out is None:
        out = []
    for nd in nodes:
        kind = nd["kind"]
        a = nd["args"]
        if kind == "separator":
            out.append({"kind": "separator", "menu_path": list(path)})
            continue
        if kind not in ("menu", "view"):
            continue
        key, label = C.split_localized(a.get("text"))
        rec = {
            "kind": "submenu" if nd["children"] else "command",
            "label": label,
            "label_string_key": key,
            "command": a.get("idName"),
            "menu_id": a.get("id"),
            "menu_path": list(path),
        }
        for extra in ("betafeature", "checkable", "enabled", "shortcut",
                      "hidden", "radio", "dynamic"):
            if extra in a:
                rec[extra] = a[extra]
        if kind == "view":
            rec["kind"] = "menu root"
        out.append(rec)
        if nd["children"]:
            walk_menu(nd["children"], path + (label or a.get("id") or "?",),
                      out, depth + 1)
    return out


def parse_menu_eve(path):
    with open(path, "r", encoding="utf-8", errors="replace") as fh:
        layouts = dw_eve.parse_eve(fh.read())
    items = []
    for lay in layouts:
        items.extend(walk_menu(lay["nodes"]))
    return items


# ---------------------------------------------------------------------------
CMD_RE = re.compile(rb"(?<![\x21-\x7e])(cmd\.[a-z0-9][A-Za-z0-9._]{2,80})\x00")


def harvest_commands(exe_path):
    with open(exe_path, "rb") as fh:
        blob = fh.read()
    out = set()
    for m in CMD_RE.finditer(blob):
        out.add(m.group(1).decode("ascii"))
    del blob
    return out


def parse_eucon_appset(path):
    with open(path, "rb") as fh:
        raw = re.sub(rb"[\x00-\x08\x0b\x0c\x0e-\x1f]", b"", fh.read())
    root = ET.fromstring(raw)
    wide = {}
    aw = root.find("AppSetWide")
    if aw is not None:
        wide = C.flat_fields(aw)
    binds = []
    for sec in root.iter("Section"):
        sec_id = sec.get("ID")
        for bank in sec.iter("Bank"):
            for el in bank.iter():
                tag = C._strip_ns(el.tag)
                if tag in ("Section", "Bank"):
                    continue
                a = dict(el.attrib)
                txt = (el.text or "").strip()
                if not a and not txt:
                    continue
                binds.append({"section": sec_id, "bank_id": bank.get("ID"),
                              "bank_size": bank.get("SIZE"),
                              "element": tag, "attributes": a,
                              "text": txt or None})
    return {"app_set_wide": wide, "bindings": binds,
            "binding_count": len(binds)}


def main(out_dir):
    R = C.PREMIERE_ROOT
    table = C.premiere_strings(SCRATCH)
    sources = []
    failures = []

    # ---- K1 shortcut sets
    sets = []
    ks_root = os.path.join(R, "Keyboard Shortcuts")
    for p in sorted(C.walk_files(ks_root, exts=(".kys",))):
        rel = C.rel(p)
        try:
            rec = parse_kys(p)
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "K1_kys", "path": rel, "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        rec["file"] = rel
        rec["language"] = rel.split("/")[1]
        rec["set_name"] = os.path.splitext(os.path.basename(p))[0]
        sets.append(rec)
    sources.append({"id": "K1_kys", "path": C.rel(ks_root),
                    "how": ("PremiereData parse of every shipped .kys; the "
                            "virtualkey field decoded per key_encoding below"),
                    "shortcut_set_files": len(sets),
                    "binding_rows": sum(s["binding_count"] for s in sets)})

    # ---- K2 menus
    menus = []
    for sub in ("NewMenus", "Menus"):
        d = os.path.join(R, "Settings", "EveScripts", sub)
        if not os.path.isdir(d):
            continue
        for p in sorted(C.walk_files(d, exts=(".eve",))):
            try:
                items = parse_menu_eve(p)
            except Exception as exc:                   # noqa: BLE001
                failures.append({"stage": "K2_menu", "path": C.rel(p),
                                 "error": repr(exc)})
                continue
            menus.append({
                "file": C.rel(p),
                "menu_family": sub,
                "menu_name": os.path.splitext(os.path.basename(p))[0],
                "item_count": len(items),
                "command_item_count": sum(1 for i in items if i.get("command")),
                "items": items,
            })
    sources.append({"id": "K2_menus",
                    "how": ("Adobe Eve grammar parse of the shipped menu trees; "
                            "each item yields its localizable label, its "
                            "hierarchy path and the cmd.* it fires"),
                    "menu_files": len(menus),
                    "menu_items": sum(m["item_count"] for m in menus)})

    # ---- K3 command surface out of the executable
    exe = os.path.join(R, "Adobe Premiere Pro.exe")
    try:
        all_commands = harvest_commands(exe)
    except Exception as exc:                           # noqa: BLE001
        all_commands = set()
        failures.append({"stage": "K3_commands", "error": repr(exc)})
    sources.append({"id": "K3_exe_commands", "path": C.rel(exe),
                    "how": ("NUL-terminated cmd.* literals matched by regex over "
                            "the executable's bytes; the file is never executed"),
                    "command_literals": len(all_commands)})

    # ---- K5 physical keyboard layouts (independent check on the key encoding)
    ksv = []
    ksv_dir = os.path.join(R, "ksvlayout")
    for p in sorted(C.walk_files(ksv_dir, exts=(".ksvlayout",))):
        try:
            rec = parse_ksvlayout(p)
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "K5_ksvlayout", "path": C.rel(p),
                             "error": repr(exc),
                             "traceback": traceback.format_exc()})
            continue
        rec["file"] = C.rel(p)
        ksv.append(rec)
    ksv_codes = collections.Counter()
    for L in ksv:
        for row in L["rows"]:
            for k in row:
                ksv_codes[k["button_code"]] += 1
    unresolved = sorted(c for c in ksv_codes
                        if isinstance(c, int) and not (c & CHAR_KEY_FLAG)
                        and (c & 0xFFFFFF) not in SPECIAL_KEYS)
    sources.append({"id": "K5_ksvlayout", "path": C.rel(ksv_dir),
                    "how": ("physical keyboard-visualiser layouts; each key's "
                            "button_code uses the same encoding as a shortcut "
                            "binding, so the physical position of a code is an "
                            "independent check on what the code means"),
                    "layouts_parsed": len(ksv),
                    "distinct_button_codes": len(ksv_codes),
                    "special_codes_unresolved": [hex(c) for c in unresolved]})

    # ---- K4 control surfaces
    control_surfaces = {}
    for stem in ("Audition", "MackieConfig", "MackieMidiConfig",
                 "ControlSurfaceConfig", "ControlSurfaceListEditor",
                 "ControlSurfaceButtonAssignmentDialog"):
        p = os.path.join(R, "xml", stem + ".xml")
        if not os.path.isfile(p):
            continue
        try:
            with open(p, "rb") as fh:
                head = fh.read(200)
            if b"<AppSet" in head:
                control_surfaces[stem] = {"file": C.rel(p), "kind": "EuCon AppSet",
                                          **parse_eucon_appset(p)}
            else:
                control_surfaces[stem] = {"file": C.rel(p),
                                          "kind": "dvaui prop.map UI archive",
                                          "prop_map_top_keys":
                                              sorted(C.parse_propmap(p))[:40]}
        except Exception as exc:                       # noqa: BLE001
            failures.append({"stage": "K4_control_surface", "path": C.rel(p),
                             "error": repr(exc)})
    sources.append({"id": "K4_control_surfaces",
                    "how": ("EuCon AppSet XML walk for Audition.xml; prop.map "
                            "walk for the dvaui configuration panels"),
                    "files": len(control_surfaces)})

    # =====================================================================
    # merge into one command surface
    # =====================================================================
    label_of = {}
    menu_path_of = {}
    for m in menus:
        for it in m["items"]:
            cmd = it.get("command")
            if not cmd:
                continue
            if it.get("label") and cmd not in label_of:
                label_of[cmd] = it["label"]
                menu_path_of[cmd] = {"menu_file": m["menu_name"],
                                     "menu_path": it["menu_path"]}

    bindings_of = collections.defaultdict(list)
    for s in sets:
        for b in s["bindings"]:
            if not b["command"]:
                continue
            bindings_of[b["command"]].append({
                "set": s["set_name"], "language": s["language"],
                "platform": s["platform"], "mode": b["mode"],
                "context": b["context"], "binding": b["binding"],
                "key": b["key"], "key_encoding": b["key_encoding"],
            })

    bound_cmds = set(bindings_of)
    menu_cmds = set(label_of)
    surface = []
    for cmd in sorted(all_commands | bound_cmds | menu_cmds):
        if C.looks_ai(cmd, label_of.get(cmd)):
            continue
        default_en = [b for b in bindings_of.get(cmd, [])
                      if b["language"] == "en"
                      and b["set"].startswith("Adobe Premiere Pro Defaults")]
        surface.append({
            "command": cmd,
            "label": label_of.get(cmd),
            "namespace": cmd.split(".")[1] if cmd.count(".") >= 1 else None,
            "menu_location": menu_path_of.get(cmd),
            "default_bindings_en": sorted({b["binding"] for b in default_en}),
            "binding_count_all_sets": len(bindings_of.get(cmd, [])),
            "bindings": bindings_of.get(cmd, []),
            "evidence": sorted(set(
                (["K3_exe_commands"] if cmd in all_commands else []) +
                (["K1_kys"] if cmd in bound_cmds else []) +
                (["K2_menus"] if cmd in menu_cmds else []))),
        })

    ns_counts = collections.Counter(c["namespace"] for c in surface)
    contexts = collections.Counter()
    for s in sets:
        for k, v in s["contexts"].items():
            contexts[k] += v

    # per-set summary without repeating every binding
    set_summary = []
    for s in sets:
        set_summary.append({
            "set_name": s["set_name"], "language": s["language"],
            "file": s["file"], "platform": s["platform"],
            "shortcuts_version": s["shortcuts_version"],
            "modes": s["modes"], "contexts": s["contexts"],
            "binding_count": s["binding_count"],
            "distinct_commands": len({b["command"] for b in s["bindings"]}),
        })

    payload = C.envelope(
        "handshake.studio.premiere.commands_shortcuts.v1",
        {
            "summary": ("Premiere's command surface: every cmd.* the executable "
                        "declares, its menu label and location where it has one, "
                        "and its keyboard binding in every shipped shortcut set, "
                        "with the key encoding decoded and the decode's evidence "
                        "recorded."),
            "streams": {
                "K1_kys": "shipped keyboard shortcut sets",
                "K2_menus": "menu trees with labels and command ids",
                "K3_exe_commands": "the full command literal table",
                "K4_control_surfaces": "EuCon and Mackie hardware bindings",
            },
            "confidence_legend": {
                "parsed": "read verbatim from a shipped file",
                "derived and cross-checked": ("a decoded key whose meaning is "
                                              "confirmed by the command it is "
                                              "bound to"),
                "heuristic": "a decode with no independent cross-check",
            },
            "known_gaps": [
                ("A cmd.* that appears only in the executable has no shipped "
                 "label; Premiere resolves those at runtime from panel code. "
                 "label is null rather than guessed."),
                ("The three shortcut sets ship per language directory; the "
                 "binding rows differ by keyboard layout, which is why every "
                 "binding row carries its own language."),
            ],
        },
        sources,
        {
            "extraction_summary": {
                "distinct_commands": len(surface),
                "commands_with_a_menu_label": sum(1 for c in surface if c["label"]),
                "commands_with_a_default_en_binding": sum(
                    1 for c in surface if c["default_bindings_en"]),
                "commands_only_in_the_executable": sum(
                    1 for c in surface if c["evidence"] == ["K3_exe_commands"]),
                "shortcut_set_files": len(sets),
                "binding_rows_all_sets": sum(s["binding_count"] for s in sets),
                "menu_files": len(menus),
                "menu_items": sum(m["item_count"] for m in menus),
                "shortcut_contexts": len(contexts),
                "control_surface_files": len(control_surfaces),
                "physical_keyboard_layouts": len(ksv),
                "count_semantics": ("distinct_commands counts commands, not "
                                    "files; binding_rows_all_sets counts rows "
                                    "across every language and every set, so it "
                                    "is far larger than the number of distinct "
                                    "default shortcuts"),
            },
            "key_encoding": {
                "field_name_note": ("the field is named <virtualkey> but does not "
                                    "hold a Windows virtual-key code"),
                "bit_31_set": {
                    "mask": "0x80000000",
                    "meaning": ("the low 24 bits are the Unicode code point of "
                                "the key's unshifted character"),
                    "confidence": "parsed",
                    "verification": (
                        "Two checks. (1) The punctuation keys land on ',' '.' "
                        "'/' ';' '=' '[' ']' '\\\\' '`' and match the commands "
                        "that use them: 0x8000002C is on "
                        "cmd.timeline.slide.left.one and ',' is Premiere's Slide "
                        "Clip Left. A Windows VK reading is excluded because "
                        "VK_OEM_COMMA is 0xBC, not 0x2C. (2) The shipped "
                        "keyboard-visualiser layouts use the same encoding for "
                        "physical keys, and the German layout's third row reads "
                        "Q W E R T Z U I O P in QWERTZ order while the Russian "
                        "layout carries Cyrillic code points above 0xFF -- which "
                        "is also why the field is a code point, not a byte."),
                },
                "bit_30_set_with_bit_31": {
                    "mask": "0xC0000000",
                    "meaning": "the same character, on the numeric keypad",
                    "confidence": "derived and cross-checked",
                    "verification": ("only ever seen on the keypad block of the "
                                     "shipped physical layouts: * + - . / 0-9 ="),
                },
                "bit_31_clear": {
                    "meaning": "the low bits index an Adobe special-key enum",
                    "confidence": "derived and cross-checked",
                    "values": {hex(k): v for k, v in sorted(SPECIAL_KEYS.items())},
                    "evidence": {hex(k): v for k, v in sorted(SPECIAL_KEY_EVIDENCE.items())},
                    "unresolved_codes_seen": [hex(c) for c in unresolved],
                },
                "modifiers": ("modifier.ctrl / modifier.alt / modifier.shift are "
                              "independent booleans; there is no Command/Meta "
                              "field in the windows platform sets"),
            },
            "shortcut_contexts": dict(contexts),
            "command_namespaces": dict(ns_counts.most_common()),
            "shortcut_sets": set_summary,
            "command_surface": surface,
            "menus": menus,
            "physical_keyboard_layouts": ksv,
            "control_surfaces": control_surfaces,
            "failures": failures,
        })

    path, size = C.write_json(out_dir, "premiere_commands_shortcuts.json", payload)
    print("wrote", path, size, "bytes")
    print("commands", len(surface), "labelled",
          sum(1 for c in surface if c["label"]),
          "sets", len(sets), "menus", len(menus), "failures", len(failures))
    return payload


if __name__ == "__main__":
    main(sys.argv[1])
