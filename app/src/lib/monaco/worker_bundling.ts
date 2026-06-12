// WP-KERNEL-009 / MT-167 — MonacoWorkerBundling (code-block worker binding).
//
// Monaco language services run in web workers. setup.ts already maps every
// worker `label` to a locally bundled Vite `?worker` constructor (no CDN, no
// runtime URL-from-config) and the MT-027 dist scanner proves the BUILT chunks
// reference only local URLs. This module is the WP-009 code-block-specific
// binding: it answers "for a given code-block language, which bundled worker
// kind serves it?" so the embedded code block (MT-165) can reason about / verify
// its language-service worker is one of the locally bundled kinds — never an
// external fetch.
//
// Pure logic over worker_map + language_registry (no monaco import), so it is
// unit-testable in jsdom. The runtime startup proof (workers actually BOOT
// offline) is the MT-030/MT-175 Playwright spec; this guarantees the mapping
// that spec relies on is correct and exhaustive.

import { workerKindForLabel, BUNDLED_MONACO_WORKER_KINDS, type MonacoWorkerKind } from "./worker_map";
import { HANDSHAKE_CODE_LANGUAGE_IDS } from "./language_registry";

/**
 * The locally bundled worker kind that backs a given code-block language id.
 * Languages without a dedicated language-service worker (rust, python, go, …)
 * are served by monaco's editor-core worker ("editor") — still locally bundled.
 * This mirrors monaco's language→worker contract via workerKindForLabel.
 */
export function workerKindForLanguage(languageId: string): MonacoWorkerKind {
  return workerKindForLabel(languageId);
}

/** True when the worker kind is one of the locally bundled (offline) kinds. */
export function isBundledWorkerKind(kind: MonacoWorkerKind): boolean {
  return BUNDLED_MONACO_WORKER_KINDS.includes(kind);
}

/**
 * Verifies every curated code-block language resolves to a LOCALLY BUNDLED
 * worker kind (i.e. no language can route to a non-bundled / external worker).
 * Returns the list of language ids that fail the check (empty == all bundled).
 */
export function languagesWithUnbundledWorker(): string[] {
  return HANDSHAKE_CODE_LANGUAGE_IDS.filter(
    (id) => !isBundledWorkerKind(workerKindForLanguage(id)),
  );
}

/** Map of curated language id → bundled worker kind (for tooling/diagnostics). */
export function codeBlockWorkerBindings(): Record<string, MonacoWorkerKind> {
  const bindings: Record<string, MonacoWorkerKind> = {};
  for (const id of HANDSHAKE_CODE_LANGUAGE_IDS) {
    bindings[id] = workerKindForLanguage(id);
  }
  return bindings;
}
