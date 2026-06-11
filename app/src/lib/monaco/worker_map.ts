// WP-KERNEL-009 / MT-020 — Monaco worker label → bundled worker kind mapping.
//
// Pure logic split out of setup.ts so the mapping is unit-testable in jsdom
// without importing the monaco-editor runtime (which requires a real browser).
// The worker kinds correspond 1:1 to the locally bundled Vite `?worker`
// entries in setup.ts — NO CDN, NO runtime URL construction from config.

export type MonacoWorkerKind = "editor" | "typescript" | "json" | "css" | "html";

/**
 * Maps a MonacoEnvironment.getWorker `label` to the bundled worker kind.
 * Mirrors monaco-editor's language→worker contract:
 *  - json                          → json.worker
 *  - css/scss/less                 → css.worker
 *  - html/handlebars/razor         → html.worker
 *  - typescript/javascript         → ts.worker
 *  - everything else               → editor.worker (editor core services)
 */
export function workerKindForLabel(label: string): MonacoWorkerKind {
  switch (label) {
    case "json":
      return "json";
    case "css":
    case "scss":
    case "less":
      return "css";
    case "html":
    case "handlebars":
    case "razor":
      return "html";
    case "typescript":
    case "javascript":
      return "typescript";
    default:
      return "editor";
  }
}

/** All worker kinds that must ship as locally bundled assets (MT-027). */
export const BUNDLED_MONACO_WORKER_KINDS: readonly MonacoWorkerKind[] = [
  "editor",
  "typescript",
  "json",
  "css",
  "html",
];
