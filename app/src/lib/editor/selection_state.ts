// WP-KERNEL-009 / MT-171 — EditorSelectionAndCursorState.
//
// Typed, actor-attributed selection/cursor snapshots for operator/model/agent
// actors, with privacy + quiet-mode constraints. The editor produces a
// SelectionSnapshot describing WHERE an actor's caret/selection is (UTF-8 byte
// offsets, matching the existing TiptapSelectionInfo convention in
// TiptapEditor.tsx) and WHO owns it, so multiple parallel actors' positions can
// be rendered/attributed (Spec §7.1.1.8 multi-actor; GLOBAL-BUILD parallel model
// paths) — observable and attributable, never relying on hidden session state.
//
// Privacy: a non-operator actor's selection TEXT is redacted by default (only
// the range is shared) so a model/agent cursor does not leak document content
// into a shared presence channel; quiet-mode drops presence entirely. These are
// pure transforms over a minimal editor-state shape, unit-testable without a DOM.

export type SelectionActorKind = "operator" | "local_model" | "cloud_model" | "agent" | "validator";

/** A minimal slice of editor selection state (decouples from Tiptap types). */
export interface EditorSelectionInput {
  /** Selected text (may be empty for a collapsed caret). */
  text: string;
  /** UTF-8 byte offset of selection start from document start. */
  startUtf8: number;
  /** UTF-8 byte offset of selection end. */
  endUtf8: number;
  /** True when start === end (a caret, not a range). */
  collapsed: boolean;
}

export interface SelectionActor {
  kind: SelectionActorKind;
  /** Stable actor id (operator id, session id, agent id). */
  id: string;
  /** Display label. */
  label?: string;
}

/** A privacy-and-quiet-mode-aware presence snapshot for one actor. */
export interface SelectionSnapshot {
  actor: SelectionActor;
  startUtf8: number;
  endUtf8: number;
  collapsed: boolean;
  /** Selection text — present only when sharing is permitted (see policy). */
  text: string | null;
  /** Why text was withheld, when it was. */
  redactedReason: "none" | "privacy_non_operator" | "quiet_mode";
}

export interface PresencePolicy {
  /**
   * Quiet mode: no presence is broadcast at all (returns null). Mirrors the
   * GLOBAL-BUILD-QUIET / HBR-QUIET non-intrusive constraint — a model working in
   * the doc must not light up a shared cursor channel.
   */
  quietMode: boolean;
  /**
   * Whether a non-operator actor may share its selection TEXT (not just range).
   * Default false: only the operator's own selection text is shared.
   */
  shareNonOperatorText: boolean;
}

export const DEFAULT_PRESENCE_POLICY: PresencePolicy = {
  quietMode: false,
  shareNonOperatorText: false,
};

/**
 * Builds a privacy-aware presence snapshot for an actor's selection under a
 * policy. Returns null in quiet mode (no presence at all). For a non-operator
 * actor whose text sharing is not permitted, the range is shared but the text is
 * redacted. The operator's own selection text is always available to itself.
 */
export function buildSelectionSnapshot(
  actor: SelectionActor,
  selection: EditorSelectionInput,
  policy: PresencePolicy = DEFAULT_PRESENCE_POLICY,
): SelectionSnapshot | null {
  if (policy.quietMode) return null;

  const isOperator = actor.kind === "operator";
  const mayShareText = isOperator || policy.shareNonOperatorText;
  return {
    actor,
    startUtf8: selection.startUtf8,
    endUtf8: selection.endUtf8,
    collapsed: selection.collapsed,
    text: mayShareText ? selection.text : null,
    redactedReason: mayShareText ? "none" : "privacy_non_operator",
  };
}

/**
 * Computes a SelectionSnapshot directly from raw selection text + UTF-8 offsets
 * (the shape TiptapEditor.tsx already derives via TextEncoder). Convenience over
 * constructing EditorSelectionInput by hand.
 */
export function snapshotFromOffsets(
  actor: SelectionActor,
  text: string,
  startUtf8: number,
  endUtf8: number,
  policy: PresencePolicy = DEFAULT_PRESENCE_POLICY,
): SelectionSnapshot | null {
  return buildSelectionSnapshot(
    actor,
    { text, startUtf8, endUtf8, collapsed: startUtf8 === endUtf8 },
    policy,
  );
}

/**
 * Merges multiple actors' snapshots into a stable, attributable presence list
 * (operator first, then others by id), dropping nulls (quiet/absent). Used by
 * the editor to render parallel cursors deterministically.
 */
export function mergePresence(
  snapshots: Array<SelectionSnapshot | null>,
): SelectionSnapshot[] {
  const present = snapshots.filter((s): s is SelectionSnapshot => s !== null);
  return present.sort((a, b) => {
    if (a.actor.kind === "operator" && b.actor.kind !== "operator") return -1;
    if (b.actor.kind === "operator" && a.actor.kind !== "operator") return 1;
    return a.actor.id.localeCompare(b.actor.id);
  });
}
