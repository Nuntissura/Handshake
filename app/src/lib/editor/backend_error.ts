// WP-KERNEL-009 / MT-174 — EditorBackendErrorStates (classification).
//
// Maps a raw backend failure (save/load error message) or a schema-assertion
// result into a TYPED EditorBackendError the editor renders inline (MT-174) —
// never a blank screen, always an actionable message. Pure classification so it
// is unit-testable; the editor component owns the rendering.
//
// The backend authority (PostgreSQL/EventLedger via the rich-doc API) returns
// optimistic-concurrency conflicts, schema mismatches, and load failures as
// error messages; this normalizes them into a small typed vocabulary the UI and
// the visual-debug selectors key on (data-error-kind).

export type EditorBackendErrorKind =
  | "save"
  | "load"
  | "conflict"
  | "schema"
  | "index"
  | "projection";

export interface EditorBackendError {
  kind: EditorBackendErrorKind;
  message: string;
  hint?: string;
}

/** Classifies a save failure message into a typed error (conflict vs generic). */
export function classifySaveError(error: unknown): EditorBackendError {
  const message = error instanceof Error ? error.message : String(error);
  const lower = message.toLowerCase();
  // Schema is checked before conflict: a "schema_version mismatch" contains the
  // substring "version" but is a schema problem, not an optimistic-concurrency
  // conflict.
  if (lower.includes("schema")) {
    return { kind: "schema", message, hint: "The document schema changed; reload to migrate." };
  }
  if (
    lower.includes("conflict") ||
    lower.includes("version") ||
    lower.includes("409") ||
    lower.includes("expected_version") ||
    lower.includes("optimistic")
  ) {
    return {
      kind: "conflict",
      message,
      hint: "Reload to get the latest version, then re-apply your edit.",
    };
  }
  return { kind: "save", message, hint: "Your edits are kept locally; try saving again." };
}

/** Classifies a load failure message into a typed error. */
export function classifyLoadError(error: unknown): EditorBackendError {
  const message = error instanceof Error ? error.message : String(error);
  const lower = message.toLowerCase();
  if (lower.includes("schema")) {
    return { kind: "schema", message, hint: "Update Handshake if the document is newer." };
  }
  if (lower.includes("projection")) {
    return { kind: "projection", message, hint: "The projection could not be built; the authority document is unaffected." };
  }
  if (lower.includes("index")) {
    return { kind: "index", message };
  }
  return { kind: "load", message, hint: "Reload, or check the backend diagnostics." };
}

/**
 * Builds a typed schema error from a failed editor schema assertion (MT-162),
 * so a newer-than-editor or unknown-version document surfaces as a "schema"
 * backend error rather than crashing the load.
 */
export function schemaMismatchError(reason: string): EditorBackendError {
  return { kind: "schema", message: reason, hint: undefined };
}
