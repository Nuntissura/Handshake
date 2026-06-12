// WP-KERNEL-009 / MT-167 — MonacoWorkerBundling (code-block worker binding) tests.
//
// Proves the code-block language→worker mapping resolves EVERY curated language
// to a locally bundled worker kind (no language can route to an external/CDN
// worker), and that the language-service languages bind to their dedicated
// bundled workers. The runtime "workers actually boot offline" proof is the
// MT-030/MT-175 Playwright spec; this guarantees the mapping it depends on.

import { describe, it, expect } from "vitest";
import {
  workerKindForLanguage,
  isBundledWorkerKind,
  languagesWithUnbundledWorker,
  codeBlockWorkerBindings,
} from "./worker_bundling";
import { BUNDLED_MONACO_WORKER_KINDS } from "./worker_map";

describe("code-block worker bundling (MT-167)", () => {
  it("binds language-service languages to their dedicated bundled workers", () => {
    expect(workerKindForLanguage("typescript")).toBe("typescript");
    expect(workerKindForLanguage("javascript")).toBe("typescript");
    expect(workerKindForLanguage("json")).toBe("json");
    expect(workerKindForLanguage("css")).toBe("css");
    expect(workerKindForLanguage("html")).toBe("html");
  });

  it("routes languages without a dedicated worker to the bundled editor-core worker", () => {
    for (const id of ["rust", "python", "go", "sql", "powershell", "plaintext"]) {
      expect(workerKindForLanguage(id)).toBe("editor");
    }
  });

  it("resolves EVERY curated language to a locally bundled worker kind (no external)", () => {
    expect(languagesWithUnbundledWorker()).toEqual([]);
  });

  it("only ever resolves to one of the known bundled worker kinds", () => {
    const bindings = codeBlockWorkerBindings();
    for (const kind of Object.values(bindings)) {
      expect(isBundledWorkerKind(kind)).toBe(true);
      expect(BUNDLED_MONACO_WORKER_KINDS).toContain(kind);
    }
  });
});
