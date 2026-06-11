// WP-KERNEL-009 / MT-020 — builds the dependency-policy harness before the
// offline specs run, when app/dist-harness is missing or stale. The specs
// must exercise REAL built assets (no dev server, no mocks).
//
// Set HARNESS_SKIP_BUILD=1 to reuse an existing build (fast iteration).

import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { existsSync } from "node:fs";
import path from "node:path";

// CJS context (tests/dependency_policy/package.json sets type commonjs;
// Playwright transpiles these files to CJS — import.meta is unavailable).
const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const harnessIndex = path.join(appDir, "dist-harness", "harness", "dependency-policy.html");

export default function globalSetup(): void {
  if (process.env.HARNESS_SKIP_BUILD === "1" && existsSync(harnessIndex)) {
    return;
  }
  if (existsSync(harnessIndex) && process.env.HARNESS_FORCE_BUILD !== "1") {
    // Reuse the existing build; `pnpm run build:harness` refreshes it.
    return;
  }
  // Resolve the app's own vite binary (no PATH/shell assumptions — Windows-safe).
  const appRequire = createRequire(path.join(appDir, "package.json"));
  const viteCli = appRequire.resolve("vite/bin/vite.js");
  const result = spawnSync(
    process.execPath,
    [viteCli, "build", "--config", "vite.harness.config.ts"],
    { cwd: appDir, stdio: "inherit" },
  );
  if (result.status !== 0) {
    throw new Error(`harness build failed with exit code ${result.status}`);
  }
  if (!existsSync(harnessIndex)) {
    throw new Error(`harness build produced no ${harnessIndex}`);
  }
}
