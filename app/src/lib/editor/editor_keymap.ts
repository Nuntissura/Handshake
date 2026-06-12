// WP-KERNEL-009 / MT-170 — EditorKeyboardAndCommandPalette (keymap).
//
// Explicit, machine-readable keyboard bindings for editor commands and the
// command palette — no hidden chat context, no implicit global handler. Each
// binding names a command id from the catalog (editor_commands.ts) so the
// keyboard, toolbar, and palette stay one surface. A pure `resolveShortcut`
// turns a normalized key event into a command id (or "palette.open") so the
// editor component can dispatch deterministically and tests can assert bindings
// without a DOM.
//
// "Mod" = Cmd on macOS, Ctrl elsewhere; the resolver accepts either metaKey or
// ctrlKey for Mod so the binding is portable.

export interface KeyboardEventLike {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
}

/** A single keyboard binding. `action` is a command id or a special token. */
export interface KeyBinding {
  /** Display chord, e.g. "Mod-b", "Mod-Shift-p". */
  chord: string;
  /** Command id from editor_commands.ts, or a special editor action token. */
  action: string;
  /** Human description. */
  description: string;
}

/** Special non-command actions the editor itself handles. */
export const PALETTE_OPEN_ACTION = "palette.open";
/** Opens the document-wide find panel (MT-244). */
export const FIND_OPEN_ACTION = "find.open";
/** Opens the find panel with the replace row (MT-244). */
export const REPLACE_OPEN_ACTION = "replace.open";

/**
 * The explicit binding table. Formatting chords (Mod-b/i/e) are also provided
 * natively by StarterKit; listing them here keeps the catalog the single source
 * of truth for what the UI advertises and lets the palette show hints.
 */
export const EDITOR_KEY_BINDINGS: readonly KeyBinding[] = [
  { chord: "Mod-b", action: "format.bold", description: "Bold" },
  { chord: "Mod-i", action: "format.italic", description: "Italic" },
  { chord: "Mod-e", action: "format.code", description: "Inline code" },
  { chord: "Mod-k", action: "link.wikilink", description: "Insert link" },
  { chord: "Mod-Alt-c", action: "code.insert", description: "Insert code block" },
  { chord: "Mod-p", action: PALETTE_OPEN_ACTION, description: "Open command palette" },
  { chord: "Mod-Shift-p", action: PALETTE_OPEN_ACTION, description: "Open command palette" },
  { chord: "Mod-f", action: FIND_OPEN_ACTION, description: "Find in document" },
  { chord: "Mod-h", action: REPLACE_OPEN_ACTION, description: "Find and replace" },
] as const;

/** Builds the canonical chord string for a key event (Mod-… form). */
export function chordFromEvent(event: KeyboardEventLike): string {
  const parts: string[] = [];
  if (event.ctrlKey || event.metaKey) parts.push("Mod");
  if (event.altKey) parts.push("Alt");
  if (event.shiftKey) parts.push("Shift");
  const key = event.key.length === 1 ? event.key.toLowerCase() : event.key;
  parts.push(key);
  return parts.join("-");
}

/**
 * Resolves a key event to a bound action (command id or PALETTE_OPEN_ACTION),
 * or null when nothing is bound. Deterministic; pure.
 */
export function resolveShortcut(event: KeyboardEventLike): string | null {
  const chord = chordFromEvent(event);
  const binding = EDITOR_KEY_BINDINGS.find((b) => b.chord === chord);
  return binding?.action ?? null;
}

/** All bindings for a given command id (for showing hints in the UI). */
export function bindingsForAction(action: string): KeyBinding[] {
  return EDITOR_KEY_BINDINGS.filter((b) => b.action === action);
}
