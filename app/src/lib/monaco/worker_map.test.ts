// WP-KERNEL-009 / MT-020 — Monaco worker label mapping (pure logic).
//
// The real worker-boot proof runs against built assets in the offline
// Playwright spec (tests/dependency_policy/offline_editor_load.spec.ts).
// This unit test pins the label→bundled-worker contract that setup.ts
// installs into MonacoEnvironment.

import { describe, expect, it } from "vitest";
import { BUNDLED_MONACO_WORKER_KINDS, workerKindForLabel } from "./worker_map";

describe("MT-020 monaco worker map", () => {
  it("routes language labels to their bundled workers", () => {
    expect(workerKindForLabel("typescript")).toBe("typescript");
    expect(workerKindForLabel("javascript")).toBe("typescript");
    expect(workerKindForLabel("json")).toBe("json");
    expect(workerKindForLabel("css")).toBe("css");
    expect(workerKindForLabel("scss")).toBe("css");
    expect(workerKindForLabel("less")).toBe("css");
    expect(workerKindForLabel("html")).toBe("html");
    expect(workerKindForLabel("handlebars")).toBe("html");
    expect(workerKindForLabel("razor")).toBe("html");
  });

  it("routes everything else to the core editor worker", () => {
    expect(workerKindForLabel("editorWorkerService")).toBe("editor");
    expect(workerKindForLabel("rust")).toBe("editor");
    expect(workerKindForLabel("")).toBe("editor");
  });

  it("declares all five bundled worker kinds for the MT-027 bundling proof", () => {
    expect([...BUNDLED_MONACO_WORKER_KINDS].sort()).toEqual(
      ["css", "editor", "html", "json", "typescript"].sort(),
    );
  });
});
