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
  | "projection"
  | "integrity";

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
      // Iteration-3 H5: the hint must never instruct a destructive reload —
      // the editor preserves the local version as a snapshot first.
      hint:
        "Your local version is preserved as a snapshot (download or restore it " +
        "below). Reload fetches the latest server version; nothing is discarded.",
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
 *
 * Iteration-3 H2 (fail-closed): the document opens READ-ONLY and saving is
 * blocked — ProseMirror silently DROPS nodes its schema does not know, so one
 * save of an editable mismatched doc would persist the stripped content and
 * destroy the newer-schema data. The hint states that contract.
 */
export function schemaMismatchError(reason: string): EditorBackendError {
  return {
    kind: "schema",
    message: reason,
    hint:
      "The document is opened read-only and saving is disabled so no content " +
      "can be lost. Update Handshake to edit it.",
  };
}

/**
 * Typed error for code-block round-trip hash violations found on load
 * (iteration-3 M9 — verifyCodeBlockIntegrity wired into the product load
 * path). Editing stays possible: the backend content_sha256 remains the
 * durable authority and a re-save re-mints correct editor-layer hashes; the
 * banner makes the out-of-band alteration VISIBLE instead of silently trusted.
 */
export function codeIntegrityError(
  violations: number,
  checked: number,
): EditorBackendError {
  return {
    kind: "integrity",
    message:
      `${violations} of ${checked} embedded code block(s) failed the round-trip ` +
      `integrity check (stored hash does not match {language, code}).`,
    hint:
      "The content was altered outside the editor or by an older defect. " +
      "Review the code blocks; saving recomputes the integrity hashes.",
  };
}

/**
 * Typed error for a save attempt against a schema-blocked document (H2
 * defense-in-depth: the Save button is disabled AND the save path refuses).
 */
export function schemaSaveBlockedError(reason: string): EditorBackendError {
  return {
    kind: "schema",
    message: `Save blocked: ${reason}`,
    hint:
      "Saving this document from an older editor would silently drop content " +
      "the newer schema added. Update Handshake, then edit and save.",
  };
}
