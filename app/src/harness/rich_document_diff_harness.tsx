// WP-KERNEL-009 / MT-247 — offline rich-document diff proof harness.
//
// This harness uses the same document diff model as RichDocumentView and mounts
// a real Monaco diff editor through the bundled setup entry point. It publishes
// state on window so Playwright can assert readiness without screen scraping.

import { StrictMode, useEffect, useMemo, useRef } from "react";
import { createRoot } from "react-dom/client";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";
import { buildRichDocumentDiff, type RichDocumentDiff } from "../lib/editor/document_diff_merge";
import { createConfiguredDiffEditor, monaco } from "../lib/monaco/setup";
import "monaco-editor/min/vs/editor/editor.main.css";

interface DiffHarnessState {
  monacoDiffReady: boolean;
  diffLineChanges: number | null;
  blockKinds: string[];
  blockStatuses: string[];
  leftCode: string;
  rightCode: string;
  errors: string[];
}

declare global {
  interface Window {
    __HS_RICH_DOCUMENT_DIFF_HARNESS__?: DiffHarnessState;
  }
}

const before = {
  type: "doc",
  content: [
    { type: "paragraph", content: [{ type: "text", text: "Intro v1" }] },
    { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("ts", "const count = 1;") },
  ],
};

const after = {
  type: "doc",
  content: [
    { type: "paragraph", content: [{ type: "text", text: "Intro v2" }] },
    { type: "monacoCodeBlock", attrs: makeCodeBlockAttrs("typescript", "const count = 2;") },
  ],
};

const diff = buildRichDocumentDiff({ left: before, right: after });
const codeBlock = diff.blocks.find((block) => block.kind === "code");

const state: DiffHarnessState = {
  monacoDiffReady: false,
  diffLineChanges: null,
  blockKinds: diff.blocks.map((block) => block.kind),
  blockStatuses: diff.blocks.map((block) => block.status),
  leftCode: codeBlock?.leftCode?.code ?? "",
  rightCode: codeBlock?.rightCode?.code ?? "",
  errors: [],
};
window.__HS_RICH_DOCUMENT_DIFF_HARNESS__ = state;

function blockText(block: RichDocumentDiff["blocks"][number], side: "left" | "right"): string {
  if (block.kind === "code") {
    return side === "left" ? block.leftCode?.code ?? "" : block.rightCode?.code ?? "";
  }
  return side === "left" ? block.leftText ?? "" : block.rightText ?? "";
}

function HarnessShell() {
  const monacoHost = useRef<HTMLDivElement>(null);
  const mounted = useRef(false);
  const blocks = useMemo(() => diff.blocks, []);

  useEffect(() => {
    if (mounted.current || !monacoHost.current) return;
    mounted.current = true;
    let poll: number | null = null;
    try {
      monaco.editor.setTheme("vs-dark");
      const editor = createConfiguredDiffEditor({
        container: monacoHost.current,
        renderSideBySide: true,
        originalEditable: false,
        readOnly: true,
        fontSize: 13,
      });
      const original = monaco.editor.createModel(state.leftCode, "typescript");
      const modified = monaco.editor.createModel(state.rightCode, "typescript");
      editor.setModel({ original, modified });

      const publishReady = () => {
        const lineChanges = editor.getLineChanges();
        if (lineChanges === null) return;
        state.diffLineChanges = lineChanges.length;
        state.monacoDiffReady = true;
        if (poll !== null) {
          window.clearInterval(poll);
          poll = null;
        }
      };
      const disposable = editor.onDidUpdateDiff(publishReady);
      poll = window.setInterval(publishReady, 100);
      publishReady();

      return () => {
        if (poll !== null) window.clearInterval(poll);
        disposable.dispose();
        editor.dispose();
        original.dispose();
        modified.dispose();
      };
    } catch (error) {
      state.errors.push(error instanceof Error ? error.message : String(error));
    }
  }, []);

  return (
    <div data-testid="rich-document-diff-harness-root" style={{ padding: 16 }}>
      <h1 style={{ fontSize: 16 }}>Handshake rich-document diff harness</h1>
      <section data-testid="rich-document-diff-panel" data-left-version="1" data-right-version="2">
        {blocks.map((block) => (
          <div
            key={block.blockIndex}
            data-testid="rich-document-diff-block"
            data-diff-kind={block.kind}
            data-diff-status={block.status}
            style={{ border: "1px solid #777", marginBottom: 8, padding: 8 }}
          >
            <strong>Block {block.blockIndex + 1}</strong>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <pre data-testid={`rich-document-diff-block-${block.blockIndex}-left`}>
                {blockText(block, "left")}
              </pre>
              <pre data-testid={`rich-document-diff-block-${block.blockIndex}-right`}>
                {blockText(block, "right")}
              </pre>
            </div>
          </div>
        ))}
      </section>
      <section>
        <h2 style={{ fontSize: 14 }}>Monaco code diff</h2>
        <div
          ref={monacoHost}
          data-testid="rich-document-code-diff-monaco"
          style={{ height: 260, border: "1px solid #777" }}
        />
      </section>
    </div>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
