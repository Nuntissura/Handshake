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
