// WP-KERNEL-009 / MT-165 — MonacoEmbeddedCodeBlock NodeView.
//
// The React NodeView that mounts a REAL Monaco editor inside a Tiptap code-block
// node (the operator's in-document VS Code replacement). It:
//   - mounts Monaco via the single bundled entry point (createConfiguredEditor
//     in app/src/lib/monaco/setup.ts — bundled workers, offline, no CDN),
//   - is language-aware (a language picker over the curated MT-166 registry;
//     changing it re-tags the node + the Monaco model),
//   - writes code + language + round-trip hash back into the node attrs on every
//     edit (MT-168 bridge), so the whole document content_json is one authority
//     record persisted through the rich-doc save API,
//   - degrades gracefully: if Monaco fails to mount (e.g. a headless/jsdom
//     context, or a bundled-worker failure), it reports a typed dependency
//     failure (DependencyFailureBanner surfaces it) AND keeps an editable
//     <textarea> fallback so the code is never lost or shown on a blank surface.
//
// Stable selectors for visual debug (MT-172): data-testid="monaco-code-block",
// data-language, data-rt-hash, data-monaco-mounted; the language picker is
// data-testid="monaco-code-block-language".

import { useEffect, useRef, useState } from "react";
import { NodeViewWrapper, type ReactNodeViewProps } from "@tiptap/react";
import { createConfiguredEditor, monaco } from "../lib/monaco/setup";
import { SnippetController2 } from "monaco-editor/esm/vs/editor/contrib/snippet/browser/snippetController2.js";
import {
  HANDSHAKE_CODE_LANGUAGES,
  DEFAULT_CODE_LANGUAGE,
} from "../lib/monaco/language_registry";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";
import { EDITOR_SNIPPETS, monacoSnippetTemplateForId } from "../lib/editor/editor_snippets";
import { registerCodeBlockFindHandle } from "../lib/editor/code_block_find_registry";
import {
  dependencyFailures,
  formatDependencyFailureMessage,
} from "../lib/dependency_policy/dependency_failure";
import {
  installHandshakeCodeIntelligenceEditorActions,
  refreshHandshakeCodeIntelligenceMarkers,
} from "../lib/monaco/code_intelligence";

type Editor = monaco.editor.IStandaloneCodeEditor;

const HANDSHAKE_MONACO_LIGHT_THEME = "handshake-light";
const HANDSHAKE_MONACO_DARK_THEME = "handshake-dark";

function currentWorkspaceMonacoTheme() {
  const workspaceTheme = document.getElementById("main-window")?.getAttribute("data-theme");
  return workspaceTheme === "dark" ? HANDSHAKE_MONACO_DARK_THEME : HANDSHAKE_MONACO_LIGHT_THEME;
}

export function MonacoCodeBlockView(props: ReactNodeViewProps<HTMLElement>) {
  const { node, updateAttributes, editor, getPos } = props;
  const language = String(node.attrs.language || DEFAULT_CODE_LANGUAGE);
  const code = String(node.attrs.code ?? "");
  const rtHash = String(node.attrs.contentHash ?? "");

  const hostRef = useRef<HTMLDivElement>(null);
  const fallbackRef = useRef<HTMLTextAreaElement>(null);
  const editorRef = useRef<Editor | null>(null);
  const [mounted, setMounted] = useState(false);
  const [degraded, setDegraded] = useState(false);
  // Iteration-3 L7 (lazy mount): a full Monaco instance per code block at
  // parse time made large documents pay the whole boot cost upfront. The
  // editor now mounts when the block approaches the viewport (generous
  // rootMargin), immediately when IntersectionObserver is unavailable, or
  // on demand when document-wide find needs to reveal a match inside it.
  const [shouldMount, setShouldMount] = useState(false);
  const pendingRevealRef = useRef<{ start: number; end: number } | null>(null);
  // Guards re-entrant attr writes when WE set the model value programmatically.
  const applyingRef = useRef(false);
  // Iteration-3 H4: the Monaco mount effect runs ONCE, so its
  // onDidChangeContent closure captured the MOUNT-TIME language — after a
  // language switch every keystroke minted contentHash against the old
  // language, corrupting the MT-168 round-trip invariant. The ref always
  // carries the current language for the persistent listener.
  const languageRef = useRef(language);
  useEffect(() => {
    languageRef.current = language;
  }, [language]);

  const isEditable = editor.isEditable;

  // Iteration-3 M6/M17 (WCAG 2.1.2 keyboard trap): Escape returns focus from
  // the embedded code editor to the prose document, placing the caret right
  // AFTER this block so keyboard-only operators can continue writing. Monaco
  // keeps Escape for its own open widgets (find/suggest) via the keybinding
  // context below; the degraded textarea wires the same exit directly.
  const exitToProse = () => {
    const pos = typeof getPos === "function" ? getPos() : null;
    if (typeof pos !== "number") return;
    editor
      .chain()
      .focus()
      .setTextSelection(Math.min(pos + node.nodeSize, editor.state.doc.content.size))
      .run();
  };
  const exitToProseRef = useRef(exitToProse);
  exitToProseRef.current = exitToProse;

  // MT-244: register the find/replace reveal handle so document-wide find can
  // highlight + scroll a match INSIDE this code block (Monaco when mounted,
  // the degraded textarea otherwise). Unregisters on unmount.
  useEffect(() => {
    return registerCodeBlockFindHandle({
      getPos,
      reveal: (start, end) => {
        const instance = editorRef.current;
        const model = instance?.getModel();
        if (instance && model) {
          const from = model.getPositionAt(start);
          const to = model.getPositionAt(end);
          const range = {
            startLineNumber: from.lineNumber,
            startColumn: from.column,
            endLineNumber: to.lineNumber,
            endColumn: to.column,
          };
          instance.setSelection(range);
          instance.revealRangeInCenterIfOutsideViewport(range);
          return;
        }
        const fallback = fallbackRef.current;
        if (fallback) {
          fallback.focus();
          fallback.setSelectionRange(start, end);
          return;
        }
        // L7: the block has not lazily mounted yet (offscreen) — force the
        // mount and replay this reveal once Monaco is up.
        pendingRevealRef.current = { start, end };
        setShouldMount(true);
      },
    });
  }, [getPos]);

  // L7: viewport-driven mount trigger. Falls back to immediate mounting when
  // IntersectionObserver is unavailable (jsdom, very old webviews).
  useEffect(() => {
    if (shouldMount) return;
    const host = hostRef.current;
    if (!host || typeof IntersectionObserver === "undefined") {
      setShouldMount(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) {
          setShouldMount(true);
          observer.disconnect();
        }
      },
      { rootMargin: "600px" },
    );
    observer.observe(host);
    return () => observer.disconnect();
  }, [shouldMount]);

  // Mount Monaco once the lazy trigger fires (L7).
  useEffect(() => {
    if (!shouldMount) return;
    const host = hostRef.current;
    if (!host || editorRef.current) return;
    let instance: Editor | null = null;
    let codeIntelligenceActions: monaco.IDisposable | null = null;
    try {
      instance = createConfiguredEditor({
        container: host,
        value: code,
        language,
        readOnly: !isEditable,
        scrollBeyondLastLine: false,
        lineNumbers: "on",
        fontSize: 13,
        theme: currentWorkspaceMonacoTheme(),
      });
      editorRef.current = instance;
      codeIntelligenceActions = installHandshakeCodeIntelligenceEditorActions(instance);
      const model = instance.getModel();
      if (model) {
        model.onDidChangeContent(() => {
          if (applyingRef.current) return;
          const next = instance!.getValue();
          // H4/M10: read the CURRENT language (not the mount-time closure) and
          // mint attrs through the single minting point so the hash can never
          // drift from {language, code}.
          updateAttributes(makeCodeBlockAttrs(languageRef.current, next));
        });
      }
      // M6/M17: keyboard exit. The context expression leaves Escape to Monaco
      // while its own popups are open (find widget, suggestions, rename).
      instance.addCommand(
        monaco.KeyCode.Escape,
        () => exitToProseRef.current(),
        "!suggestWidgetVisible && !findWidgetVisible && !renameInputVisible",
      );
      // L7: replay a reveal that arrived before the lazy mount completed.
      const pendingReveal = pendingRevealRef.current;
      if (pendingReveal && model) {
        pendingRevealRef.current = null;
        const from = model.getPositionAt(pendingReveal.start);
        const to = model.getPositionAt(pendingReveal.end);
        const range = {
          startLineNumber: from.lineNumber,
          startColumn: from.column,
          endLineNumber: to.lineNumber,
          endColumn: to.column,
        };
        instance.setSelection(range);
        instance.revealRangeInCenterIfOutsideViewport(range);
      }
      setMounted(true);
    } catch (error) {
      // createConfiguredEditor already reported a typed dependency failure; we
      // additionally flip to the textarea fallback so the code stays editable.
      const failure = {
        dependency: "monaco-editor",
        component: "code-block-nodeview",
        phase: "editor_mount" as const,
        cause: error instanceof Error ? error.message : String(error),
      };
      dependencyFailures.report({ ...failure, message: formatDependencyFailureMessage(failure) });
      setDegraded(true);
    }
    return () => {
      codeIntelligenceActions?.dispose();
      instance?.dispose();
      editorRef.current = null;
    };
    // Mount-once (per lazy trigger): language/code changes are reconciled by
    // the effects below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [shouldMount]);

  useEffect(() => {
    if (!mounted) return undefined;
    const shell = document.getElementById("main-window");
    const applyTheme = () => monaco.editor.setTheme(currentWorkspaceMonacoTheme());
    applyTheme();
    if (!shell || typeof MutationObserver === "undefined") {
      return undefined;
    }
    const observer = new MutationObserver(applyTheme);
    observer.observe(shell, { attributes: true, attributeFilter: ["data-theme"] });
    return () => observer.disconnect();
  }, [mounted]);

  // Reconcile external code changes (undo/redo, collaborative edits, reload)
  // into the Monaco model without clobbering the user's caret on self-edits.
  //
  // Iteration-3 M5/M7: setValue() RESET Monaco's undo stack, caret, scroll and
  // folds on every external reconcile. pushEditOperations applies the change
  // as a regular edit (undo-able, caret-mapped) and the saved view state
  // restores scroll/selection. UNDO OWNERSHIP CONTRACT (M7): ProseMirror owns
  // DOCUMENT-level history (block add/remove + attr snapshots); Monaco owns
  // intra-block text history while focus is inside the code island. A PM undo
  // that reverts the code attr flows through this reconcile as one more
  // Monaco edit — the two stacks stay coherent instead of diverging.
  useEffect(() => {
    const instance = editorRef.current;
    if (!instance) return;
    if (instance.getValue() !== code) {
      applyingRef.current = true;
      const model = instance.getModel();
      if (model) {
        const viewState = instance.saveViewState();
        model.pushEditOperations(
          [],
          [{ range: model.getFullModelRange(), text: code }],
          () => null,
        );
        if (viewState) instance.restoreViewState(viewState);
      } else {
        instance.setValue(code);
      }
      applyingRef.current = false;
    }
  }, [code]);

  // Reconcile language changes into the Monaco model.
  useEffect(() => {
    const instance = editorRef.current;
    const model = instance?.getModel();
    if (model) monaco.editor.setModelLanguage(model, language);
  }, [language]);

  useEffect(() => {
    const instance = editorRef.current;
    if (!instance) return;
    void refreshHandshakeCodeIntelligenceMarkers(instance, monaco);
  }, [code, language, mounted]);

  // Reflect read-only state.
  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly: !isEditable });
  }, [isEditable]);

  // M10: every attr write goes through makeCodeBlockAttrs — the single minting
  // point that normalizes the language and computes the matching hash.
  const onLanguageChange = (nextLanguage: string) => {
    updateAttributes(makeCodeBlockAttrs(nextLanguage, code));
  };

  const insertCodeSnippet = (snippetId: string) => {
    const instance = editorRef.current;
    const template = monacoSnippetTemplateForId(snippetId);
    if (!instance || !template || !isEditable) return;
    instance.focus();
    SnippetController2.get(instance)?.insert(template, {
      undoStopBefore: true,
      undoStopAfter: true,
    });
  };

  // Fallback textarea writer (degraded mode keeps code editable + persisted).
  const onFallbackChange = (next: string) => {
    updateAttributes(makeCodeBlockAttrs(languageRef.current, next));
  };

  return (
    <NodeViewWrapper
      className="monaco-code-block"
      data-testid="monaco-code-block"
      data-language={language}
      data-rt-hash={rtHash}
      data-monaco-mounted={mounted ? "true" : "false"}
      data-degraded={degraded ? "true" : "false"}
      data-keyboard-exit="escape"
      aria-label={`Embedded ${language} code editor. Press Escape to return to the document.`}
    >
      <div className="monaco-code-block__toolbar" contentEditable={false}>
        <label className="monaco-code-block__lang-label">
          <span className="muted">Language</span>
          <select
            data-testid="monaco-code-block-language"
            className="monaco-code-block__lang-select"
            value={language}
            disabled={!isEditable}
            onChange={(event) => onLanguageChange(event.target.value)}
          >
            {HANDSHAKE_CODE_LANGUAGES.map((lang) => (
              <option key={lang.id} value={lang.id}>
                {lang.label}
              </option>
            ))}
          </select>
        </label>
        {EDITOR_SNIPPETS.filter((snippet) => snippet.scope === "code").map((snippet) => (
          <button
            key={snippet.id}
            type="button"
            className="monaco-code-block__snippet-button"
            data-testid={`monaco-code-snippet-${snippet.id}`}
            disabled={!mounted || !isEditable}
            onClick={() => insertCodeSnippet(snippet.id)}
          >
            {snippet.label}
          </button>
        ))}
        <span className="monaco-code-block__hash muted" data-testid="monaco-code-block-hash">
          {rtHash}
        </span>
      </div>

      {/* L7: while the lazy mount has not fired (offscreen block), the code
          stays VISIBLE as a static preview — never a blank surface. */}
      {!mounted && !degraded && (
        <pre
          data-testid="monaco-code-block-preview"
          className="monaco-code-block__preview"
          contentEditable={false}
          style={{ minHeight: 120, margin: 0, overflow: "hidden" }}
        >
          {code}
        </pre>
      )}

      {/* Real Monaco host. contentEditable=false so ProseMirror does not fight
          Monaco for the DOM subtree. */}
      <div
        ref={hostRef}
        data-testid="monaco-code-block-host"
        className="monaco-code-block__host"
        contentEditable={false}
        style={{ minHeight: mounted ? 120 : 0, display: degraded ? "none" : "block" }}
      />

      {/* Degraded fallback: keep code editable + persisted if Monaco can't mount. */}
      {degraded && (
        <textarea
          ref={fallbackRef}
          data-testid="monaco-code-block-fallback"
          className="monaco-code-block__fallback"
          aria-label={`Code (${language}). Press Escape to return to the document.`}
          value={code}
          readOnly={!isEditable}
          spellCheck={false}
          onChange={(event) => onFallbackChange(event.target.value)}
          onKeyDown={(event) => {
            // M6/M17: same keyboard exit as the Monaco path.
            if (event.key === "Escape") {
              event.preventDefault();
              exitToProse();
            }
          }}
          style={{ width: "100%", minHeight: 120, fontFamily: "monospace" }}
        />
      )}
    </NodeViewWrapper>
  );
}
