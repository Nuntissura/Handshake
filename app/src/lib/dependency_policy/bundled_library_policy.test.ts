// WP-KERNEL-009 / MT-018 — BundledLibraryPolicy.
//
// Enforces that the WP-009 editor stack (Tiptap, Monaco, Yjs/CRDT, xterm,
// Excalidraw and their prosemirror substrate) is consumed ONLY as
// lockfile-governed bundled libraries:
//  - every editor-stack dependency in app/package.json resolves from the npm
//    registry (integrity hash; no link:/file:/git:/tarball-offsite entries),
//  - no runtime CDN host is referenced anywhere in product frontend source,
//  - the scanners themselves are proven against negative fixtures, so a
//    regression in the scanner cannot silently pass.

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterAll, describe, expect, it } from "vitest";
import {
  auditEditorStackResolution,
  loadAllowlist,
  scanCdnReferences,
  scanFilesForPatterns,
  selfExemptPathSet,
  walkSourceFiles,
} from "../../../scripts/lib/dependency_policy_scans.mjs";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");
const allowlist = loadAllowlist(repoRoot);

const tempDirs: string[] = [];
afterAll(() => {
  for (const dir of tempDirs) rmSync(dir, { recursive: true, force: true });
});

describe("MT-018 bundled library policy", () => {
  it("resolves every editor-stack dependency from the npm registry (lockfile-governed)", () => {
    const { violations, audited } = auditEditorStackResolution({ repoRoot, allowlist });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
    // The editor stack must actually be present — an empty audit would mean
    // the policy is scanning nothing.
    const auditedNames = audited.map((a) => a.package);
    expect(auditedNames).toContain("@tiptap/core");
    expect(auditedNames).toContain("yjs");
    expect(auditedNames.length).toBeGreaterThanOrEqual(5);
  });

  it("finds no runtime CDN host references in product source", () => {
    const { violations } = scanCdnReferences({ repoRoot, allowlist });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
  });

  it("scanner catches CDN references in a negative fixture (tripwire is alive)", () => {
    const dir = mkdtempSync(join(tmpdir(), "hsk-cdn-fixture-"));
    tempDirs.push(dir);
    const bad = join(dir, "bad_loader.ts");
    writeFileSync(
      bad,
      `export const monacoBase = "https://cdn.jsdelivr.net/npm/monaco-editor@0.52.0/min/vs";\n`,
      "utf8",
    );
    const { violations } = scanFilesForPatterns({
      repoRoot: dir,
      files: [bad],
      patterns: allowlist.forbidden_runtime_dependency_classes.find(
        (c) => c.id === "cdn_runtime_asset",
      )!.source_scan_patterns,
    });
    expect(violations.length).toBeGreaterThanOrEqual(1);
    expect(violations[0].pattern).toBe("cdn.jsdelivr.net");
    expect(violations[0].path).toBe("bad_loader.ts");
  });

  it("forbids the CDN-loading monaco wrapper packages by name", () => {
    const cdnClass = allowlist.forbidden_runtime_dependency_classes.find(
      (c) => c.id === "cdn_runtime_asset",
    );
    expect(cdnClass?.npm_package_names).toContain("@monaco-editor/loader");
    expect(cdnClass?.npm_package_names).toContain("@monaco-editor/react");
  });
});

// H3 — the blanket substring exemption ("dependency_policy"/"dependency-policy")
// previously hid the REAL product runtime file dependency_policy_harness.tsx
// from the CDN + docker source scans. Replaced with a precise exact-path
// allowlist of only the policy-data/scanner/test files.
describe("MT-018 H3 precise self-exempt allowlist (real product file is scanned)", () => {
  const harnessRel = "app/src/harness/dependency_policy_harness.tsx";

  it("does NOT exempt the real harness product file (it is scanned like any source)", () => {
    const exemptSet = selfExemptPathSet(allowlist);
    expect(exemptSet.has(harnessRel)).toBe(false);
    // The blanket substring escape hatch must be gone from the allowlist data.
    expect(JSON.stringify(allowlist.scan_self_exempt_paths.paths)).not.toContain("harness");
  });

  it("includes dependency_policy_harness.tsx in the walked product source set", () => {
    const walked = walkSourceFiles(join(repoRoot, "app", "src")).map((f) =>
      f.split(/[\\/]/).join("/"),
    );
    expect(walked.some((p) => p.endsWith("harness/dependency_policy_harness.tsx"))).toBe(true);
  });

  it("CATCHES an injected CDN string in the harness path (no longer blanket-exempt) (evasion case)", () => {
    // Simulate the harness file carrying a CDN reference: with the OLD blanket
    // substring exemption this would have been silently skipped. With the
    // precise allowlist (harness NOT listed) it is a violation.
    const dir = mkdtempSync(join(tmpdir(), "hsk-h3-harness-"));
    tempDirs.push(dir);
    const harnessFile = join(dir, "dependency_policy_harness.tsx");
    writeFileSync(
      harnessFile,
      `export const loader = "https://cdn.jsdelivr.net/npm/monaco-editor/min/vs/loader.js";\n`,
      "utf8",
    );
    const cls = allowlist.forbidden_runtime_dependency_classes.find(
      (c) => c.id === "cdn_runtime_asset",
    )!;
    const { violations } = scanFilesForPatterns({
      repoRoot: dir,
      files: [harnessFile],
      patterns: cls.source_scan_patterns,
      // The harness path is NOT in the exempt set, so a substring like
      // "dependency_policy" in its name must not save it.
      exactExemptPaths: selfExemptPathSet(allowlist),
    });
    expect(violations.length).toBeGreaterThanOrEqual(1);
    expect(violations[0].pattern).toBe("cdn.jsdelivr.net");
  });

  it("KEEPS the legitimate policy-data/fixture files exempt (exact-path match)", () => {
    const exemptSet = selfExemptPathSet(allowlist);
    // The allowlist JSON and the negative-test files DO embed forbidden literals
    // as data; they must remain exempt so their fixtures do not trip themselves.
    expect(exemptSet.has("app/src/lib/dependency_policy/runtime_dependency_allowlist.json")).toBe(
      true,
    );
    expect(exemptSet.has("app/src/lib/dependency_policy/bundled_library_policy.test.ts")).toBe(true);
    expect(exemptSet.has("app/src/lib/dependency_policy/no_docker_default.test.ts")).toBe(true);
    // And the live scan over the real tree still passes (exempt files don't trip,
    // and no real product file — including the harness — carries a CDN host).
    const { violations } = scanCdnReferences({ repoRoot, allowlist });
    expect(violations, JSON.stringify(violations, null, 2)).toHaveLength(0);
  });

  it("a real exact-path match is required — a sibling decoy file is NOT exempt", () => {
    const exemptSet = selfExemptPathSet(allowlist);
    // A file whose path merely CONTAINS an exempt substring is not exempt.
    expect(exemptSet.has("app/src/lib/dependency_policy/runtime_dependency_allowlist.json.bak")).toBe(
      false,
    );
    expect(exemptSet.has("app/src/lib/dependency_policy_decoy.ts")).toBe(false);
  });
});
