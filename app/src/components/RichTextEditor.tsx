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

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type RefObject,
} from "react";
import { EditorContent, useEditor } from "@tiptap/react";
import type { Editor, JSONContent } from "@tiptap/core";
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
  isGlobalEditorAction,
  PALETTE_OPEN_ACTION,
  FIND_OPEN_ACTION,
  REPLACE_OPEN_ACTION,
  SAVE_ACTION,
} from "../lib/editor/editor_keymap";
import { jsonDeepEquals } from "../lib/editor/doc_equality";
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
  isEditorDebugEnabled,
  publishEditorDebugSnapshot,
  EDITOR_LAST_EXPORT_GLOBAL_KEY,
  type EditorDebugSnapshot,
} from "../lib/editor/visual_debug";
import {
  EXPORT_FORMATS,
  exportHtml,
  exportMarkdown,
  exportPlainText,
  exportProseMirrorJson,
  buildExportFilename,
  type ExportFormatId,
} from "../lib/editor/export_formats";
import { saveTextToFile } from "../lib/editor/save_to_file";
import { revealCodeBlockRange } from "../lib/editor/code_block_find_registry";
import {
  buildEditorOutline,
  buildEditorStatus,
  codeLineRange,
  findFocusedCodeBlock,
  MONACO_CODE_BLOCK_STATUS_EVENT,
  type MonacoCursorSnapshot,
  type DocumentChromeStatus,
} from "../lib/editor/editor_chrome";
import { FindReplacePanel } from "./FindReplacePanel";
import type {
  EditorBackendError,
  EditorBackendErrorKind,
} from "../lib/editor/backend_error";
import type { EmbedResolverContext } from "../lib/editor/embed_assets";
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
  /**
   * Workspace/transport context for media embed NodeViews (MT-244). Omitted =
   * media embeds render a typed no_workspace error (fail-closed).
   */
  embedContext?: EmbedResolverContext;
  /** Document title used for export file naming (MT-244); optional. */
  documentTitle?: string;
  /**
   * Called once the Tiptap editor instance exists. Lets the parent (document
   * shell, merge UI, tests, diagnostics) reach the live editor without global
   * state — e.g. to flush content before a save or to drive typing in tests.
   */
  onEditorReady?: (editor: Editor) => void;
  /**
   * Called when the operator requests a save (Mod-s, including from inside an
   * embedded code block, or the palette "Save document" entry). The editor
   * never persists by itself — the parent owns the save path. Mod-s is always
   * preventDefault-ed (the browser save dialog must never appear).
   */
  onSaveRequested?: () => void;
  /** Authority-owned save/dirty/conflict state for the MT-245 status bar. */
  documentStatus?: DocumentChromeStatus;
  /**
   * Stable id for the per-editor debug namespace (iteration-3 L19):
   * `__HS_EDITOR_DEBUG_BY_ID__[debugId]`. Defaults to "default"; document
   * shells pass their document id so parallel editors stay attributable.
   */
  debugId?: string;
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
  const { extensionFactory, embedContext } = props;
  const extensions = useMemo(
    () =>
      buildGuardedExtensions(
        extensionFactory ?? (() => buildHandshakeEditorExtensions({ embedContext })),
      ),
    [extensionFactory, embedContext],
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

/**
 * Component-level palette commands (MT-244): UI-state actions (find/replace
 * panel, export menu, export formats) that operate on the editor COMPONENT
 * rather than the document, so they live beside — not inside — the pure
 * editor command catalog.
 */
interface ComponentCommand {
  id: string;
  label: string;
  keywords: string[];
}

const GOTO_LINE_ACTION = "navigate.gotoLine";

const COMPONENT_COMMANDS: readonly ComponentCommand[] = [
  { id: FIND_OPEN_ACTION, label: "Find in document", keywords: ["find", "search", "match"] },
  { id: REPLACE_OPEN_ACTION, label: "Find and replace", keywords: ["replace", "find", "substitute"] },
  { id: GOTO_LINE_ACTION, label: "Go to line in code block", keywords: ["go", "goto", "line", "code"] },
  ...EXPORT_FORMATS.map((format) => ({
    id: `export.${format.id}`,
    label: `Export: ${format.label}`,
    keywords: ["export", "save", "download", format.extension, format.id],
  })),
];

function filterComponentCommands(query: string, extra: readonly ComponentCommand[]): ComponentCommand[] {
  const all = [...COMPONENT_COMMANDS, ...extra];
  const q = query.trim().toLowerCase();
  if (q.length === 0) return all;
  return all.filter(
    (cmd) =>
      cmd.id.toLowerCase().includes(q) ||
      cmd.label.toLowerCase().includes(q) ||
      cmd.keywords.some((k) => k.toLowerCase().includes(q)),
  );
}

/**
 * Dialog focus management (iteration-3 M13): remembers the element focused
 * before a dialog opened and restores it on close; returns a Tab-trap keydown
 * handler that cycles focus within the dialog container (aria-modal dialogs
 * previously let Tab walk out of the dialog and never restored focus).
 */
function useDialogFocus(
  open: boolean,
  containerRef: RefObject<HTMLDivElement | null>,
): (event: ReactKeyboardEvent) => void {
  const restoreRef = useRef<HTMLElement | null>(null);
  useEffect(() => {
    if (!open) return;
    restoreRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    return () => {
      restoreRef.current?.focus?.();
      restoreRef.current = null;
    };
  }, [open]);
  return useCallback(
    (event: ReactKeyboardEvent) => {
      if (event.key !== "Tab") return;
      const container = containerRef.current;
      if (!container) return;
      const focusables = Array.from(
        container.querySelectorAll<HTMLElement>(
          "button, input, select, textarea, a[href], [tabindex]:not([tabindex='-1'])",
        ),
      ).filter((el) => !el.hasAttribute("disabled"));
      if (focusables.length === 0) return;
      const first = focusables[0];
      const last = focusables[focusables.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first.focus();
      }
    },
    [containerRef],
  );
}

interface LastExportInfo {
  formatId: ExportFormatId;
  filename: string;
  bytes: number;
  embedErrors: number;
  inlineSkips: number;
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
  embedContext,
  documentTitle,
  onEditorReady,
  onSaveRequested,
  documentStatus,
  debugId = "default",
}: RichTextEditorProps & { extensions: AnyExtension[] }) {
  const [, forceRefresh] = useState(0);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState("");
  // Iteration-3 M12: keyboard-navigable palette (roving active option).
  const [paletteActiveIndex, setPaletteActiveIndex] = useState(0);
  const [pendingCommand, setPendingCommand] = useState<EditorCommandDescriptor | null>(null);
  const [argValues, setArgValues] = useState<Record<string, string>>({});
  const paletteInputRef = useRef<HTMLInputElement>(null);
  // Iteration-3 M13: dialog containers for focus trap + restore.
  const paletteRef = useRef<HTMLDivElement>(null);
  const argPromptRef = useRef<HTMLDivElement>(null);
  const exportMenuRef = useRef<HTMLDivElement>(null);
  const linePromptRef = useRef<HTMLDivElement>(null);
  // Iteration-3 M16: toolbar roving tabindex (single tab stop + arrow keys).
  const toolbarRef = useRef<HTMLDivElement>(null);
  const [toolbarFocusIndex, setToolbarFocusIndex] = useState(0);

  // MT-244: find/replace panel + export menu state.
  const [findPanel, setFindPanel] = useState<"closed" | "find" | "replace">("closed");
  const [exportMenuOpen, setExportMenuOpen] = useState(false);
  // Iteration-3 L13: the overflow (insert/table-edit) menu is a real surface.
  const [overflowOpen, setOverflowOpen] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);
  const [lastExport, setLastExport] = useState<LastExportInfo | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [linePromptOpen, setLinePromptOpen] = useState(false);
  const [lineInput, setLineInput] = useState("");
  const [goToLineError, setGoToLineError] = useState<string | null>(null);
  const lineInputValueRef = useRef("");
  const lastFocusedCodeBlockRef = useRef<ReturnType<typeof findFocusedCodeBlock>>(null);
  const paletteCodeBlockTargetRef = useRef<ReturnType<typeof findFocusedCodeBlock>>(null);
  const goToLineTargetRef = useRef<ReturnType<typeof findFocusedCodeBlock>>(null);
  const [monacoCursor, setMonacoCursor] = useState<MonacoCursorSnapshot | null>(null);

  const [debugSnapshot, setDebugSnapshot] = useState<EditorDebugSnapshot | null>(null);

  // Iteration-3 H1 (echo-loop caret teleport): remember the exact JSON object
  // this editor last EMITTED through onChange. Parents (RichDocumentView, the
  // offline harness) store that object and pass it straight back down as
  // initialContent; without this guard the reload effect re-fed every keystroke
  // into setContent — an O(doc) re-parse that teleported the caret to the
  // document end (adversarial review: caret 6 -> 12/13) and broke IME.
  const lastEmittedRef = useRef<JSONContent | null>(null);
  // Keep the latest onChange without rebinding the editor's onUpdate closure.
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  const editor = useEditor({
    extensions,
    content: initialContent ?? { type: "doc", content: [{ type: "paragraph" }] },
    editable: !readOnly,
    onUpdate: ({ editor }) => {
      const json = editor.getJSON();
      lastEmittedRef.current = json;
      onChangeRef.current(json);
    },
  });

  // Reload content ONLY when the upstream document genuinely changed
  // (load/reload/conflict resolution) — never for the editor's own echo:
  //   1. identity guard: the parent passed back the object we just emitted;
  //   2. structural guard: a clone of the current document (e.g. a backend
  //      round-trip that changed nothing) must not replace the doc under the
  //      caret. A real external update still applies.
  useEffect(() => {
    if (!editor || editor.isDestroyed) return;
    if (initialContent !== null && initialContent === lastEmittedRef.current) return;
    const next = initialContent ?? { type: "doc", content: [{ type: "paragraph" }] };
    if (jsonDeepEquals(editor.getJSON(), next)) return;
    editor.commands.setContent(next, { emitUpdate: false });
    // The document is now externally owned; the previous emit no longer
    // describes the editor state.
    lastEmittedRef.current = null;
  }, [editor, initialContent]);

  // Reflect read-only changes (schema fail-closed, conflict lockout) onto the
  // live editor — useEditor only applies `editable` at construction.
  useEffect(() => {
    if (!editor || editor.isDestroyed) return;
    if (editor.isEditable === readOnly) editor.setEditable(!readOnly, false);
  }, [editor, readOnly]);

  // Hand the live editor instance to the parent (document shell / tests).
  useEffect(() => {
    if (editor && onEditorReady) onEditorReady(editor);
  }, [editor, onEditorReady]);

  // Publishes the machine-readable visual-debug snapshot (MT-172) on the
  // globals and on component state so the visual lane / a no-context model can
  // read it. Iteration-3 M15: gated behind the debug switch (the walk is
  // O(doc) per transaction — ON in dev/test and when the harness/operator
  // enables it, OFF in production bundles); L19: also published under the
  // per-editor id namespace so parallel editors stay attributable.
  const publishDebug = useCallback(() => {
    if (!editor || editor.isDestroyed) return;
    if (!isEditorDebugEnabled()) return;
    const snapshot = buildEditorDebugSnapshot(editor);
    setDebugSnapshot(snapshot);
    publishEditorDebugSnapshot(snapshot, debugId);
  }, [editor, debugId]);

  const rememberFocusedCodeBlock = useCallback(() => {
    if (!editor || editor.isDestroyed) return null;
    const focused = findFocusedCodeBlock(editor.state.doc, editor.state.selection.from);
    if (focused) lastFocusedCodeBlockRef.current = focused;
    else lastFocusedCodeBlockRef.current = null;
    return focused;
  }, [editor]);

  useEffect(() => {
    if (!editor) return;
    const root = editor.view.dom;
    const onMonacoStatus = (event: Event) => {
      const detail = (event as CustomEvent<Partial<MonacoCursorSnapshot>>).detail;
      if (!detail || typeof detail.pos !== "number") return;
      if (!detail.focused) {
        setMonacoCursor((current) => (current?.pos === detail.pos ? null : current));
        return;
      }
      setMonacoCursor({
        focused: true,
        pos: detail.pos,
        language: String(detail.language ?? "plaintext"),
        line: typeof detail.line === "number" ? detail.line : 1,
        column: typeof detail.column === "number" ? detail.column : 1,
      });
    };
    root.addEventListener(MONACO_CODE_BLOCK_STATUS_EVENT, onMonacoStatus);
    return () => root.removeEventListener(MONACO_CODE_BLOCK_STATUS_EVENT, onMonacoStatus);
  }, [editor]);

  const currentFocusedCodeBlock = useCallback(() => {
    if (!editor || editor.isDestroyed) return null;
    if (monacoCursor?.focused) {
      return findFocusedCodeBlock(editor.state.doc, monacoCursor.pos);
    }
    return rememberFocusedCodeBlock();
  }, [editor, monacoCursor, rememberFocusedCodeBlock]);

  // Toolbar active-state refresh + actor-attributed selection snapshot (MT-171)
  // + visual-debug publication (MT-172). Iteration-3 M15: selectionUpdate and
  // transaction fire together on most edits — the refresh/publish work is
  // coalesced into ONE microtask per tick instead of two synchronous
  // re-renders + two O(doc) walks per keystroke.
  useEffect(() => {
    if (!editor) return;
    let refreshQueued = false;
    let disposed = false;
    const scheduleRefresh = () => {
      if (refreshQueued) return;
      refreshQueued = true;
      queueMicrotask(() => {
        refreshQueued = false;
        if (disposed || editor.isDestroyed) return;
        rememberFocusedCodeBlock();
        forceRefresh((t) => t + 1);
        publishDebug();
      });
    };
    const onSelection = () => {
      scheduleRefresh();
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
      scheduleRefresh();
    };
    editor.on("selectionUpdate", onSelection);
    editor.on("transaction", onTx);
    // Initial publish via the same coalesced path.
    scheduleRefresh();
    return () => {
      disposed = true;
      editor.off("selectionUpdate", onSelection);
      editor.off("transaction", onTx);
    };
  }, [editor, onSelectionSnapshot, actor, presencePolicy, publishDebug, rememberFocusedCodeBlock]);

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
      paletteCodeBlockTargetRef.current = null;
      setPendingCommand(null);
    },
    [editor],
  );

  // MT-244: runs one save-to-format export end to end — build the projection
  // from the live document, download it quietly, surface the outcome (typed
  // inline error on failure, never silent), and publish a machine-readable
  // result for the visual lane.
  const runExport = useCallback(
    async (formatId: ExportFormatId) => {
      if (!editor || exportBusy) return;
      const descriptor = EXPORT_FORMATS.find((format) => format.id === formatId);
      if (!descriptor) return;
      setExportBusy(true);
      setExportError(null);
      try {
        const docJson = editor.getJSON();
        const title = documentTitle ?? "handshake-document";
        let content: string;
        let embedErrors = 0;
        let inlineSkips = 0;
        if (formatId === "html_self_contained" || formatId === "html_reference_linked") {
          const result = await exportHtml(docJson, {
            mode: formatId === "html_self_contained" ? "self_contained" : "reference_linked",
            title,
            embedContext: embedContext ?? null,
          });
          content = result.html;
          embedErrors = result.embedErrors.length;
          inlineSkips = result.inlineSkips.length;
        } else if (formatId === "markdown") {
          content = exportMarkdown(docJson);
        } else if (formatId === "plain_text") {
          content = exportPlainText(docJson);
        } else {
          content = exportProseMirrorJson(docJson);
        }
        const filename = buildExportFilename(title, formatId);
        const bytes = saveTextToFile(filename, content, descriptor.mimeType);
        const info: LastExportInfo = { formatId, filename, bytes, embedErrors, inlineSkips };
        setLastExport(info);
        (globalThis as Record<string, unknown>)[EDITOR_LAST_EXPORT_GLOBAL_KEY] = {
          ...info,
          content,
        };
      } catch (error) {
        setExportError(error instanceof Error ? error.message : String(error));
      } finally {
        setExportBusy(false);
        setExportMenuOpen(false);
      }
    },
    [editor, exportBusy, documentTitle, embedContext],
  );

  // MT-244: component-level command dispatch (find/replace/export surfaces).
  const runComponentCommand = useCallback(
    (id: string) => {
      const paletteTarget = paletteCodeBlockTargetRef.current;
      paletteCodeBlockTargetRef.current = null;
      setPaletteOpen(false);
      setPaletteQuery("");
      if (id === FIND_OPEN_ACTION) {
        setFindPanel("find");
        return;
      }
      if (id === REPLACE_OPEN_ACTION) {
        setFindPanel("replace");
        return;
      }
      if (id === GOTO_LINE_ACTION) {
        goToLineTargetRef.current = currentFocusedCodeBlock() ?? paletteTarget;
        setGoToLineError(null);
        lineInputValueRef.current = "";
        setLineInput("");
        setLinePromptOpen(true);
        return;
      }
      if (id.startsWith("export.")) {
        void runExport(id.slice("export.".length) as ExportFormatId);
      }
    },
    [runExport, currentFocusedCodeBlock],
  );

  const runGoToLine = useCallback(() => {
    if (!editor || editor.isDestroyed) return;
    const rawLine = lineInputValueRef.current.trim();
    const requested = Number(rawLine);
    const focused = goToLineTargetRef.current;
    if (!focused) {
      setGoToLineError("No focused code block. Place the cursor in a code block before running Go to line.");
      return;
    }
    const range = codeLineRange(focused.code, requested);
    if (!range) {
      setGoToLineError(`Line ${rawLine || "(blank)"} is outside this code block.`);
      return;
    }
    const revealed = revealCodeBlockRange(focused.pos, range.start, range.end);
    if (!revealed) {
      setGoToLineError("The focused code block is not mounted yet; open it and retry.");
      return;
    }
    setGoToLineError(null);
    setLinePromptOpen(false);
    goToLineTargetRef.current = null;
    lineInputValueRef.current = "";
    setLineInput("");
  }, [editor]);

  // Keyboard: explicit keymap (MT-170 + MT-244). Palette/find/replace +
  // bound commands. Iteration-3 hardening (H3):
  //   - defaultPrevented guard: ProseMirror's native keymap (StarterKit Mod-b/
  //     i/e) runs first and preventDefaults; re-dispatching here would toggle
  //     the mark twice (a net no-op for the operator).
  //   - code-block containment: keystrokes originating inside the embedded
  //     Monaco code block belong to Monaco — only explicitly global chords
  //     (command palette) may fire from there. Without this, Mod-Alt-c/Mod-k
  //     typed in code reached the prose handler and could REPLACE the
  //     node-selected code block.
  //   - IME: resolveShortcut returns null while event.isComposing (L15).
  useEffect(() => {
    if (!editor) return;
    const handler = (event: KeyboardEvent) => {
      if (event.defaultPrevented) return;
      const action = resolveShortcut(event);
      if (!action) return;
      const origin = event.target instanceof Element ? event.target : null;
      if (origin?.closest("[data-testid='monaco-code-block']") && !isGlobalEditorAction(action)) {
        return;
      }
      event.preventDefault();
      if (action === PALETTE_OPEN_ACTION) {
        paletteCodeBlockTargetRef.current = currentFocusedCodeBlock();
        setPaletteOpen(true);
        return;
      }
      if (action === FIND_OPEN_ACTION || action === REPLACE_OPEN_ACTION) {
        setFindPanel(action === FIND_OPEN_ACTION ? "find" : "replace");
        return;
      }
      if (action === SAVE_ACTION) {
        // Always swallowed (no browser save dialog); fires the parent's save
        // path when wired (L16/EXT-SAVE-001). Global from inside code blocks.
        onSaveRequested?.();
        return;
      }
      const cmd = EDITOR_COMMAND_BY_ID.get(action);
      if (cmd) runCommand(cmd);
    };
    const root = editor.view.dom;
    root.addEventListener("keydown", handler);
    return () => root.removeEventListener("keydown", handler);
  }, [editor, runCommand, onSaveRequested, currentFocusedCodeBlock]);

  // Iteration-3 M13: focus trap + restore for every modal surface. These hooks
  // are declared BEFORE the focus-moving effects below so the previously
  // focused element is captured before the dialog steals focus.
  const onPaletteTrapKeyDown = useDialogFocus(paletteOpen, paletteRef);
  const onArgPromptTrapKeyDown = useDialogFocus(pendingCommand !== null, argPromptRef);
  const onExportTrapKeyDown = useDialogFocus(exportMenuOpen, exportMenuRef);
  const onLinePromptTrapKeyDown = useDialogFocus(linePromptOpen, linePromptRef);

  useEffect(() => {
    if (paletteOpen) paletteInputRef.current?.focus();
  }, [paletteOpen]);
  useEffect(() => {
    if (pendingCommand) {
      argPromptRef.current?.querySelector<HTMLInputElement>("input")?.focus();
    }
  }, [pendingCommand]);
  useEffect(() => {
    if (exportMenuOpen) {
      exportMenuRef.current?.querySelector<HTMLButtonElement>("button")?.focus();
    }
  }, [exportMenuOpen]);
  useEffect(() => {
    if (linePromptOpen) {
      linePromptRef.current?.querySelector<HTMLInputElement>("input")?.focus();
    }
  }, [linePromptOpen]);

  // Iteration-3 M16: ARIA toolbar keyboard pattern — one tab stop, arrows move
  // between ENABLED controls; the roving index is stored in full-list terms so
  // it stays aligned with the per-button tabIndex assignment.
  const onToolbarKeyDown = useCallback((event: ReactKeyboardEvent) => {
    if (!["ArrowRight", "ArrowLeft", "Home", "End"].includes(event.key)) return;
    const container = toolbarRef.current;
    if (!container) return;
    const all = Array.from(container.querySelectorAll<HTMLButtonElement>("button"));
    const enabled = all.filter((b) => !b.disabled);
    if (enabled.length === 0) return;
    const current = enabled.indexOf(document.activeElement as HTMLButtonElement);
    let next = current;
    if (event.key === "ArrowRight") next = (current + 1 + enabled.length) % enabled.length;
    else if (event.key === "ArrowLeft") next = (current - 1 + enabled.length) % enabled.length;
    else if (event.key === "Home") next = 0;
    else if (event.key === "End") next = enabled.length - 1;
    event.preventDefault();
    const target = enabled[next];
    target?.focus();
    if (target) setToolbarFocusIndex(all.indexOf(target));
  }, []);

  if (!editor) return null;

  const outline = buildEditorOutline(editor.state.doc);
  const chromeStatus = buildEditorStatus(editor, documentStatus, monacoCursor);
  const toolbarCommands = EDITOR_COMMANDS.filter((c) =>
    ["history", "format", "block", "list", "table", "code", "link"].includes(c.category),
  );
  // Iteration-3 L13/L12: the overflow set (insert + table-structure commands)
  // is a REAL operator-reachable menu now, not a hidden stub.
  const overflowCommands = EDITOR_COMMANDS.filter((c) =>
    ["tableEdit", "embed", "graph", "mention", "manual"].includes(c.category),
  );
  // Iteration-3 M11: truthful enable/disable from the catalog's canRun.
  const commandDisabled = (cmd: EditorCommandDescriptor): boolean =>
    readOnly || (cmd.canRun ? !cmd.canRun(editor) : false);
  // M16: the roving tab stop must always sit on an ENABLED control (e.g. Undo
  // is disabled until the first edit) so the toolbar never loses its tab stop.
  const toolbarDisabledFlags = toolbarCommands.map((cmd) => commandDisabled(cmd));
  let effectiveToolbarFocus = toolbarFocusIndex;
  if (
    effectiveToolbarFocus < toolbarDisabledFlags.length &&
    toolbarDisabledFlags[effectiveToolbarFocus]
  ) {
    const firstEnabled = toolbarDisabledFlags.findIndex((d) => !d);
    effectiveToolbarFocus = firstEnabled >= 0 ? firstEnabled : toolbarDisabledFlags.length;
  }
  const paletteResults = filterEditorCommands(paletteQuery);
  const componentResults = filterComponentCommands(
    paletteQuery,
    onSaveRequested
      ? [{ id: SAVE_ACTION, label: "Save document", keywords: ["save", "persist", "write"] }]
      : [],
  );
  // Combined keyboard-navigable option order (M12): editor commands first,
  // then component commands — matching the rendered list order.
  const paletteOptionIds = [
    ...paletteResults.map((cmd) => cmd.id),
    ...componentResults.map((cmd) => cmd.id),
  ];
  const activeOptionId =
    paletteOptionIds.length > 0
      ? `palette-opt-${paletteOptionIds[Math.min(paletteActiveIndex, paletteOptionIds.length - 1)]}`
      : undefined;

  const runPaletteIndex = (index: number) => {
    const clamped = Math.min(index, paletteOptionIds.length - 1);
    if (clamped < 0) return;
    if (clamped < paletteResults.length) {
      runCommand(paletteResults[clamped]);
    } else {
      const component = componentResults[clamped - paletteResults.length];
      if (component.id === SAVE_ACTION) {
        setPaletteOpen(false);
        setPaletteQuery("");
        paletteCodeBlockTargetRef.current = null;
        onSaveRequested?.();
        return;
      }
      runComponentCommand(component.id);
    }
  };

  const onPaletteInputKeyDown = (event: ReactKeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Escape") {
      setPaletteOpen(false);
      setPendingCommand(null);
      paletteCodeBlockTargetRef.current = null;
      return;
    }
    if (paletteOptionIds.length === 0) return;
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setPaletteActiveIndex((i) => (i + 1) % paletteOptionIds.length);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setPaletteActiveIndex((i) => (i - 1 + paletteOptionIds.length) % paletteOptionIds.length);
    } else if (event.key === "Home" && paletteQuery.length === 0) {
      event.preventDefault();
      setPaletteActiveIndex(0);
    } else if (event.key === "End" && paletteQuery.length === 0) {
      event.preventDefault();
      setPaletteActiveIndex(paletteOptionIds.length - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      runPaletteIndex(paletteActiveIndex);
    }
  };

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

      <nav
        className="rich-text-editor__outline"
        aria-label="Document outline"
        data-testid="rich-text-editor-outline"
        data-outline-count={String(outline.length)}
      >
        <div className="rich-text-editor__outline-title">Outline</div>
        {outline.length === 0 ? (
          <div className="rich-text-editor__outline-empty muted" data-testid="rich-text-editor-outline-empty">
            No headings.
          </div>
        ) : (
          <ol className="rich-text-editor__outline-list">
            {outline.map((item, index) => (
              <li key={item.id}>
                <button
                  type="button"
                  className="rich-text-editor__outline-item"
                  data-testid="rich-text-editor-outline-item"
                  data-outline-id={item.id}
                  data-outline-level={String(item.level)}
                  data-pos={String(item.pos)}
                  data-selection-pos={String(item.selectionPos)}
                  data-empty={item.empty ? "true" : "false"}
                  style={{ paddingLeft: `${Math.max(0, item.level - 1) * 12}px` }}
                  onClick={() => {
                    editor.commands.setTextSelection(item.selectionPos);
                    editor.commands.scrollIntoView();
                    const heading = editor.view.dom.querySelectorAll("h1,h2,h3,h4,h5,h6")[index];
                    heading?.scrollIntoView?.({ block: "center" });
                  }}
                >
                  {item.text}
                </button>
              </li>
            ))}
          </ol>
        )}
      </nav>

      {/* Toolbar (MT-169) — labelled, keyboard-reachable (MT-173). Iteration-3
          M16: ARIA toolbar keyboard pattern — ONE tab stop with roving
          tabindex; ArrowLeft/Right/Home/End move focus between controls. */}
      <div
        ref={toolbarRef}
        className="rich-text-editor__toolbar"
        role="toolbar"
        aria-label="Editor formatting"
        data-testid="rich-text-editor-toolbar"
        onKeyDown={onToolbarKeyDown}
      >
        {toolbarCommands.map((cmd, index) => (
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
            disabled={commandDisabled(cmd)}
            tabIndex={!commandDisabled(cmd) && index === effectiveToolbarFocus ? 0 : -1}
            onFocus={() => setToolbarFocusIndex(index)}
            onClick={() => runCommand(cmd)}
          >
            {cmd.label}
          </button>
        ))}
        <button
          type="button"
          className="tt-button"
          data-testid="editor-open-overflow"
          aria-label="Insert and table commands"
          aria-haspopup="menu"
          aria-expanded={overflowOpen}
          title="Insert and table commands"
          tabIndex={toolbarCommands.length === effectiveToolbarFocus ? 0 : -1}
          onFocus={() => setToolbarFocusIndex(toolbarCommands.length)}
          onClick={() => setOverflowOpen((open) => !open)}
          disabled={readOnly}
        >
          Insert…
        </button>
        <button
          type="button"
          className="tt-button"
          data-testid="editor-open-find"
          aria-label="Find in document (Ctrl/Cmd+F)"
          title="Find in document (Ctrl/Cmd+F)"
          tabIndex={toolbarCommands.length + 1 === effectiveToolbarFocus ? 0 : -1}
          onFocus={() => setToolbarFocusIndex(toolbarCommands.length + 1)}
          onClick={() => setFindPanel((open) => (open === "closed" ? "find" : "closed"))}
        >
          Find
        </button>
        <button
          type="button"
          className="tt-button"
          data-testid="editor-open-export"
          aria-label="Export document (save to format)"
          title="Export document (save to format)"
          tabIndex={toolbarCommands.length + 1 === effectiveToolbarFocus ? 0 : -1}
          onFocus={() => setToolbarFocusIndex(toolbarCommands.length + 1)}
          onClick={() => setExportMenuOpen(true)}
        >
          Export…
        </button>
        <button
          type="button"
          className="tt-button"
          data-testid="editor-open-palette"
          aria-label="Open command palette (more actions)"
          title="More actions (Ctrl/Cmd+P)"
          tabIndex={toolbarCommands.length + 2 === effectiveToolbarFocus ? 0 : -1}
          onFocus={() => setToolbarFocusIndex(toolbarCommands.length + 2)}
          onClick={() => {
            paletteCodeBlockTargetRef.current = currentFocusedCodeBlock();
            setPaletteOpen(true);
          }}
        >
          More…
        </button>
      </div>

      {/* Document-wide find/replace (MT-244): prose + code blocks. */}
      {findPanel !== "closed" && (
        <FindReplacePanel
          editor={editor}
          withReplace={findPanel === "replace" && !readOnly}
          onClose={() => setFindPanel("closed")}
        />
      )}

      {/* The editing surface. */}
      <div className="rich-text-editor__surface tiptap-scroll" data-testid="rich-text-editor-surface">
        <EditorContent editor={editor} />
      </div>

      <div
        className="rich-text-editor__status-bar"
        data-testid="rich-text-editor-status-bar"
        data-save-state={chromeStatus.saveState}
        data-word-count={String(chromeStatus.wordCount)}
        data-cursor-line={String(chromeStatus.cursor.line)}
        data-cursor-column={String(chromeStatus.cursor.column)}
        data-code-language={chromeStatus.focusedCodeLanguage ?? ""}
        data-editable={chromeStatus.editable ? "true" : "false"}
      >
        <span data-testid="rich-text-editor-status-cursor">
          Ln {chromeStatus.cursor.line}, Col {chromeStatus.cursor.column}
        </span>
        <span data-testid="rich-text-editor-status-language">
          {chromeStatus.focusedCodeLanguage ? chromeStatus.focusedCodeLanguage : "Prose"}
        </span>
        <span data-testid="rich-text-editor-status-save">
          {saveStateLabel(chromeStatus.saveState)}
          {chromeStatus.lastSavedAt ? ` at ${chromeStatus.lastSavedAt}` : ""}
        </span>
        <span data-testid="rich-text-editor-status-words">
          {chromeStatus.wordCount} word{chromeStatus.wordCount === 1 ? "" : "s"}
        </span>
      </div>

      {/* Save-to-format export menu (MT-244 / DEC-003). Iteration-3 M13:
          focus trap + Escape close + focus restore. */}
      {exportMenuOpen && (
        <div
          ref={exportMenuRef}
          className="rich-text-editor__export-menu"
          role="dialog"
          aria-label="Export document"
          aria-modal="true"
          data-testid="editor-export-menu"
          data-export-busy={exportBusy ? "true" : "false"}
          onKeyDown={(e) => {
            if (e.key === "Escape" && !exportBusy) {
              setExportMenuOpen(false);
              return;
            }
            onExportTrapKeyDown(e);
          }}
        >
          <h4>Export document</h4>
          <p className="muted small">
            Exports are projections — the saved Handshake document stays the authority.
          </p>
          <ul className="rich-text-editor__export-list">
            {EXPORT_FORMATS.map((format) => (
              <li key={format.id}>
                <button
                  type="button"
                  className="rich-text-editor__export-item"
                  data-testid={`export-format-${format.id}`}
                  disabled={exportBusy}
                  onClick={() => void runExport(format.id)}
                >
                  <span>{format.label}</span>
                  <span className="muted small">.{format.extension}</span>
                </button>
              </li>
            ))}
          </ul>
          <button
            type="button"
            data-testid="editor-export-cancel"
            disabled={exportBusy}
            onClick={() => setExportMenuOpen(false)}
          >
            Cancel
          </button>
        </div>
      )}

      {/* Export outcome (typed, machine-readable; never silent). */}
      {exportError && (
        <div className="rich-text-editor__export-error" role="alert" data-testid="export-error">
          Export failed: {exportError}
        </div>
      )}
      {lastExport && !exportError && (
        <div
          className="rich-text-editor__export-status muted"
          data-testid="export-status"
          data-export-format={lastExport.formatId}
          data-export-bytes={String(lastExport.bytes)}
          data-export-embed-errors={String(lastExport.embedErrors)}
          data-export-inline-skips={String(lastExport.inlineSkips)}
        >
          Exported {lastExport.filename} ({lastExport.bytes} bytes
          {lastExport.embedErrors > 0 ? `, ${lastExport.embedErrors} embed error(s)` : ""}
          {lastExport.inlineSkips > 0 ? `, ${lastExport.inlineSkips} inline skip(s)` : ""}).
        </div>
      )}

      {/* Command palette (MT-170) — overflow + searchable all-command surface.
          Iteration-3 M12/M13: ArrowUp/Down + Enter keyboard navigation with
          valid listbox/option semantics (aria-activedescendant) and a focus
          trap that restores focus on close. */}
      {paletteOpen && (
        <div
          ref={paletteRef}
          className="rich-text-editor__palette"
          role="dialog"
          aria-label="Command palette"
          aria-modal="true"
          data-testid="editor-command-palette"
          onKeyDown={onPaletteTrapKeyDown}
        >
          <input
            ref={paletteInputRef}
            type="text"
            className="rich-text-editor__palette-input"
            data-testid="editor-command-palette-input"
            aria-label="Search editor commands"
            placeholder="Search commands…"
            role="combobox"
            aria-expanded="true"
            aria-controls="editor-command-palette-listbox"
            aria-activedescendant={activeOptionId}
            value={paletteQuery}
            onChange={(e) => {
              setPaletteQuery(e.target.value);
              setPaletteActiveIndex(0);
            }}
            onKeyDown={onPaletteInputKeyDown}
          />
          <ul
            id="editor-command-palette-listbox"
            className="rich-text-editor__palette-list"
            role="listbox"
            aria-label="Matching commands"
            data-testid="editor-command-palette-list"
          >
            {paletteResults.map((cmd, index) => (
              <li
                key={cmd.id}
                id={`palette-opt-${cmd.id}`}
                role="option"
                aria-selected={index === paletteActiveIndex}
                data-palette-active={index === paletteActiveIndex ? "true" : "false"}
              >
                <button
                  type="button"
                  tabIndex={-1}
                  className="rich-text-editor__palette-item"
                  data-testid={`palette-cmd-${cmd.id}`}
                  data-command-id={cmd.id}
                  disabled={commandDisabled(cmd)}
                  onClick={() => runCommand(cmd)}
                >
                  <span>{cmd.label}</span>
                  <span className="muted small">{paletteHint(cmd)}</span>
                </button>
              </li>
            ))}
            {/* Component-level surfaces: find/replace + export (MT-244) + save. */}
            {componentResults.map((cmd, index) => (
              <li
                key={cmd.id}
                id={`palette-opt-${cmd.id}`}
                role="option"
                aria-selected={paletteResults.length + index === paletteActiveIndex}
                data-palette-active={
                  paletteResults.length + index === paletteActiveIndex ? "true" : "false"
                }
              >
                <button
                  type="button"
                  tabIndex={-1}
                  className="rich-text-editor__palette-item"
                  data-testid={`palette-cmd-${cmd.id}`}
                  data-command-id={cmd.id}
                  onClick={() => runPaletteIndex(paletteResults.length + index)}
                >
                  <span>{cmd.label}</span>
                  <span className="muted small">{bindingsForAction(cmd.id)[0]?.chord ?? "editor"}</span>
                </button>
              </li>
            ))}
            {paletteResults.length === 0 && componentResults.length === 0 && (
              <li className="muted" data-testid="editor-command-palette-empty">
                No matching commands.
              </li>
            )}
          </ul>
        </div>
      )}

      {/* Arg prompt for commands needing input (link target, language, …).
          Iteration-3 M13: focus lands on the first field, Tab is trapped,
          Escape cancels, and focus restores on close. */}
      {pendingCommand && (
        <div
          ref={argPromptRef}
          className="rich-text-editor__arg-prompt"
          role="dialog"
          aria-label={`${pendingCommand.label} options`}
          aria-modal="true"
          data-testid="editor-arg-prompt"
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setPendingCommand(null);
              return;
            }
            onArgPromptTrapKeyDown(e);
          }}
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

      {linePromptOpen && (
        <div
          ref={linePromptRef}
          className="rich-text-editor__arg-prompt"
          role="dialog"
          aria-label="Go to line options"
          aria-modal="true"
          data-testid="editor-go-to-line-prompt"
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              setLinePromptOpen(false);
              goToLineTargetRef.current = null;
              lineInputValueRef.current = "";
              return;
            }
            onLinePromptTrapKeyDown(e);
          }}
        >
          <h4>Go to line</h4>
          <label className="rich-text-editor__arg-field">
            <span className="muted small">Line</span>
            <input
              type="text"
              inputMode="numeric"
              data-testid="editor-arg-line"
              value={lineInput}
              onChange={(e) => {
                lineInputValueRef.current = e.target.value;
                setLineInput(e.target.value);
              }}
            />
          </label>
          <div className="rich-text-editor__arg-actions">
            <button type="button" data-testid="editor-arg-confirm" onClick={runGoToLine}>
              Go
            </button>
            <button
              type="button"
              data-testid="editor-arg-cancel"
              onClick={() => {
                lineInputValueRef.current = "";
                goToLineTargetRef.current = null;
                setLinePromptOpen(false);
              }}
            >
              Cancel
            </button>
          </div>
        </div>
      )}

      {goToLineError && (
        <div
          className="rich-text-editor__goto-line-error"
          role="alert"
          data-testid="editor-go-to-line-error"
        >
          {goToLineError}
        </div>
      )}

      {/* Overflow menu (iteration-3 L13): a REAL operator-reachable menu for
          insert + table-structure commands — no longer a hidden stub. */}
      {overflowOpen && (
        <div
          className="rich-text-editor__overflow"
          role="menu"
          aria-label="More editor commands"
          data-testid="rich-text-editor-overflow"
        >
          {overflowCommands.map((cmd) => (
            <button
              key={cmd.id}
              type="button"
              role="menuitem"
              className="rich-text-editor__overflow-item"
              data-testid={`overflow-cmd-${cmd.id}`}
              data-command-id={cmd.id}
              disabled={commandDisabled(cmd)}
              onClick={() => {
                setOverflowOpen(false);
                runCommand(cmd);
              }}
            >
              {cmd.label}
            </button>
          ))}
          <button
            type="button"
            data-testid="overflow-close"
            onClick={() => setOverflowOpen(false)}
          >
            Close
          </button>
        </div>
      )}
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
    case "integrity":
      return "Code integrity check failed";
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

function saveStateLabel(state: "saved" | "dirty" | "saving" | "blocked" | "conflict" | "error"): string {
  switch (state) {
    case "saving":
      return "Saving";
    case "blocked":
      return "Save blocked";
    case "conflict":
      return "Conflict";
    case "error":
      return "Save error";
    case "dirty":
      return "Unsaved";
    case "saved":
      return "Saved";
  }
}
