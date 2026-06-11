// WP-KERNEL-009 / MT-020 + MT-030 — dependency-policy Playwright lane.
//
// Runs tests/dependency_policy/*.spec.ts against the BUILT harness
// (app/dist-harness, produced by `pnpm run build:harness`; the global setup
// builds it when missing). Separate from the visual-matrix lane so the
// offline dependency proofs do not multiply across viewport projects.
//
// Run: cd app && pnpm run test:dependency-policy

import { defineConfig } from "@playwright/test";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(appDir, "..");

// Spec files live at repoRoot/tests (outside app/), so node module resolution
// cannot walk into app/node_modules on its own. Same NODE_PATH bridge the
// visual lane uses (app/playwright.config.ts).
const require = createRequire(import.meta.url);
const moduleApi = require("node:module") as {
  Module?: { _initPaths?: () => void };
  _initPaths?: () => void;
};
process.env.NODE_PATH = [
  path.join(appDir, "node_modules"),
  process.env.NODE_PATH,
].filter(Boolean).join(path.delimiter);
(moduleApi.Module?._initPaths ?? moduleApi._initPaths)?.();
// Same sibling artifact root the cargo target-dir uses (.cargo/config.toml):
// <worktrees-parent>/Handshake_Artifacts. Overridable for portability.
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT
  ?? path.resolve(repoRoot, "..", "Handshake_Artifacts");

export default defineConfig({
  testDir: path.join(repoRoot, "tests", "dependency_policy"),
  testMatch: ["**/*.spec.ts"],
  globalSetup: path.join(repoRoot, "tests", "dependency_policy", "global_setup.ts"),
  timeout: 120_000,
  workers: 1,
  fullyParallel: false,
  reporter: [["list"]],
  outputDir: path.join(artifactRoot, "dependency-policy-results"),
  use: {
    // Quiet mode: never a foreground window.
    headless: true,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium-offline-dependency-proof" }],
});
