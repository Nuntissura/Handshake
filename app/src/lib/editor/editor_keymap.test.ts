// WP-KERNEL-009 / MT-170 — EditorKeyboardAndCommandPalette (keymap) tests.
//
// Proves keyboard bindings resolve deterministically to command ids / the
// palette-open action (no hidden context), Mod accepts ctrl OR meta, and every
// bound command id (except the palette token) exists in the command catalog.

import { describe, it, expect } from "vitest";
import {
  EDITOR_KEY_BINDINGS,
  PALETTE_OPEN_ACTION,
  chordFromEvent,
  resolveShortcut,
  bindingsForAction,
} from "./editor_keymap";
import { EDITOR_COMMAND_BY_ID } from "./editor_commands";

describe("editor keymap (MT-170)", () => {
  it("builds canonical chords from key events (ctrl or meta => Mod)", () => {
    expect(chordFromEvent({ key: "b", ctrlKey: true })).toBe("Mod-b");
    expect(chordFromEvent({ key: "b", metaKey: true })).toBe("Mod-b");
    expect(chordFromEvent({ key: "p", ctrlKey: true, shiftKey: true })).toBe("Mod-Shift-p");
    expect(chordFromEvent({ key: "c", ctrlKey: true, altKey: true })).toBe("Mod-Alt-c");
  });

  it("resolves bound shortcuts to command ids", () => {
    expect(resolveShortcut({ key: "b", ctrlKey: true })).toBe("format.bold");
    expect(resolveShortcut({ key: "k", metaKey: true })).toBe("link.wikilink");
    expect(resolveShortcut({ key: "c", ctrlKey: true, altKey: true })).toBe("code.insert");
  });

  it("resolves the command-palette open chords", () => {
    expect(resolveShortcut({ key: "p", ctrlKey: true })).toBe(PALETTE_OPEN_ACTION);
    expect(resolveShortcut({ key: "p", ctrlKey: true, shiftKey: true })).toBe(PALETTE_OPEN_ACTION);
  });

  it("returns null for unbound chords", () => {
    expect(resolveShortcut({ key: "q", ctrlKey: true })).toBeNull();
    expect(resolveShortcut({ key: "a" })).toBeNull();
  });

  it("every bound command id (non-palette) exists in the command catalog", () => {
    for (const binding of EDITOR_KEY_BINDINGS) {
      if (binding.action === PALETTE_OPEN_ACTION) continue;
      expect(EDITOR_COMMAND_BY_ID.has(binding.action)).toBe(true);
    }
  });

  it("lists bindings for an action", () => {
    expect(bindingsForAction(PALETTE_OPEN_ACTION).length).toBeGreaterThanOrEqual(1);
    expect(bindingsForAction("format.bold")[0].chord).toBe("Mod-b");
  });
});
