// WP-KERNEL-009 / MT-249 — real-backend Monaco code-intelligence proof.
//
// This harness mounts the same configured Monaco entry point the product uses,
// then points the existing API client at a loopback fixture server by rewriting
// only the hardcoded test base URL at fetch time. The providers still call the
// real API client functions and the backend serves the real /knowledge/code/*
// and /api/diagnostics routes.

import { StrictMode, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import {
  getCodeSymbol,
  listProblemGroups,
  type CodeSymbolResponse,
  type ProblemGroup,
} from "../lib/api";
import {
  configureHandshakeCodeIntelligence,
  refreshHandshakeCodeIntelligenceMarkers,
} from "../lib/monaco/code_intelligence";
import { createConfiguredEditor, monaco } from "../lib/monaco/setup";
import "monaco-editor/min/vs/editor/editor.main.css";

type HarnessRequest = {
  originalUrl: string;
  rewrittenUrl: string;
  method: string;
  ok?: boolean;
  status?: number;
  error?: string;
};

type HarnessProof = {
  completionWidgetText: string;
  hoverText: string;
  definitionRoute: HarnessRequest;
  referenceRoute: HarnessRequest;
  formatRemovedTrailingWhitespace: boolean;
  markers: Array<{ message: string; startLineNumber: number; startColumn: number }>;
  problem: ProblemGroup;
  symbolDetail: CodeSymbolResponse;
  routeHits: HarnessRequest[];
};

type HarnessState = {
  ready: boolean;
  workspaceId: string | null;
  symbolEntityId: string | null;
  backendBase: string | null;
  errors: string[];
  requests: HarnessRequest[];
  runFullProof?: () => Promise<HarnessProof>;
};

declare global {
  interface Window {
    __MT249_STATE__?: HarnessState;
  }
}

const DEFAULT_API_BASE = "http://127.0.0.1:37501";
type FetchInput = Parameters<typeof fetch>[0];
type FetchInit = Parameters<typeof fetch>[1];
const SAMPLE = [
  "// MT-249 runtime Monaco proof",
  "pub fn scratch() -> i32 {",
  "  ad",
  "  add(1, 2)",
  "}  ",
  "",
].join("\n");

const params = new URLSearchParams(window.location.search);
const backendBase = params.get("backend");
const workspaceId = params.get("workspace_id");
const symbolEntityId = params.get("symbol_entity_id");

const state: HarnessState = {
  ready: false,
  workspaceId,
  symbolEntityId,
  backendBase,
  errors: [],
  requests: [],
};
window.__MT249_STATE__ = state;

function installFetchRewrite(): void {
  if (!backendBase) return;
  const realFetch = window.fetch.bind(window);
  window.fetch = async (input: FetchInput | URL, init?: FetchInit) => {
    const originalUrl =
      typeof input === "string"
        ? input
        : input instanceof URL
          ? input.toString()
          : input.url;
    const rewrittenUrl = originalUrl.startsWith(DEFAULT_API_BASE)
      ? `${backendBase}${originalUrl.slice(DEFAULT_API_BASE.length)}`
      : originalUrl;
    const requestRecord: HarnessRequest | null =
      originalUrl !== rewrittenUrl
        ? {
            originalUrl,
            rewrittenUrl,
            method: init?.method ?? (input instanceof Request ? input.method : "GET"),
          }
        : null;
    if (requestRecord) state.requests.push(requestRecord);
    try {
      const response =
        input instanceof Request
          ? await realFetch(new Request(rewrittenUrl, input), init)
          : await realFetch(rewrittenUrl, init);
      if (requestRecord) {
        requestRecord.ok = response.ok;
        requestRecord.status = response.status;
        if (!response.ok) {
          requestRecord.error = await response.clone().text();
        }
      }
      return response;
    } catch (error) {
      if (requestRecord) {
        requestRecord.ok = false;
        requestRecord.error = error instanceof Error ? error.message : String(error);
      }
      throw error;
    }
  };
}

function textFrom(selector: string): string {
  return Array.from(document.querySelectorAll(selector))
    .map((node) => node.textContent ?? "")
    .join("\n");
}

async function waitFor<T>(
  probe: () => T | null | undefined | Promise<T | null | undefined>,
  label: string,
  timeoutMs = 12_000,
): Promise<T> {
  const deadline = Date.now() + timeoutMs;
  let lastValue: T | null | undefined;
  while (Date.now() < deadline) {
    lastValue = await probe();
    if (lastValue) return lastValue;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for ${label}; last=${JSON.stringify(lastValue)}`);
}

async function waitForProblem(workspace: string): Promise<ProblemGroup> {
  try {
    return await waitFor(
      async () => {
        const groups = await listProblemGroups({
          source: "lsp",
          surface: "monaco",
          wsid: workspace,
          limit: 20,
        });
        return groups.find((group) => group.sample.code === "HSK-CODE-INTEL-STALE");
      },
      "stale code-intelligence diagnostic problem group",
      20_000,
    );
  } catch (error) {
    throw new Error(
      `${error instanceof Error ? error.message : String(error)}; requests=${JSON.stringify(
        state.requests.filter((request) => request.originalUrl.includes("/api/diagnostics")),
      )}`,
    );
  }
}

async function waitForRouteAfter(
  startIndex: number,
  predicate: (request: HarnessRequest) => boolean,
  label: string,
): Promise<HarnessRequest> {
  return waitFor(
    () => state.requests.slice(startIndex).find(predicate),
    label,
    12_000,
  );
}

async function executeReferenceProvider(editor: monaco.editor.IStandaloneCodeEditor): Promise<void> {
  const model = editor.getModel();
  const position = editor.getPosition();
  const commandService = (
    editor as unknown as {
      _commandService?: {
        executeCommand: (id: string, ...args: unknown[]) => Promise<unknown>;
      };
    }
  )._commandService;
  if (!model || !position || !commandService) {
    throw new Error("Monaco reference provider command service unavailable");
  }
  await commandService.executeCommand("_executeReferenceProvider", model.uri, position);
}

function HarnessShell() {
  const monacoHost = useRef<HTMLDivElement>(null);
  const mounted = useRef(false);

  useEffect(() => {
    if (mounted.current) return;
    mounted.current = true;
    if (!monacoHost.current) return;
    if (!workspaceId) {
      state.errors.push("workspace_id query param missing");
      return;
    }
    if (!backendBase) {
      state.errors.push("backend query param missing");
      return;
    }
    if (!symbolEntityId) {
      state.errors.push("symbol_entity_id query param missing");
      return;
    }
    installFetchRewrite();
    configureHandshakeCodeIntelligence({
      workspaceId,
      openCodeSymbol(symbolEntityId) {
        state.requests.push({
          originalUrl: `harness://open-symbol/${symbolEntityId}`,
          rewrittenUrl: `harness://open-symbol/${symbolEntityId}`,
          method: "HARNESS",
        });
      },
    });

    const editor = createConfiguredEditor({
      container: monacoHost.current,
      value: SAMPLE,
      language: "rust",
      theme: "vs-dark",
      hover: { enabled: true, delay: 0 },
      quickSuggestions: true,
    });
    state.runFullProof = async () => {
      editor.focus();
      editor.setPosition({ lineNumber: 3, column: 5 });
      editor.trigger("mt249", "editor.action.triggerSuggest", {});
      const completionWidgetText = await waitFor(() => {
        const text = textFrom(".suggest-widget");
        return text.includes("add") ? text : null;
      }, "Monaco completion widget with backend add suggestion");

      editor.setPosition({ lineNumber: 4, column: 4 });
      await editor.getAction("editor.action.showHover")?.run();
      const hoverText = await waitFor(() => {
        const text = textFrom(".monaco-hover");
        return text.includes("Adds two numbers.") && text.includes("marked_stale")
          ? text
          : null;
      }, "Monaco hover with backend file-lens documentation and staleness");

      const definitionStart = state.requests.length;
      editor.setPosition({ lineNumber: 4, column: 4 });
      await editor.getAction("editor.action.revealDefinition")?.run();
      const definitionRoute = await waitForRouteAfter(
        definitionStart,
        (request) =>
          request.originalUrl.includes("/knowledge/code/symbols?") &&
          request.originalUrl.includes("name=add"),
        "Monaco definition provider backend symbol lookup",
      );

      const referencesStart = state.requests.length;
      editor.setPosition({ lineNumber: 4, column: 4 });
      await executeReferenceProvider(editor);
      const referenceRoute = await waitForRouteAfter(
        referencesStart,
        (request) => request.originalUrl.includes(`/knowledge/code/symbols/${symbolEntityId}/references`),
        "Monaco references provider backend references lookup",
      );

      const beforeFormat = editor.getValue();
      await editor.getAction("editor.action.formatDocument")?.run();
      const afterFormat = editor.getValue();
      const formatRemovedTrailingWhitespace =
        beforeFormat.includes("}  \n") && !afterFormat.includes("}  \n");

      await refreshHandshakeCodeIntelligenceMarkers(editor, monaco);
      const markers = monaco.editor
        .getModelMarkers({ owner: "handshake-code-intelligence" })
        .map((marker) => ({
          message: marker.message,
          startLineNumber: marker.startLineNumber,
          startColumn: marker.startColumn,
        }));
      const problem = await waitForProblem(workspaceId);
      const symbolDetail = await getCodeSymbol(symbolEntityId);
      return {
        completionWidgetText,
        hoverText,
        definitionRoute,
        referenceRoute,
        formatRemovedTrailingWhitespace,
        markers,
        problem,
        symbolDetail,
        routeHits: state.requests.filter(
          (request) =>
            request.originalUrl.includes("/knowledge/code/") ||
            request.originalUrl.includes("/api/diagnostics"),
        ),
      };
    };
    state.ready = true;
  }, []);

  return (
    <main data-testid="mt249-code-intelligence-root" style={{ padding: 16 }}>
      <h1 style={{ fontSize: 16 }}>MT-249 code intelligence harness</h1>
      <div
        ref={monacoHost}
        data-testid="mt249-monaco-host"
        style={{ height: 360, border: "1px solid #888" }}
      />
    </main>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
