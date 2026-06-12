// WP-KERNEL-009 / MT-169..174 — the integrated Handshake rich-text editor.
//
// The real editing surface (the operator's VS Code-editing replacement) that
// composes everything built in TiptapMonacoIntegration:
//   - the full extension set (StarterKit + tables/task-lists/mentions/tags +
//     typed wikilinks (MT-163) + embedded Monaco code blocks (MT-165) +
//     prose->code rules (MT-164)) via buildHandshakeEditorExtensions,
//   - a feature-complete TOOLBAR + overflow menu driven by the command catalog
//     (MT-169),
//   - a KEYBOARD + COMMAND PALETTE driven by the explicit keymap (MT-170),
//   - actor-attributed SELECTION/CURSOR state with privacy/quiet-mode (MT-171),
//   - stable VISUAL-DEBUG selectors + a debug payload (MT-172),
//   - ACCESSIBILITY/readability affordances (roles, focus order, labels — MT-173),
//   - inline BACKEND ERROR states (save/load/conflict/schema), reusing the
//     DependencyFailureBanner pattern, never a blank surface (MT-174).
//
// It is presentational over a controlled document model: the parent owns load /
// save / authority (RichDocumentView wires it to the rich-doc API). Mounting
// degrades gracefully — a failed extension set reports a typed dependency
// failure and the editor still renders its chrome with an inline notice rather
// than blanking.

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import type { JSONContent } from "@tiptap/core";
import type { AnyExtension } from "@tiptap/core";
import { buildHandshakeEditorExtensions } from "../lib/editor/build_editor_extensions";
import {
  EDITOR_COMMANDS,
  EDITOR_COMMAND_BY_ID,
  filterEditorCommands,
  commandRequiresArg,
  type EditorCommandDescriptor,
} from "../lib/editor/editor_commands";
import {
  resolveShortcut,
  bindingsForAction,
  PALETTE_OPEN_ACTION,
} from "../lib/editor/editor_keymap";
import {
  snapshotFromOffsets,
  type SelectionActor,
  type SelectionSnapshot,
  type PresencePolicy,
  DEFAULT_PRESENCE_POLICY,
} from "../lib/editor/selection_state";
import {
  dependencyFailures,
  formatDependencyFailureMessage,
} from "../lib/dependency_policy/dependency_failure";
import {
  buildEditorDebugSnapshot,
  EDITOR_DEBUG_GLOBAL_KEY,
  type EditorDebugSnapshot,
} from "../lib/editor/visual_debug";
import type {
  EditorBackendError,
  EditorBackendErrorKind,
} from "../lib/editor/backend_error";
import { DependencyFailureBanner } from "./DependencyFailureBanner";

export type { EditorBackendError } from "../lib/editor/backend_error";

export type RichTextEditorProps = {
  initialContent: JSONContent | null;
  onChange: (doc: JSONContent) => void;
  readOnly?: boolean;
  /** Actor whose selection presence is computed (defaults to operator). */
  actor?: SelectionActor;
  presencePolicy?: PresencePolicy;
  onSelectionSnapshot?: (snapshot: SelectionSnapshot | null) => void;
  /** Typed backend error rendered inline (MT-174); null = none. */
  backendError?: EditorBackendError | null;
  /** Extension factory override (tests / DI). */
  extensionFactory?: () => AnyExtension[];
};

const DEFAULT_ACTOR: SelectionActor = { kind: "operator", id: "operator", label: "Operator" };

/** Guarded extension build: a failure reports a typed dependency failure. */
function buildGuardedExtensions(
  factory: () => AnyExtension[],
): AnyExtension[] | null {
  try {
    return factory();
  } catch (error) {
    const failure = {
      dependency: "@tiptap/starter-kit",
      component: "extension:rich-text-editor",
      phase: "extension_init" as const,
      cause: error instanceof Error ? error.message : String(error),
    };
    dependencyFailures.report({ ...failure, message: formatDependencyFailureMessage(failure) });
    return null;
  }
}

export function RichTextEditor(props: RichTextEditorProps) {
  const { extensionFactory } = props;
  const extensions = useMemo(
    () => buildGuardedExtensions(extensionFactory ?? (() => buildHandshakeEditorExtensions())),
    [extensionFactory],
  );
  if (extensions === null) {
    // Never blank: show the typed failure surface + an inline notice.
    return (
      <div className="rich-text-editor" data-testid="rich-text-editor" data-editor-degraded="true">
        <DependencyFailureBanner />
        <div className="rich-text-editor__fatal" role="alert" data-testid="rich-text-editor-fatal">
          The editor could not initialize its bundled components. Restart Handshake; no
          download is required (every editor asset ships inside the app).
        </div>
      </div>
    );
  }
  return <RichTextEditorInner extensions={extensions} {...props} />;
}

function RichTextEditorInner({
  extensions,
  initialContent,
  onChange,
  readOnly = false,
  actor = DEFAULT_ACTOR,
  presencePolicy = DEFAULT_PRESENCE_POLICY,
  onSelectionSnapshot,
  backendError = null,
}: RichTextEditorProps & { extensions: AnyExtension[] }) {
  const [, forceRefresh] = useState(0);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState("");
  const [pendingCommand, setPendingCommand] = useState<EditorCommandDescriptor | null>(null);
  const [argValues, setArgValues] = useState<Record<string, string>>({});
  const paletteInputRef = useRef<HTMLInputElement>(null);

  const [debugSnapshot, setDebugSnapshot] = useState<EditorDebugSnapshot | null>(null);

  const editor = useEditor({
    extensions,
    content: initialContent ?? { type: "doc", content: [{ type: "paragraph" }] },
    editable: !readOnly,
    onUpdate: ({ editor }) => onChange(editor.getJSON()),
  });

  // Reload content when the upstream document changes.
  useEffect(() => {
    if (!editor) return;
    editor.commands.setContent(
      initialContent ?? { type: "doc", content: [{ type: "paragraph" }] },
      { emitUpdate: false },
    );
  }, [editor, initialContent]);

  // Publishes the machine-readable visual-debug snapshot (MT-172) on a global
  // and on component state so the visual lane / a no-context model can read it.
  const publishDebug = useCallback(() => {
    if (!editor) return;
    const snapshot = buildEditorDebugSnapshot(editor);
    setDebugSnapshot(snapshot);
    (globalThis as Record<string, unknown>)[EDITOR_DEBUG_GLOBAL_KEY] = snapshot;
  }, [editor]);

  // Toolbar active-state refresh + actor-attributed selection snapshot (MT-171)
  // + visual-debug publication (MT-172).
  useEffect(() => {
    if (!editor) return;
    const onSelection = () => {
      forceRefresh((t) => t + 1);
      publishDebug();
      if (!onSelectionSnapshot) return;
      const { from, to } = editor.state.selection;
      const doc = editor.state.doc;
      const prefix = doc.textBetween(0, from, "\n");
      const selected = doc.textBetween(from, to, "\n");
      const enc = new TextEncoder();
      const startUtf8 = enc.encode(prefix).length;
      const endUtf8 = startUtf8 + enc.encode(selected).length;
      onSelectionSnapshot(snapshotFromOffsets(actor, selected, startUtf8, endUtf8, presencePolicy));
    };
    const onTx = () => {
      forceRefresh((t) => t + 1);
      publishDebug();
    };
    editor.on("selectionUpdate", onSelection);
    editor.on("transaction", onTx);
    // Defer the initial publish out of the effect body so it does not cascade a
    // synchronous re-render (the live updates flow through editor events above).
    const initial = queueMicrotask
      ? (queueMicrotask(publishDebug), undefined)
      : setTimeout(publishDebug, 0);
    return () => {
      if (initial !== undefined) clearTimeout(initial);
      editor.off("selectionUpdate", onSelection);
      editor.off("transaction", onTx);
    };
  }, [editor, onSelectionSnapshot, actor, presencePolicy, publishDebug]);

  const runCommand = useCallback(
    (cmd: EditorCommandDescriptor, args?: Record<string, string>) => {
      if (!editor) return;
      if (commandRequiresArg(cmd) && !args) {
        setPendingCommand(cmd);
        setArgValues({});
        return;
      }
      cmd.run(editor, args);
      setPaletteOpen(false);
      setPaletteQuery("");
      setPendingCommand(null);
    },
    [editor],
  );

  // Keyboard: explicit keymap (MT-170). Palette open + bound commands.
  useEffect(() => {
    if (!editor) return;
    const handler = (event: KeyboardEvent) => {
      const action = resolveShortcut(event);
      if (!action) return;
      event.preventDefault();
      if (action === PALETTE_OPEN_ACTION) {
        setPaletteOpen(true);
        return;
      }
      const cmd = EDITOR_COMMAND_BY_ID.get(action);
      if (cmd) runCommand(cmd);
    };
    const root = editor.view.dom;
    root.addEventListener("keydown", handler);
    return () => root.removeEventListener("keydown", handler);
  }, [editor, runCommand]);

  useEffect(() => {
    if (paletteOpen) paletteInputRef.current?.focus();
  }, [paletteOpen]);

  if (!editor) return null;

  const toolbarCommands = EDITOR_COMMANDS.filter((c) =>
    ["format", "block", "list", "table", "code", "link"].includes(c.category),
  );
  const overflowCommands = EDITOR_COMMANDS.filter((c) =>
    ["embed", "graph", "mention", "manual"].includes(c.category),
  );
  const paletteResults = filterEditorCommands(paletteQuery);

  return (
    <div
      className="rich-text-editor"
      data-testid="rich-text-editor"
      data-editor-degraded="false"
      data-readonly={readOnly ? "true" : "false"}
      data-code-block-count={String(debugSnapshot?.codeBlocks.length ?? 0)}
      data-link-count={String(debugSnapshot?.links.length ?? 0)}
      data-selection-empty={debugSnapshot ? String(debugSnapshot.selection.empty) : "true"}
    >
      <DependencyFailureBanner />

      {/* Inline backend error (MT-174): never a blank screen. */}
      {backendError && (
        <div
          className={`rich-text-editor__backend-error rich-text-editor__backend-error--${backendError.kind}`}
          role="alert"
          data-testid="rich-text-editor-backend-error"
          data-error-kind={backendError.kind}
        >
          <strong>{backendErrorTitle(backendError.kind)}:</strong> {backendError.message}
          {backendError.hint ? <span className="muted"> {backendError.hint}</span> : null}
        </div>
      )}

      {/* Toolbar (MT-169) — labelled, keyboard-reachable (MT-173). */}
      <div
        className="rich-text-editor__toolbar"
        role="toolbar"
        aria-label="Editor formatting"
        data-testid="rich-text-editor-toolbar"
      >
        {toolbarCommands.map((cmd) => (
          <button
            key={cmd.id}
            type="button"
            className={cmd.isActive?.(editor) ? "tt-button tt-button--active" : "tt-button"}
            data-testid={`editor-cmd-${cmd.id}`}
            data-command-id={cmd.id}
            data-active={cmd.isActive?.(editor) ? "true" : "false"}
            aria-pressed={cmd.isActive?.(editor) ? "true" : "false"}
            aria-label={ariaLabelFor(cmd)}
            title={ariaLabelFor(cmd)}
            disabled={readOnly}
            onClick={() => runCommand(cmd)}
          >
            {cmd.label}
          </button>
        ))}
        <button
          type="button"
          className="tt-button"
          data-testid="editor-open-palette"
          aria-label="Open command palette (more actions)"
          title="More actions (Ctrl/Cmd+P)"
          onClick={() => setPaletteOpen(true)}
        >
          More…
        </button>
      </div>

      {/* The editing surface. */}
      <div className="rich-text-editor__surface tiptap-scroll" data-testid="rich-text-editor-surface">
        <EditorContent editor={editor} />
      </div>

      {/* Command palette (MT-170) — overflow + searchable all-command surface. */}
      {paletteOpen && (
        <div
          className="rich-text-editor__palette"
          role="dialog"
          aria-label="Command palette"
          aria-modal="true"
          data-testid="editor-command-palette"
        >
          <input
            ref={paletteInputRef}
            type="text"
            className="rich-text-editor__palette-input"
            data-testid="editor-command-palette-input"
            aria-label="Search editor commands"
            placeholder="Search commands…"
            value={paletteQuery}
            onChange={(e) => setPaletteQuery(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setPaletteOpen(false);
                setPendingCommand(null);
              }
            }}
          />
          <ul className="rich-text-editor__palette-list" role="listbox" data-testid="editor-command-palette-list">
            {paletteResults.map((cmd) => (
              <li key={cmd.id} role="option" aria-selected="false">
                <button
                  type="button"
                  className="rich-text-editor__palette-item"
                  data-testid={`palette-cmd-${cmd.id}`}
                  data-command-id={cmd.id}
                  onClick={() => runCommand(cmd)}
                >
                  <span>{cmd.label}</span>
                  <span className="muted small">{paletteHint(cmd)}</span>
                </button>
              </li>
            ))}
            {paletteResults.length === 0 && (
              <li className="muted" data-testid="editor-command-palette-empty">
                No matching commands.
              </li>
            )}
          </ul>
        </div>
      )}

      {/* Arg prompt for commands needing input (link target, language, …). */}
      {pendingCommand && (
        <div
          className="rich-text-editor__arg-prompt"
          role="dialog"
          aria-label={`${pendingCommand.label} options`}
          data-testid="editor-arg-prompt"
        >
          <h4>{pendingCommand.label}</h4>
          {(pendingCommand.args ?? []).map((arg) => (
            <label key={arg.id} className="rich-text-editor__arg-field">
              <span className="muted small">{arg.label}</span>
              <input
                type="text"
                data-testid={`editor-arg-${arg.id}`}
                placeholder={arg.placeholder}
                value={argValues[arg.id] ?? ""}
                onChange={(e) => setArgValues((prev) => ({ ...prev, [arg.id]: e.target.value }))}
              />
            </label>
          ))}
          <div className="rich-text-editor__arg-actions">
            <button
              type="button"
              data-testid="editor-arg-confirm"
              onClick={() => runCommand(pendingCommand, argValues)}
            >
              Insert
            </button>
            <button type="button" data-testid="editor-arg-cancel" onClick={() => setPendingCommand(null)}>
              Cancel
            </button>
          </div>
        </div>
      )}

      {/* Overflow menu commands also reachable as a static list for discovery. */}
      <div className="rich-text-editor__overflow" data-testid="rich-text-editor-overflow" hidden>
        {overflowCommands.map((cmd) => (
          <span key={cmd.id} data-command-id={cmd.id} />
        ))}
      </div>
    </div>
  );
}

function backendErrorTitle(kind: EditorBackendErrorKind): string {
  switch (kind) {
    case "save":
      return "Save failed";
    case "load":
      return "Load failed";
    case "conflict":
      return "Version conflict";
    case "schema":
      return "Schema mismatch";
    case "index":
      return "Index error";
    case "projection":
      return "Projection error";
  }
}

function ariaLabelFor(cmd: EditorCommandDescriptor): string {
  const hint = bindingsForAction(cmd.id)[0]?.chord;
  return hint ? `${cmd.label} (${hint})` : cmd.label;
}

function paletteHint(cmd: EditorCommandDescriptor): string {
  const chord = bindingsForAction(cmd.id)[0]?.chord;
  return chord ?? cmd.category;
}
