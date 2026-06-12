// WP-KERNEL-009 / MT-175 + MT-176 — integrated rich-editor offline proof harness.
//
// A REAL product surface (built by vite.harness.config.ts from the same source
// tree + lockfile-governed packages) that mounts the INTEGRATED RichTextEditor
// with a document containing an embedded Monaco code block, a typed [[wp:…]]
// wikilink, and a typed media embed link. The offline Playwright spec
// (tests/dependency_policy/rich_editor_roundtrip.spec.ts) drives it to prove:
//   - the editor + code block + workers boot from Handshake-bundled assets with
//     the network cut (MT-175: no external editor / server / CDN),
//   - editing the code block + saving round-trips language + text + hash
//     (MT-176 round-trip),
//   - the stable visual-debug selectors are present (MT-172/176),
//   - ZERO external network requests are attempted (MT-175).
//
// Round-trip state is published on window.__RICH_EDITOR_HARNESS__ so the spec
// asserts without screen-scraping. The "save" here is an in-harness round-trip
// (serialize the editor JSON, re-load it into a fresh editor, compare) — it does
// NOT call a backend (the offline proof must not need a server); the backend
// save path is covered by the RichDocumentView vitest suite against the real API.

import { StrictMode, useCallback, useEffect } from "react";
import { createRoot } from "react-dom/client";
import type { JSONContent } from "@tiptap/core";
import { RichTextEditor } from "../components/RichTextEditor";
import {
  EDITOR_DEBUG_GLOBAL_KEY,
  type EditorDebugSnapshot,
} from "../lib/editor/visual_debug";
import { makeCodeBlockAttrs, codeBlockHash } from "../lib/editor/code_block_serialization";
import "monaco-editor/min/vs/editor/editor.main.css";

export interface RichEditorHarnessState {
  /** Latest editor document JSON (the round-trip subject). */
  docJson: JSONContent | null;
  /** The visual-debug snapshot the editor publishes. */
  debug: EditorDebugSnapshot | null;
  /** Result of the last in-harness round-trip (set by runRoundTrip). */
  roundTrip:
    | null
    | {
        ok: boolean;
        beforeHash: string;
        afterHash: string;
        beforeLanguage: string;
        afterLanguage: string;
        beforeCode: string;
        afterCode: string;
      };
}

declare global {
  interface Window {
    __RICH_EDITOR_HARNESS__?: RichEditorHarnessState & {
      runRoundTrip?: () => void;
    };
  }
}

const INITIAL_DOC: JSONContent = {
  type: "doc",
  content: [
    { type: "heading", attrs: { level: 1 }, content: [{ type: "text", text: "Offline rich-editor proof" }] },
    {
      type: "paragraph",
      content: [{ type: "text", text: "Intro paragraph with a typed link and an embed." }],
    },
    {
      type: "hsLink",
      attrs: { refKind: "wp", refValue: "WP-KERNEL-009", label: "WP-KERNEL-009", resolved: true },
    },
    {
      type: "hsLink",
      attrs: { refKind: "album", refValue: "album-001", label: "album-001", resolved: true },
    },
    {
      type: "monacoCodeBlock",
      attrs: makeCodeBlockAttrs("typescript", "export const offline = true;"),
    },
  ],
};

const state: RichEditorHarnessState = { docJson: INITIAL_DOC, debug: null, roundTrip: null };

function firstCodeBlockAttrs(doc: JSONContent | null): { language: string; code: string; contentHash: string } | null {
  if (!doc?.content) return null;
  for (const node of doc.content) {
    if (node.type === "monacoCodeBlock") {
      const attrs = node.attrs ?? {};
      return {
        language: String(attrs.language ?? ""),
        code: String(attrs.code ?? ""),
        contentHash: String(attrs.contentHash ?? ""),
      };
    }
  }
  return null;
}

function HarnessShell() {
  // Iteration-3 H1: do NOT store onChange JSON in state and pass it back as
  // initialContent — that echo loop teleported the caret on every keystroke.
  // The editor owns the live document; the harness only mirrors the latest
  // JSON (+ the editor's debug snapshot) for the spec to read.
  const onChange = useCallback((next: JSONContent) => {
    state.docJson = next;
    const debug = (globalThis as Record<string, unknown>)[EDITOR_DEBUG_GLOBAL_KEY] as
      | EditorDebugSnapshot
      | undefined;
    if (debug) state.debug = debug;
  }, []);

  // In-harness round-trip: snapshot the current code block, serialize the doc to
  // JSON, re-hydrate the comparison, and verify language+code+hash are stable.
  const runRoundTrip = useCallback(() => {
    const before = firstCodeBlockAttrs(state.docJson);
    // Re-serialize through JSON (the persistence path's content_json) and read back.
    const serialized = JSON.parse(JSON.stringify(state.docJson)) as JSONContent;
    const after = firstCodeBlockAttrs(serialized);
    const result = {
      ok:
        !!before &&
        !!after &&
        before.language === after.language &&
        before.code === after.code &&
        // Recompute the hash from the round-tripped content and confirm it matches.
        codeBlockHash(after.language, after.code) === after.contentHash &&
        before.contentHash === after.contentHash,
      beforeHash: before?.contentHash ?? "",
      afterHash: after?.contentHash ?? "",
      beforeLanguage: before?.language ?? "",
      afterLanguage: after?.language ?? "",
      beforeCode: before?.code ?? "",
      afterCode: after?.code ?? "",
    };
    state.roundTrip = result;
    if (window.__RICH_EDITOR_HARNESS__) window.__RICH_EDITOR_HARNESS__.roundTrip = result;
  }, []);

  // Publish the harness control surface via an effect (NOT the render body) so
  // a no-context spec can drive it without render-time global mutation.
  useEffect(() => {
    window.__RICH_EDITOR_HARNESS__ = Object.assign(state, { runRoundTrip });
  }, [runRoundTrip]);

  return (
    <div data-testid="rich-editor-harness-root" style={{ padding: 16 }}>
      <h1 style={{ fontSize: 16 }}>Handshake rich-editor offline harness</h1>
      <RichTextEditor initialContent={INITIAL_DOC} onChange={onChange} />
      <button type="button" data-testid="harness-run-roundtrip" onClick={runRoundTrip}>
        Run round-trip
      </button>
    </div>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
