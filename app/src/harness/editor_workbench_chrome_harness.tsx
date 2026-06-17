// WP-KERNEL-009 / MT-245 — editor workbench chrome offline proof harness.
//
// A REAL product surface (built by vite.harness.config.ts from the same source
// tree + lockfile-governed packages) mounting the INTEGRATED RichTextEditor with
// the MT-245 workbench chrome live:
//   - ED-NAV-004: prose outline panel (heading tree + click-to-scroll).
//   - ED-WB-007:  status bar (cursor Ln/Col, focused code-block language, save/
//                 dirty/conflict state, word count). `documentStatus` is fed
//                 from harness controls so the bar reflects authority save state.
//   - ED-NAV-006: go-to-line inside the focused embedded code block (palette).
//   - EXT-SAVE-001: Mod-s save + palette save route through onSaveRequested.
//   - EXT-NAV-LINK-001: clicking a typed hsLink dispatches the navigation intent;
//                 this harness consumes it through the SAME pure resolver App.tsx
//                 uses (resolveHsLinkTarget) and renders a typed visible error for
//                 unresolvable links — never a silent no-op.
//
// The offline Playwright spec (tests/dependency_policy/editor_workbench_chrome.spec.ts)
// drives this harness with the network cut and asserts each row functions in a
// REAL browser. State the spec reads is published on window.__MT245_CHROME__.

import { StrictMode, useCallback, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "../components/RichTextEditor";
import {
  EDITOR_DEBUG_ENABLE_KEY,
} from "../lib/editor/visual_debug";
import {
  onHsLinkNavigate,
  resolveHsLinkTarget,
  type HsLinkTarget,
} from "../lib/editor/link_navigation";
import { makeCodeBlockAttrs } from "../lib/editor/code_block_serialization";
import type { DocumentChromeStatus } from "../lib/editor/editor_chrome";
import "monaco-editor/min/vs/editor/editor.main.css";

(globalThis as Record<string, unknown>)[EDITOR_DEBUG_ENABLE_KEY] = true;

const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Runbook" }] },
    { type: "paragraph", content: [{ type: "text", text: "Intro paragraph with several words to count." }] },
    { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Deploy steps" }] },
    {
      type: "paragraph",
      content: [
        {
          type: "hsLink",
          attrs: { refKind: "wp", refValue: "WP-KERNEL-009", label: "WP-KERNEL-009", resolved: true },
        },
        { type: "text", text: " and " },
        {
          type: "hsLink",
          attrs: { refKind: "ghost", refValue: "nowhere-123", label: "ghost", resolved: false },
        },
      ],
    },
    { type: "heading", attrs: { level: 2 }, content: [{ type: "text", text: "Snippet" }] },
    {
      type: "monacoCodeBlock",
      attrs: makeCodeBlockAttrs(
        "typescript",
        "const a = 1;\nconst b = 2;\nconst c = 3;\nconst d = 4;\nconst sum = a + b + c + d;",
      ),
    },
  ],
};

interface Mt245ChromeState {
  saveCount: number;
  lastLink: HsLinkTarget | null;
}

declare global {
  interface Window {
    __MT245_CHROME__?: Mt245ChromeState;
  }
}

const harnessState: Mt245ChromeState = { saveCount: 0, lastLink: null };

function HarnessShell() {
  // ED-WB-007: authority-owned save/dirty/conflict state for the status bar.
  // Harness controls flip these so the spec can prove the bar reflects each.
  const [docStatus, setDocStatus] = useState<DocumentChromeStatus>({
    dirty: true,
    saving: false,
    blocked: false,
    backendErrorKind: null,
    lastSavedAt: null,
  });
  const [saveCount, setSaveCount] = useState(0);
  const [linkError, setLinkError] = useState<string | null>(null);

  // EXT-NAV-LINK-001: consume the typed navigation intent through the SAME pure
  // resolver App.tsx uses. Resolvable -> record target + clear error; otherwise
  // a typed, visible error.
  useEffect(
    () =>
      onHsLinkNavigate((detail) => {
        const target = resolveHsLinkTarget(detail);
        harnessState.lastLink = target;
        if (window.__MT245_CHROME__) window.__MT245_CHROME__.lastLink = target;
        if (target.kind === "error") {
          setLinkError(target.message);
        } else {
          setLinkError(null);
        }
      }),
    [],
  );

  useEffect(() => {
    window.__MT245_CHROME__ = harnessState;
  }, []);

  const onSaveRequested = useCallback(() => {
    harnessState.saveCount += 1;
    setSaveCount((c) => c + 1);
    if (window.__MT245_CHROME__) window.__MT245_CHROME__.saveCount = harnessState.saveCount;
    // A real save flips dirty -> saved with a stamped lastSavedAt.
    setDocStatus((prev) => ({ ...prev, dirty: false, lastSavedAt: "2026-06-17T00:00:00Z" }));
  }, []);

  return (
    <div data-testid="mt245-chrome-root" style={{ padding: 16 }}>
      <h1 style={{ fontSize: 16 }}>MT-245 editor workbench chrome offline harness</h1>
      <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
        <button
          type="button"
          data-testid="harness-mark-conflict"
          onClick={() => setDocStatus((p) => ({ ...p, backendErrorKind: "conflict", dirty: true }))}
        >
          Force conflict
        </button>
        <button
          type="button"
          data-testid="harness-mark-saving"
          onClick={() => setDocStatus((p) => ({ ...p, saving: true }))}
        >
          Force saving
        </button>
        <span data-testid="harness-save-count">{saveCount}</span>
      </div>
      {linkError && (
        <div data-testid="harness-link-error" role="alert">
          {linkError}
        </div>
      )}
      <RichTextEditor
        initialContent={INITIAL_DOC}
        documentStatus={docStatus}
        onSaveRequested={onSaveRequested}
        onChange={() => {}}
      />
    </div>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
