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
import {
  HANDSHAKE_CODE_LANGUAGES,
  DEFAULT_CODE_LANGUAGE,
} from "../lib/monaco/language_registry";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";
import { registerCodeBlockFindHandle } from "../lib/editor/code_block_find_registry";
import {
  dependencyFailures,
  formatDependencyFailureMessage,
} from "../lib/dependency_policy/dependency_failure";

type Editor = monaco.editor.IStandaloneCodeEditor;

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
        }
      },
    });
  }, [getPos]);

  // Mount Monaco once.
  useEffect(() => {
    const host = hostRef.current;
    if (!host || editorRef.current) return;
    let instance: Editor | null = null;
    try {
      instance = createConfiguredEditor({
        container: host,
        value: code,
        language,
        readOnly: !isEditable,
        scrollBeyondLastLine: false,
        lineNumbers: "on",
        fontSize: 13,
        theme: "vs-dark",
      });
      editorRef.current = instance;
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
      instance?.dispose();
      editorRef.current = null;
    };
    // Mount-once: language/code changes are reconciled by the effects below.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Reconcile external code changes (undo/redo, collaborative edits, reload)
  // into the Monaco model without clobbering the user's caret on self-edits.
  useEffect(() => {
    const instance = editorRef.current;
    if (!instance) return;
    if (instance.getValue() !== code) {
      applyingRef.current = true;
      instance.setValue(code);
      applyingRef.current = false;
    }
  }, [code]);

  // Reconcile language changes into the Monaco model.
  useEffect(() => {
    const instance = editorRef.current;
    const model = instance?.getModel();
    if (model) monaco.editor.setModelLanguage(model, language);
  }, [language]);

  // Reflect read-only state.
  useEffect(() => {
    editorRef.current?.updateOptions({ readOnly: !isEditable });
  }, [isEditable]);

  // M10: every attr write goes through makeCodeBlockAttrs — the single minting
  // point that normalizes the language and computes the matching hash.
  const onLanguageChange = (nextLanguage: string) => {
    updateAttributes(makeCodeBlockAttrs(nextLanguage, code));
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
        <span className="monaco-code-block__hash muted" data-testid="monaco-code-block-hash">
          {rtHash}
        </span>
      </div>

      {/* Real Monaco host. contentEditable=false so ProseMirror does not fight
          Monaco for the DOM subtree. */}
      <div
        ref={hostRef}
        data-testid="monaco-code-block-host"
        className="monaco-code-block__host"
        contentEditable={false}
        style={{ minHeight: 120, display: degraded ? "none" : "block" }}
      />

      {/* Degraded fallback: keep code editable + persisted if Monaco can't mount. */}
      {degraded && (
        <textarea
          ref={fallbackRef}
          data-testid="monaco-code-block-fallback"
          className="monaco-code-block__fallback"
          value={code}
          readOnly={!isEditable}
          spellCheck={false}
          onChange={(event) => onFallbackChange(event.target.value)}
          style={{ width: "100%", minHeight: 120, fontFamily: "monospace" }}
        />
      )}
    </NodeViewWrapper>
  );
}
