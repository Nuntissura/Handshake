// WP-KERNEL-009 / MT-020 + MT-030 — dependency-policy proof harness.
//
// A minimal REAL product surface (built by vite.harness.config.ts from the
// same source tree, same lockfile-governed packages) that mounts the bundled
// editor stack so the offline Playwright spec can prove:
//   - Monaco loads from locally bundled assets (workers included) with the
//     network blocked (MT-020 / MT-027 / MT-030),
//   - the Tiptap WP-009 extension set instantiates (MT-021 / MT-030),
//   - dependency failures surface as typed, visible messages (MT-031).
//
// The harness publishes machine-readable state on window.__HARNESS_STATE__ so
// a no-context model can assert readiness without screen-scraping.

import { StrictMode, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import {
  createConfiguredEditor,
  proveTypescriptWorkerRoundTrip,
} from "../lib/monaco/setup";
import { dependencyFailures } from "../lib/dependency_policy/dependency_failure";
import { mountWp009TiptapProof } from "./tiptap_harness_mount";
import "monaco-editor/min/vs/editor/editor.main.css";

export interface HarnessState {
  monacoReady: boolean;
  monacoWorkerProof: string | null;
  tiptapReady: boolean;
  tiptapExtensions: string[];
  /** Live document-model text accessor (MT-030 offline typing proof). */
  tiptapDocText?: () => string;
  /** Deterministic caret placement for the MT-030 keyboard-typing proof. */
  tiptapFocusFreshLeadingParagraph?: () => void;
  failures: Array<{ dependency: string; component: string; message: string }>;
  errors: string[];
}

declare global {
  interface Window {
    __HARNESS_STATE__?: HarnessState;
  }
}

const state: HarnessState = {
  monacoReady: false,
  monacoWorkerProof: null,
  tiptapReady: false,
  tiptapExtensions: [],
  failures: [],
  errors: [],
};
window.__HARNESS_STATE__ = state;

dependencyFailures.subscribe((failure) => {
  state.failures.push({
    dependency: failure.dependency,
    component: failure.component,
    message: failure.message,
  });
});

const TS_SAMPLE = [
  "// WP-KERNEL-009 offline Monaco proof",
  "export function knowledgeIndexEntry(id: string, title: string): string {",
  "  return `${id}: ${title}`;",
  "}",
  "",
].join("\n");

async function mountMonaco(host: HTMLElement): Promise<void> {
  const editor = createConfiguredEditor({
    container: host,
    value: TS_SAMPLE,
    language: "typescript",
    theme: "vs-dark",
  });
  const model = editor.getModel();
  if (!model) throw new Error("monaco model missing");
  // Real worker round-trip: getSyntacticDiagnostics answered by the bundled
  // ts.worker proves a genuine web worker booted from local assets.
  const responded = await proveTypescriptWorkerRoundTrip(model);
  state.monacoWorkerProof = responded ? "ts-worker-responded" : "ts-worker-no-response";
  state.monacoReady = true;
}

function HarnessShell() {
  const monacoHost = useRef<HTMLDivElement>(null);
  const tiptapHost = useRef<HTMLDivElement>(null);
  const mounted = useRef(false);

  useEffect(() => {
    // StrictMode double-invokes effects; the proof must mount exactly once.
    if (mounted.current) return;
    if (!monacoHost.current || !tiptapHost.current) return;
    mounted.current = true;
    const tiptapEl = tiptapHost.current;
    void mountMonaco(monacoHost.current)
      .catch((error: unknown) => {
        state.errors.push(`monaco: ${error instanceof Error ? error.message : String(error)}`);
      })
      .then(() => {
        try {
          const mount = mountWp009TiptapProof(tiptapEl);
          state.tiptapExtensions = mount.extensionNames;
          state.tiptapDocText = mount.docText;
          state.tiptapFocusFreshLeadingParagraph = mount.focusFreshLeadingParagraph;
          state.tiptapReady = true;
        } catch (error) {
          state.errors.push(`tiptap: ${error instanceof Error ? error.message : String(error)}`);
        }
      });
  }, []);

  return (
    <div data-testid="dependency-policy-harness-root" style={{ padding: 16 }}>
      <h1 style={{ fontSize: 16 }}>Handshake dependency-policy harness</h1>
      <section>
        <h2 style={{ fontSize: 14 }}>Monaco (bundled workers, offline)</h2>
        <div
          ref={monacoHost}
          data-testid="monaco-host"
          style={{ height: 240, border: "1px solid #888" }}
        />
      </section>
      <section>
        <h2 style={{ fontSize: 14 }}>Tiptap (WP-009 extension set)</h2>
        <div
          ref={tiptapHost}
          data-testid="tiptap-host"
          style={{ minHeight: 120, border: "1px solid #888" }}
        />
      </section>
      <section aria-live="polite" data-testid="harness-status" />
    </div>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
