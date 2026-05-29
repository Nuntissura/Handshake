import { defineConfig } from "@playwright/test";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(appDir, "..");
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT
  ?? path.resolve(repoRoot, "..", "..", "Handshake_Artifacts");
const bootstrapBaselineHash = "0".repeat(64);

const visualMatrixProjects = [
  {
    name: "visual-normal-1280-light-en-US",
    use: {
      viewport: { width: 1280, height: 720 },
      colorScheme: "light" as const,
      locale: "en-US",
    },
    metadata: {
      capture_matrix_entry: {
        scenario_id: "visual-normal-dashboard",
        route: "fixture:normal",
        viewport: { width: 1280, height: 720 },
        color_scheme: "light",
        locale: "en-US",
        edge_state_tag: "normal",
        wait_for: "[data-testid='capture-root']",
        mask_selectors: ["[data-testid='volatile-clock']"],
        baseline_hash: bootstrapBaselineHash,
      },
    },
  },
  {
    name: "visual-constrained-390-dark-en-US",
    use: {
      viewport: { width: 390, height: 844 },
      colorScheme: "dark" as const,
      locale: "en-US",
    },
    metadata: {
      capture_matrix_entry: {
        scenario_id: "visual-constrained-mobile",
        route: "fixture:constrained",
        viewport: { width: 390, height: 844 },
        color_scheme: "dark",
        locale: "en-US",
        edge_state_tag: "normal",
        wait_for: "[data-testid='capture-root']",
        mask_selectors: ["[data-testid='volatile-clock']"],
        baseline_hash: bootstrapBaselineHash,
      },
    },
  },
  {
    name: "visual-edge-empty-1024-light-nl-NL",
    use: {
      viewport: { width: 1024, height: 768 },
      colorScheme: "light" as const,
      locale: "nl-NL",
    },
    metadata: {
      capture_matrix_entry: {
        scenario_id: "visual-edge-empty-state",
        route: "fixture:empty",
        viewport: { width: 1024, height: 768 },
        color_scheme: "light",
        locale: "nl-NL",
        edge_state_tag: "empty",
        wait_for: "[data-testid='capture-root']",
        mask_selectors: ["[data-testid='volatile-clock']"],
        baseline_hash: bootstrapBaselineHash,
      },
    },
  },
];

process.env.PLAYWRIGHT_BROWSERS_PATH =
  process.env.PLAYWRIGHT_BROWSERS_PATH
  ?? path.join(artifactRoot, "playwright-browsers");

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

export default defineConfig({
  testDir: repoRoot,
  testMatch: ["tests/visual/**/*.spec.ts"],
  timeout: 30_000,
  workers: 1,
  fullyParallel: false,
  reporter: [["list"]],
  outputDir: path.join(artifactRoot, "playwright-results"),
  projects: visualMatrixProjects.map((project) => ({
    ...project,
    testMatch: ["tests/visual/**/*.spec.ts"],
  })),
  expect: {
    toHaveScreenshot: {
      maxDiffPixelRatio: 0.01,
    },
  },
  use: {
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
});
