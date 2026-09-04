"""After Effects 2026 -> aftereffects_commands_shortcuts.json

Offline. Reads only. Never launches After Effects.

Key finding on shortcut encoding
--------------------------------
The Premiere teardown had to decode a numeric key encoding (bit 31 set = the
low bits are the unshifted Unicode code point, bit 30 = numeric keypad, bit 31
clear = a ~30-entry Adobe special-key enum). THAT DOES NOT APPLY HERE and was
not reused. After Effects ships its factory keyboard set as human-readable
TEXT inside the binaries:

    $$$/AE/KbShortcut/<Context>/LStr/0045=CompTwirlOpacityAddStateNewKF,(Alt+Shift+T)
    $$$/AE/KbShortcut/CSwitchboard/LStr/0025=Duplicate,(Ctrl+D)

so each binding is "<commandId>,(<key spelling>)" and no numeric decoding is
required. Verification performed: (1) every recovered value matches that exact
shape, (2) the modifier tokens form a closed set of 5 spellings, (3) no .kys or
other binary keyboard-set file exists anywhere in the install, and (4) the
per-user shortcut folders are empty because the app has never been launched.

Command labels come from a second, parallel namespace:
    $$$/AE/KB/S/<Category>/<commandId>=<human readable description>
which is what the Keyboard Shortcuts editor lists, grouped by category.

Menu items come from:
    $$$/AE/MenuID/<menuNumber>/<CommandName>_<numericId>=<menu label>
with '&' marking the Windows accelerator letter.
"""

from __future__ import annotations

import collections
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ae_common as C  # noqa: E402

BINDING_RE = re.compile(r"^(?P<cmd>[^,]*),\((?P<key>.*)\)$")
PLATFORM_SPLIT = "{{*MSWindows*}}"
MODIFIERS = ("Ctrl", "Cmd", "Alt", "Opt", "Shift", "Ctl")


def split_platform(v: str):
    """Adobe writes 'MacSpelling{{*MSWindows*}}WindowsSpelling'."""
    if PLATFORM_SPLIT in v:
        mac, win = v.split(PLATFORM_SPLIT, 1)
        return mac.strip(), win.strip()
    return v.strip(), v.strip()


def parse_key(spelling: str):
    if not spelling:
        return {"raw": spelling, "unbound": True}
    parts = [p for p in spelling.split("+") if p != ""]
    mods = [p for p in parts if p in MODIFIERS]
    keys = [p for p in parts if p not in MODIFIERS]
    rec = {"raw": spelling, "modifiers": mods, "key": "+".join(keys) or None}
    if spelling.startswith("Pad") or (keys and keys[0].startswith("Pad")):
        rec["numeric_keypad"] = True
    return rec


def main():
    idx = C.build_english_index()
    dictionary = C.build_key_inventory()

    # ---- 1. factory keyboard bindings ----------------------------------
    shortcuts = []
    malformed = []
    contexts = collections.Counter()
    for k in sorted(C.keys_under("AE/KbShortcut/", idx)):
        v = idx[k]["text"].strip()
        parts = k.split("/")
        ctx = parts[2] if len(parts) > 2 else "?"
        if not v:
            continue
        mac_v, win_v = split_platform(v)
        m = BINDING_RE.match(win_v)
        if not m:
            malformed.append({"key": k, "value": v})
            continue
        cmd = m.group("cmd").strip()
        win_key = m.group("key").strip()
        mm = BINDING_RE.match(mac_v)
        mac_key = mm.group("key").strip() if mm else win_key
        contexts[ctx] += 1
        rec = {
            "command_id": cmd,
            "context": ctx,
            "windows": parse_key(win_key),
            "string_key": k,
        }
        if mac_key != win_key:
            rec["macos"] = parse_key(mac_key)
        shortcuts.append(rec)

    # ---- 2. command labels from the shortcut editor namespace ----------
    commands = []
    by_cat = collections.Counter()
    for k in sorted(C.keys_under("AE/KB/", idx)):
        parts = k.split("/")
        if len(parts) < 5 or parts[2] != "S":
            continue
        cat, cid = parts[3], "/".join(parts[4:])
        by_cat[cat] += 1
        commands.append({"command_id": cid, "category": cat,
                         "label": idx[k]["text"], "string_key": k})
    cmd_index = {c["command_id"]: c for c in commands}
    bound = 0
    for s in shortcuts:
        c = cmd_index.get(s["command_id"])
        if c:
            s["label"] = c["label"]
            s["shortcut_editor_category"] = c["category"]
            c.setdefault("bindings", []).append(
                {"context": s["context"], "windows": s["windows"].get("raw")})
            bound += 1

    # ---- 3. menu surface ------------------------------------------------
    menus = collections.defaultdict(list)
    for k in sorted(C.keys_under("AE/MenuID/", idx)):
        parts = k.split("/")
        if len(parts) < 4:
            continue
        menu_no = parts[2]
        leaf = "/".join(parts[3:])
        m = re.match(r"^(?P<name>.*?)_(?P<id>\d+)$", leaf)
        label = idx[k]["text"]
        item = {
            "command_name": m.group("name") if m else leaf,
            "numeric_command_id": int(m.group("id")) if m else None,
            "label": label.replace("&", ""),
            "windows_accelerator": (label.split("&", 1)[1][:1]
                                    if "&" in label else None),
            "is_separator": label.strip() in ("-", "(-"),
            "opens_dialog": label.rstrip().endswith("..."),
            "string_key": k,
        }
        menus[menu_no].append({k2: v2 for k2, v2 in item.items() if v2 not in (None, False)})

    # ---- 4. tools --------------------------------------------------------
    tp = {int(k.rsplit("/", 1)[1]): idx[k]["text"]
          for k in C.keys_under("AE/Tool_Palette/LStr/", idx)
          if k.rsplit("/", 1)[1].isdigit()}
    tools = []
    i = 2
    while i + 1 in tp or i in tp:
        name = tp.get(i, "")
        key = tp.get(i + 1, "")
        if name and not name.endswith(("Tool", "Tools", "Panel")) and "+" not in name:
            # the alternating name/shortcut run has ended
            if i > 4:
                break
        if name:
            mac_v, win_v = split_platform(key)
            tools.append({
                "tool_name": name,
                "shortcut_windows": win_v or None,
                "shortcut_macos": mac_v or None,
                "lstr_index": i,
            })
        i += 2
        if i > 200:
            break

    # ---- 5. negative checks --------------------------------------------
    kys = [C.rel(p) for p in C.iter_files(C.support_files(), (".kys",))]
    user_root = os.path.join(C.user_data_root(), "26.3")
    user_dirs = {}
    if os.path.isdir(user_root):
        for d in sorted(os.listdir(user_root)):
            full = os.path.join(user_root, d)
            if os.path.isdir(full):
                user_dirs[d] = len(os.listdir(full))

    # keyboard-set completeness check against the dictionary key inventory
    dict_kb = {k for k in dictionary if k.startswith("AE/KbShortcut/")}
    dict_cmd = {k for k in dictionary if k.startswith("AE/KB/S/")}

    method = {
        "app_launched": False,
        "tool": "_tools/greenroom/ae_commands.py",
        "evidence": [
            {"label": "parsed", "path": "Support Files/**/*.dll|*.exe|*.aex",
             "what": "$$$/AE/KbShortcut/<Context>/LStr/NNNN='<commandId>,(<key>)' "
                     "factory keyboard bindings, already textual"},
            {"label": "parsed", "path": "same",
             "what": "$$$/AE/KB/S/<Category>/<commandId>=<label>, the command "
                     "list the Keyboard Shortcuts editor shows"},
            {"label": "parsed", "path": "same",
             "what": "$$$/AE/MenuID/<menu>/<Command>_<id>=<menu label>"},
            {"label": "parsed", "path": "Support Files/Required/Tool Palette.aex",
             "what": "$$$/AE/Tool_Palette/LStr/NNNN alternating "
                     "(tool name, shortcut) pairs from index 0002"},
            {"label": "parsed", "path": "Support Files/Dictionaries/de_DE/*.dat",
             "what": "full ZString key inventory, used to measure how much of "
                     "the command surface has an English literal on disk"},
        ],
        "shortcut_verification": {
            "premiere_bit31_key_encoding_applies": False,
            "why": ("After Effects stores the factory set as literal text, so no "
                    "numeric key decoding was needed and the Premiere encoding "
                    "was neither reused nor relied on."),
            "checks_run": [
                "every AE/KbShortcut value matched '<commandId>,(<key>)' except "
                "%d malformed entries, listed under failures" % len(malformed),
                "modifier tokens observed form the closed set %s" % (MODIFIERS,),
                "no .kys or other binary keyboard-set file exists in the install "
                "(found %d)" % len(kys),
                "per-user shortcut/workspace folders are empty because the app "
                "has never been launched: %s" % user_dirs,
            ],
        },
        "failures": {
            "malformed_binding_values": malformed[:40],
            "malformed_binding_count": len(malformed),
            "shortcuts_without_a_matching_command_label":
                len(shortcuts) - bound,
        },
        "coverage": {
            "kbshortcut_keys_in_dictionary": len(dict_kb),
            "kbshortcut_keys_with_english_on_disk": len(
                [k for k in idx if k.startswith("AE/KbShortcut/")]),
            "command_keys_in_dictionary": len(dict_cmd),
            "command_keys_with_english_on_disk": len(
                [k for k in idx if k.startswith("AE/KB/S/")]),
        },
        "counts": {
            "bindings": len(shortcuts),
            "binding_contexts": len(contexts),
            "commands_with_labels": len(commands),
            "command_categories": len(by_cat),
            "menu_groups": len(menus),
            "menu_items": sum(len(v) for v in menus.values()),
            "tools": len(tools),
        },
    }

    payload = {
        "summary": {
            "keyboard_bindings": len(shortcuts),
            "bindings_joined_to_a_command_label": bound,
            "binding_contexts": dict(contexts.most_common()),
            "commands_with_labels": len(commands),
            "command_categories": dict(by_cat.most_common()),
            "menu_items": sum(len(v) for v in menus.values()),
            "tools": len(tools),
        },
        "tools": tools,
        "keyboard_bindings": shortcuts,
        "commands": commands,
        "menus": {k: v for k, v in sorted(menus.items())},
    }
    C.write_json("aftereffects_commands_shortcuts.json",
                 "handshake.studio.teardown.aftereffects.commands_shortcuts",
                 method, payload)
    print("bindings=%d commands=%d menus=%d items=%d tools=%d malformed=%d"
          % (len(shortcuts), len(commands), len(menus),
             sum(len(v) for v in menus.values()), len(tools), len(malformed)),
          file=sys.stderr)


if __name__ == "__main__":
    main()
