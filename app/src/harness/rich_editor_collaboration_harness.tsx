// WP-KERNEL-009 / MT-246 - same-document RichTextEditor collaboration proof.
//
// Offline harness for the REAL RichTextEditor mounted twice against one Yjs
// document. Playwright drives editor A through the exposed control surface and
// proves editor B converges without save/reload or backend polling.

import { StrictMode, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Editor, JSONContent } from "@tiptap/core";
import { Doc as YDoc } from "yjs";
import { RichTextEditor } from "../components/RichTextEditor";
import "../App.css";

const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [{ type: "paragraph", content: [{ type: "text", text: "shared alpha" }] }],
};

const sharedCollaborationDocument = new YDoc({ guid: "mt246-same-document-proof" });

type CollaborationHarnessSnapshot = {
  ready: boolean;
  editorBMounted: boolean;
  consistent: boolean;
  editorAText: string;
  editorBText: string;
  collaborationDocumentGuid: string;
};

type CollaborationHarnessControl = {
  getState: () => CollaborationHarnessSnapshot;
  applyEdit: (text?: string) => CollaborationHarnessSnapshot;
  openSecondEditor: () => CollaborationHarnessSnapshot;
};

declare global {
  interface Window {
    __MT246_COLLAB_HARNESS__?: CollaborationHarnessControl;
  }
}

function editorText(editor: Editor | null): string {
  if (!editor || editor.isDestroyed) return "";
  return editor.state.doc.textBetween(0, editor.state.doc.content.size, "\n");
}

function docForText(text: string): JSONContent {
  return {
    type: "doc",
    content: [{ type: "paragraph", content: [{ type: "text", text }] }],
  };
}

function HarnessShell() {
  const editorARef = useRef<Editor | null>(null);
  const editorBRef = useRef<Editor | null>(null);
  const [showEditorB, setShowEditorB] = useState(false);
  const [snapshot, setSnapshot] = useState<CollaborationHarnessSnapshot>({
    ready: false,
    editorBMounted: false,
    consistent: false,
    editorAText: "",
    editorBText: "",
    collaborationDocumentGuid: sharedCollaborationDocument.guid,
  });

  const capture = useCallback((): CollaborationHarnessSnapshot => {
    const editorAText = editorText(editorARef.current);
    const editorBText = editorText(editorBRef.current);
    return {
      ready: !!editorARef.current,
      editorBMounted: !!editorBRef.current,
      consistent: !!editorBRef.current && editorAText.length > 0 && editorAText === editorBText,
      editorAText,
      editorBText,
      collaborationDocumentGuid: sharedCollaborationDocument.guid,
    };
  }, []);

  const publish = useCallback(() => {
    const next = capture();
    setSnapshot(next);
    return next;
  }, [capture]);

  const applyEdit = useCallback(
    (text = "shared beta browser proof") => {
      editorARef.current?.commands.setContent(docForText(text));
      const next = publish();
      queueMicrotask(publish);
      window.setTimeout(publish, 0);
      return next;
    },
    [publish],
  );

  const openSecondEditor = useCallback(() => {
    setShowEditorB(true);
    const next = publish();
    queueMicrotask(publish);
    window.setTimeout(publish, 0);
    return next;
  }, [publish]);

  const control = useMemo<CollaborationHarnessControl>(
    () => ({
      getState: capture,
      applyEdit,
      openSecondEditor,
    }),
    [applyEdit, capture, openSecondEditor],
  );

  useEffect(() => {
    window.__MT246_COLLAB_HARNESS__ = control;
    return () => {
      delete window.__MT246_COLLAB_HARNESS__;
    };
  }, [control]);

  const onEditorAReady = useCallback(
    (editor: Editor) => {
      editorARef.current = editor;
      publish();
    },
    [publish],
  );
  const onEditorBReady = useCallback(
    (editor: Editor) => {
      editorBRef.current = editor;
      publish();
    },
    [publish],
  );
  const onEditorChange = useCallback(() => {
    publish();
  }, [publish]);

  return (
    <main className="mt246-root" data-testid="mt246-collaboration-root">
      <style>{`
        .mt246-root {
          box-sizing: border-box;
          min-height: 100vh;
          padding: 16px;
          background: var(--hs-color-bg);
          color: var(--hs-color-text);
        }

        .mt246-grid {
          display: grid;
          grid-template-columns: repeat(2, minmax(0, 1fr));
          gap: 12px;
          min-height: 520px;
        }

        .mt246-group {
          min-width: 0;
          border: 1px solid var(--hs-color-border);
          border-radius: 8px;
          background: var(--hs-color-surface);
          padding: 10px;
        }

        .mt246-group h2 {
          margin: 0 0 8px;
          font-size: 14px;
          line-height: 1.2;
        }

        .mt246-group .rich-text-editor {
          min-height: 430px;
        }

        .mt246-status {
          display: block;
          margin-bottom: 12px;
          font-size: 13px;
        }
      `}</style>
      <output
        className="mt246-status"
        data-testid="mt246-consistency-status"
        data-ready={String(snapshot.ready)}
        data-consistent={String(snapshot.consistent)}
      >
        {snapshot.consistent ? "consistent" : "waiting"}
      </output>
      <section className="mt246-grid" aria-label="Same document editor groups">
        <article className="mt246-group" data-testid="mt246-editor-group-a">
          <h2>Group A</h2>
          <RichTextEditor
            initialContent={INITIAL_DOC}
            onChange={onEditorChange}
            onEditorReady={onEditorAReady}
            collaborationDocument={sharedCollaborationDocument}
            debugId="mt246-editor-a"
          />
        </article>
        <article className="mt246-group" data-testid="mt246-editor-group-b">
          <h2>Group B</h2>
          {showEditorB ? (
            <RichTextEditor
              initialContent={INITIAL_DOC}
              onChange={onEditorChange}
              onEditorReady={onEditorBReady}
              collaborationDocument={sharedCollaborationDocument}
              debugId="mt246-editor-b"
            />
          ) : (
            <p className="muted" data-testid="mt246-editor-group-b-placeholder">
              Second split group not mounted yet.
            </p>
          )}
        </article>
      </section>
    </main>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
