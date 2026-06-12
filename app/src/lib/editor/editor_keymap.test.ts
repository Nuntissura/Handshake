// WP-KERNEL-009 / MT-170 — EditorKeyboardAndCommandPalette (keymap) tests.
//
// Proves keyboard bindings resolve deterministically to command ids / the
// palette-open action (no hidden context), Mod accepts ctrl OR meta, and every
// bound command id (except the palette token) exists in the command catalog.
// Iteration-3 additions: IME composition guard (L15), save action (L16), and
// the code-block chord-containment classification (H3).

import { describe, it, expect } from "vitest";
import {
  EDITOR_KEY_BINDINGS,
  PALETTE_OPEN_ACTION,
  FIND_OPEN_ACTION,
  REPLACE_OPEN_ACTION,
  SAVE_ACTION,
  chordFromEvent,
  resolveShortcut,
  bindingsForAction,
  isGlobalEditorAction,
  shouldContainChordFromCodeBlock,
} from "./editor_keymap";
import { EDITOR_COMMAND_BY_ID } from "./editor_commands";

/** Special editor-component actions handled outside the command catalog. */
const SPECIAL_ACTIONS = new Set([
  PALETTE_OPEN_ACTION,
  FIND_OPEN_ACTION,
  REPLACE_OPEN_ACTION,
  SAVE_ACTION,
]);

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

  it("ignores chords during IME composition (iteration-3 L15)", () => {
    expect(resolveShortcut({ key: "b", ctrlKey: true, isComposing: true })).toBeNull();
    expect(resolveShortcut({ key: "s", ctrlKey: true, isComposing: true })).toBeNull();
  });

  it("every bound command id (non-special) exists in the command catalog", () => {
    for (const binding of EDITOR_KEY_BINDINGS) {
      if (SPECIAL_ACTIONS.has(binding.action)) continue;
      expect(EDITOR_COMMAND_BY_ID.has(binding.action)).toBe(true);
    }
  });

  it("resolves the find/replace chords to their special actions (MT-244)", () => {
    expect(resolveShortcut({ key: "f", ctrlKey: true })).toBe(FIND_OPEN_ACTION);
    expect(resolveShortcut({ key: "f", metaKey: true })).toBe(FIND_OPEN_ACTION);
    expect(resolveShortcut({ key: "h", ctrlKey: true })).toBe(REPLACE_OPEN_ACTION);
  });

  it("resolves Mod-s to the save action and classifies it global (iteration-3 L16)", () => {
    expect(resolveShortcut({ key: "s", ctrlKey: true })).toBe(SAVE_ACTION);
    expect(resolveShortcut({ key: "s", metaKey: true })).toBe(SAVE_ACTION);
    expect(isGlobalEditorAction(SAVE_ACTION)).toBe(true);
    expect(isGlobalEditorAction(PALETTE_OPEN_ACTION)).toBe(true);
    expect(isGlobalEditorAction("format.bold")).toBe(false);
  });

  it("lists bindings for an action", () => {
    expect(bindingsForAction(PALETTE_OPEN_ACTION).length).toBeGreaterThanOrEqual(1);
    expect(bindingsForAction("format.bold")[0].chord).toBe("Mod-b");
  });
});

describe("code-block chord containment (iteration-3 H3)", () => {
  it("contains prose-level modifier chords originating in a code block", () => {
    // Our own bindings (non-global).
    expect(shouldContainChordFromCodeBlock({ key: "b", ctrlKey: true })).toBe(true);
    expect(shouldContainChordFromCodeBlock({ key: "k", ctrlKey: true })).toBe(true);
    expect(shouldContainChordFromCodeBlock({ key: "c", ctrlKey: true, altKey: true })).toBe(true);
    // StarterKit natives that would silently mutate prose.
    expect(shouldContainChordFromCodeBlock({ key: "1", ctrlKey: true, altKey: true })).toBe(true);
    expect(shouldContainChordFromCodeBlock({ key: "8", ctrlKey: true, shiftKey: true })).toBe(true);
  });

  it("never contains global, clipboard, undo, or plain keys", () => {
    // Global escape hatches.
    expect(shouldContainChordFromCodeBlock({ key: "p", ctrlKey: true, shiftKey: true })).toBe(false);
    expect(shouldContainChordFromCodeBlock({ key: "s", ctrlKey: true })).toBe(false);
    // Clipboard must keep its default behavior inside code.
    expect(shouldContainChordFromCodeBlock({ key: "c", ctrlKey: true })).toBe(false);
    expect(shouldContainChordFromCodeBlock({ key: "v", ctrlKey: true })).toBe(false);
    expect(shouldContainChordFromCodeBlock({ key: "x", ctrlKey: true })).toBe(false);
    // Undo/redo are Monaco-owned (and must work in the textarea fallback).
    expect(shouldContainChordFromCodeBlock({ key: "z", ctrlKey: true })).toBe(false);
    expect(shouldContainChordFromCodeBlock({ key: "y", ctrlKey: true })).toBe(false);
    // Plain typing and structural keys are never touched.
    expect(shouldContainChordFromCodeBlock({ key: "a" })).toBe(false);
    expect(shouldContainChordFromCodeBlock({ key: "Backspace" })).toBe(false);
    expect(shouldContainChordFromCodeBlock({ key: "Enter", shiftKey: true })).toBe(false);
  });
});
