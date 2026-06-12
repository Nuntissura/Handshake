// WP-KERNEL-009 iteration-3 hardening (H1) — structural document equality.
//
// Deep structural equality over ProseMirror/Tiptap document JSON (or any
// JSON-shaped value). Used by the editor's reload effect to distinguish a
// GENUINE external document update (backend reload, conflict resolution) from a
// re-render that passes back a structurally identical document (the echo loop
// the adversarial review proved teleports the caret on every keystroke).
//
// Pure and dependency-free so it is unit-testable and reusable by any surface
// that needs "did the document actually change" semantics (the identity-based
// lastEmitted guard in RichTextEditor catches the common echo case without ever
// reaching this comparison; this handles clones from JSON round-trips).

/** Deep structural equality for JSON-shaped values (objects/arrays/primitives). */
export function jsonDeepEquals(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === null || b === null) return false;
  if (typeof a !== typeof b) return false;
  if (Array.isArray(a)) {
    if (!Array.isArray(b) || a.length !== b.length) return false;
    for (let i = 0; i < a.length; i++) {
      if (!jsonDeepEquals(a[i], b[i])) return false;
    }
    return true;
  }
  if (typeof a === "object") {
    if (Array.isArray(b)) return false;
    const objA = a as Record<string, unknown>;
    const objB = b as Record<string, unknown>;
    const keysA = Object.keys(objA);
    const keysB = Object.keys(objB);
    if (keysA.length !== keysB.length) return false;
    for (const key of keysA) {
      if (!Object.prototype.hasOwnProperty.call(objB, key)) return false;
      if (!jsonDeepEquals(objA[key], objB[key])) return false;
    }
    return true;
  }
  // Primitives that failed the identity check (JSON has no NaN, so no NaN case).
  return false;
}
