// WP-KERNEL-009 / MT-020 — Monaco package integration (bundled, offline-only).
//
// Configures the `monaco-editor` npm package (lockfile-governed, MIT) with
// Vite-bundled web workers. Every worker is imported via the `?worker` suffix,
// which makes Vite emit it as a local chunk loaded with
// `new Worker(new URL("...", import.meta.url))` — same-origin, CSP-safe
// (script-src 'self'), and fully offline. The CDN-based @monaco-editor/loader
// is forbidden by the runtime dependency allowlist (MT-017/MT-018).
//
// Worker construction failures are reported through the typed
// dependency-failure surface (MT-031) instead of dying silently.

import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import JsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import CssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import HtmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import TsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import {
  dependencyFailures,
  formatDependencyFailureMessage,
} from "../dependency_policy/dependency_failure";
import {
  BUNDLED_MONACO_WORKER_KINDS,
  workerKindForLabel,
  type MonacoWorkerKind,
} from "./worker_map";
import { ensureHandshakeCodeIntelligenceProviders } from "./code_intelligence";

type WorkerConstructor = new () => Worker;

const BUNDLED_WORKERS: Record<MonacoWorkerKind, WorkerConstructor> = {
  editor: EditorWorker,
  typescript: TsWorker,
  json: JsonWorker,
  css: CssWorker,
  html: HtmlWorker,
};

/** Test seam: lets MT-031 simulate worker-construction failure for one kind. */
let workerFactoryOverride: ((kind: MonacoWorkerKind) => Worker) | null = null;

export function setMonacoWorkerFactoryForTests(
  factory: ((kind: MonacoWorkerKind) => Worker) | null,
): void {
  workerFactoryOverride = factory;
}

function constructWorker(label: string): Worker {
  const kind = workerKindForLabel(label);
  try {
    if (workerFactoryOverride) return workerFactoryOverride(kind);
    return new BUNDLED_WORKERS[kind]();
  } catch (error) {
    const failure = {
      dependency: "monaco-editor",
      component: `worker:${kind}`,
      phase: "worker_construction" as const,
      cause: error instanceof Error ? error.message : String(error),
    };
    dependencyFailures.report({
      ...failure,
      message: formatDependencyFailureMessage(failure),
    });
    throw error;
  }
}

let environmentInstalled = false;
let handshakeThemesInstalled = false;
const proofWorkers: Worker[] = [];
export const HANDSHAKE_MONACO_LIGHT_THEME = "handshake-light";
export const HANDSHAKE_MONACO_DARK_THEME = "handshake-dark";

/**
 * Installs MonacoEnvironment.getWorker with the locally bundled workers.
 * Idempotent; called automatically by createConfiguredEditor.
 */
export function ensureMonacoEnvironment(): void {
  if (environmentInstalled) return;
  (globalThis as { MonacoEnvironment?: monaco.Environment }).MonacoEnvironment = {
    getWorker(_workerId: string, label: string): Worker {
      return constructWorker(label);
    },
  };
  environmentInstalled = true;
}

export function ensureHandshakeMonacoThemes(): void {
  if (handshakeThemesInstalled) return;
  monaco.editor.defineTheme(HANDSHAKE_MONACO_LIGHT_THEME, {
    base: "vs",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": "#ffffff",
      "editor.foreground": "#0f172a",
      "editorLineNumber.foreground": "#64748b",
      "editor.selectionBackground": "#bfdbfe",
      "editorCursor.foreground": "#2563eb",
    },
  });
  monaco.editor.defineTheme(HANDSHAKE_MONACO_DARK_THEME, {
    base: "vs-dark",
    inherit: true,
    rules: [],
    colors: {
      "editor.background": "#1d232b",
      "editor.foreground": "#f8fafc",
      "editorLineNumber.foreground": "#94a3b8",
      "editor.selectionBackground": "#166534",
      "editorCursor.foreground": "#22c55e",
    },
  });
  handshakeThemesInstalled = true;
}

export interface ConfiguredEditorOptions
  extends monaco.editor.IStandaloneEditorConstructionOptions {
  container: HTMLElement;
}

export interface ConfiguredDiffEditorOptions
  extends monaco.editor.IDiffEditorConstructionOptions {
  container: HTMLElement;
}

/**
 * Creates a standalone Monaco editor with the bundled-worker environment
 * installed. This is the single product entry point for Monaco mounting;
 * editor surfaces must not import monaco-editor directly (keeps worker and
 * failure policy in one place).
 */
export function createConfiguredEditor(
  options: ConfiguredEditorOptions,
): monaco.editor.IStandaloneCodeEditor {
  ensureMonacoEnvironment();
  ensureHandshakeMonacoThemes();
  ensureHandshakeCodeIntelligenceProviders(monaco);
  const { container, ...editorOptions } = options;
  try {
    return monaco.editor.create(container, {
      automaticLayout: true,
      minimap: { enabled: true },
      stickyScroll: { enabled: true },
      inlayHints: { enabled: "on" },
      columnSelection: true,
      multiCursorModifier: "alt",
      multiCursorPaste: "spread",
      multiCursorMergeOverlapping: true,
      ...editorOptions,
    });
  } catch (error) {
    const failure = {
      dependency: "monaco-editor",
      component: "editor",
      phase: "editor_mount" as const,
      cause: error instanceof Error ? error.message : String(error),
    };
    dependencyFailures.report({
      ...failure,
      message: formatDependencyFailureMessage(failure),
    });
    throw error;
  }
}

/**
 * Creates a standalone Monaco diff editor through the same bundled-worker
 * setup and typed dependency-failure path as regular code editors.
 */
export function createConfiguredDiffEditor(
  options: ConfiguredDiffEditorOptions,
): monaco.editor.IStandaloneDiffEditor {
  ensureMonacoEnvironment();
  ensureHandshakeMonacoThemes();
  ensureHandshakeCodeIntelligenceProviders(monaco);
  const { container, ...editorOptions } = options;
  try {
    return monaco.editor.createDiffEditor(container, {
      automaticLayout: true,
      minimap: { enabled: true },
      stickyScroll: { enabled: true },
      inlayHints: { enabled: "on" },
      readOnly: true,
      ...editorOptions,
    });
  } catch (error) {
    const failure = {
      dependency: "monaco-editor",
      component: "diff-editor",
      phase: "editor_mount" as const,
      cause: error instanceof Error ? error.message : String(error),
    };
    dependencyFailures.report({
      ...failure,
      message: formatDependencyFailureMessage(failure),
    });
    throw error;
  }
}

/**
 * Proves the TypeScript language worker actually booted by performing a real
 * round-trip (used by the offline Playwright proof, MT-020/MT-030).
 *
 * monaco registers the TypeScript language mode through a lazy dynamic import
 * after the first typescript model is created; calling getTypeScriptWorker()
 * too early throws "TypeScript not registered!". Bounded retry until the mode
 * lands (all assets are local, so this converges quickly offline).
 */
// monaco-editor 0.55 typing gap: the ESM editor.api.d.ts stubs
// `languages.typescript` as `{ deprecated: true }` while the runtime object is
// fully populated by the bundled typescript contribution (the contribution's
// own .d.ts is empty `export {}`). The offline Playwright spec proves the
// runtime shape; this narrow type restores the documented API surface.
interface TypescriptWorkerNamespace {
  getTypeScriptWorker(): Promise<
    (...uris: monaco.Uri[]) => Promise<{
      getSyntacticDiagnostics(fileName: string): Promise<unknown[]>;
    }>
  >;
}

export async function proveTypescriptWorkerRoundTrip(
  model: monaco.editor.ITextModel,
  timeoutMs = 30_000,
): Promise<boolean> {
  const tsNamespace = monaco.languages.typescript as unknown as TypescriptWorkerNamespace;
  const deadline = Date.now() + timeoutMs;
  let lastError: unknown = null;
  while (Date.now() < deadline) {
    try {
      const getWorker = await tsNamespace.getTypeScriptWorker();
      const worker = await getWorker(model.uri);
      const diagnostics = await worker.getSyntacticDiagnostics(model.uri.toString());
      return Array.isArray(diagnostics);
    } catch (error) {
      lastError = error;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error(`typescript worker round-trip timed out: ${String(lastError)}`);
}

export type MonacoWorkerProofs = Record<MonacoWorkerKind, string>;

const PROOF_LABELS: Record<MonacoWorkerKind, string> = {
  editor: "editor",
  typescript: "typescript",
  json: "json",
  css: "css",
  html: "html",
};

/**
 * MT-235 runtime proof: construct every locally bundled Monaco worker kind
 * through the same MonacoEnvironment route the editor uses. The TypeScript
 * worker gets the stronger language-service round-trip; the other workers are
 * held alive so the offline Playwright request ledger can observe each local
 * chunk request before the page closes.
 */
export async function proveBundledMonacoWorkers(
  model: monaco.editor.ITextModel,
): Promise<MonacoWorkerProofs> {
  ensureMonacoEnvironment();
  const environment = (globalThis as { MonacoEnvironment?: monaco.Environment }).MonacoEnvironment;
  if (!environment?.getWorker) throw new Error("MonacoEnvironment.getWorker missing");

  const proofs = {} as MonacoWorkerProofs;
  for (const kind of BUNDLED_MONACO_WORKER_KINDS) {
    if (kind === "typescript") {
      const responded = await proveTypescriptWorkerRoundTrip(model);
      proofs.typescript = responded ? "ts-worker-responded" : "ts-worker-no-response";
      continue;
    }
    const worker = await environment.getWorker(`mt235-${kind}`, PROOF_LABELS[kind]);
    proofWorkers.push(worker);
    proofs[kind] = "worker-constructed";
  }
  return proofs;
}

export { monaco };
