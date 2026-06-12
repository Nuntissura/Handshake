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
  /** True while an IME composition is active — no chord may fire then. */
  isComposing?: boolean;
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
/** Requests a save from the owning document shell (iteration-3 L16/EXT-SAVE-001). */
export const SAVE_ACTION = "editor.save";

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
  { chord: "Mod-s", action: SAVE_ACTION, description: "Save document" },
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
 * or null when nothing is bound. Deterministic; pure. Returns null during an
 * IME composition (iteration-3 L15): firing chords mid-composition would
 * corrupt the composed text.
 */
export function resolveShortcut(event: KeyboardEventLike): string | null {
  if (event.isComposing) return null;
  const chord = chordFromEvent(event);
  const binding = EDITOR_KEY_BINDINGS.find((b) => b.chord === chord);
  return binding?.action ?? null;
}

/**
 * Actions that stay global even when the keystroke originates INSIDE an
 * embedded code editor (iteration-3 H3). Everything else typed in a Monaco
 * code block belongs to Monaco — the prose keymap must not intercept it
 * (Mod-Alt-c / Mod-k typed in code were replacing the node-selected block).
 * The command palette is the deliberate escape hatch (VS Code parity: F1 /
 * Ctrl+Shift+P works everywhere).
 */
export const CODE_BLOCK_GLOBAL_ACTIONS: ReadonlySet<string> = new Set([
  PALETTE_OPEN_ACTION,
  // Save is document-level: Mod-s typed while editing code must save the
  // document (VS Code parity), not vanish into the code island.
  SAVE_ACTION,
]);

/** True when `action` may fire from a keystroke originating inside a code block. */
export function isGlobalEditorAction(action: string): boolean {
  return CODE_BLOCK_GLOBAL_ACTIONS.has(action);
}

/**
 * Prose-level chords that must be CONTAINED when they bubble out of an
 * embedded Monaco editor (iteration-3 H3). Two sources:
 *   1. this keymap's own bindings (minus the global escape hatches), and
 *   2. StarterKit's native ProseMirror keymaps — those run on the PROSE state
 *      even when the key event originated inside the code island (PM applies
 *      its keymap regardless of event origin), silently mutating the document
 *      (e.g. Mod-Alt-1 turning the paragraph above into a heading while the
 *      operator types in code).
 *
 * Deliberately NOT contained:
 *   - Mod-z / Mod-y / Mod-Shift-z — Monaco resolves undo/redo itself and stops
 *     propagation; they never bubble, and containing them would risk breaking
 *     undo in the degraded textarea fallback.
 *   - Mod-c / Mod-v / Mod-x — clipboard keys must keep their browser/Monaco
 *     default behavior; preventing them would break copy/paste inside code.
 */
const STARTERKIT_PROSE_CHORDS: readonly string[] = [
  "Mod-b",
  "Mod-i",
  "Mod-u",
  "Mod-e",
  "Mod-Shift-s",
  "Mod-Shift-x",
  "Mod-Alt-c",
  "Mod-Shift-b",
  "Mod-Shift-7",
  "Mod-Shift-8",
  "Mod-Alt-0",
  "Mod-Alt-1",
  "Mod-Alt-2",
  "Mod-Alt-3",
  "Mod-Alt-4",
  "Mod-Alt-5",
  "Mod-Alt-6",
  "Mod-Enter",
];

const CONTAINED_CODE_BLOCK_CHORDS: ReadonlySet<string> = new Set([
  ...STARTERKIT_PROSE_CHORDS,
  ...EDITOR_KEY_BINDINGS.filter((b) => !CODE_BLOCK_GLOBAL_ACTIONS.has(b.action)).map(
    (b) => b.chord,
  ),
]);

/**
 * True when a key event that originated inside an embedded code editor must be
 * contained (claimed and ignored) instead of reaching the prose keymaps.
 * Only modifier chords are ever contained — plain typing, Enter, Backspace and
 * friends keep their Monaco/textarea behavior untouched.
 */
export function shouldContainChordFromCodeBlock(event: KeyboardEventLike): boolean {
  if (!event.ctrlKey && !event.metaKey) return false;
  const action = resolveShortcut(event);
  if (action && isGlobalEditorAction(action)) return false;
  return CONTAINED_CODE_BLOCK_CHORDS.has(chordFromEvent(event));
}

/** All bindings for a given command id (for showing hints in the UI). */
export function bindingsForAction(action: string): KeyBinding[] {
  return EDITOR_KEY_BINDINGS.filter((b) => b.action === action);
}
