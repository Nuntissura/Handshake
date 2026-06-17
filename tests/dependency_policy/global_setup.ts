// WP-KERNEL-009 / MT-020 — builds the dependency-policy harness before the
// offline specs run, when app/dist-harness is missing or stale. The specs
// must exercise REAL built assets (no dev server, no mocks).
//
// Set HARNESS_SKIP_BUILD=1 to reuse an existing build (fast iteration).

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

// CJS context (tests/dependency_policy/package.json sets type commonjs;
// Playwright transpiles these files to CJS — import.meta is unavailable).
const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const harnessDir = path.join(appDir, "dist-harness", "harness");
const requiredHarnesses = [
  "dependency-policy.html",
  "rich-editor.html",
  "rich-editor-embeds.html",
  "rich-editor-collaboration.html",
  "rich-document-diff.html",
  "editor-draft-recovery.html",
  "mt249-code-intelligence.html",
].map((fileName) => path.join(harnessDir, fileName));

function allHarnessesExist(): boolean {
  return requiredHarnesses.every((harnessPath) => existsSync(harnessPath));
}

export default function globalSetup(): void {
  if (process.env.HARNESS_SKIP_BUILD === "1" && allHarnessesExist()) {
    return;
  }
  if (allHarnessesExist() && process.env.HARNESS_FORCE_BUILD !== "1") {
    // Reuse the existing build; `pnpm run build:harness` refreshes it.
    return;
  }
  // Resolve the app's own vite CLI through its package.json `bin` field
  // (no PATH/shell assumptions — Windows-safe). Direct subpath resolution
  // ("vite/bin/vite.js") broke in vite 7: the bin file is no longer an
  // exported subpath, but "./package.json" still is.
  const appRequire = createRequire(path.join(appDir, "package.json"));
  const vitePkgPath = appRequire.resolve("vite/package.json");
  const vitePkg = JSON.parse(readFileSync(vitePkgPath, "utf8")) as {
    bin?: string | Record<string, string>;
  };
  const binRel = typeof vitePkg.bin === "string" ? vitePkg.bin : vitePkg.bin?.vite;
  if (!binRel) throw new Error(`vite package.json at ${vitePkgPath} declares no bin entry`);
  const viteCli = path.join(path.dirname(vitePkgPath), binRel);
  const result = spawnSync(
    process.execPath,
    [viteCli, "build", "--config", "vite.harness.config.ts"],
    { cwd: appDir, stdio: "inherit" },
  );
  if (result.status !== 0) {
    throw new Error(`harness build failed with exit code ${result.status}`);
  }
  const missing = requiredHarnesses.filter((harnessPath) => !existsSync(harnessPath));
  if (missing.length > 0) {
    throw new Error(`harness build missing expected outputs: ${missing.join(", ")}`);
  }
}
