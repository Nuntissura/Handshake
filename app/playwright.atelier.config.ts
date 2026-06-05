import { defineConfig, devices } from "@playwright/test";
import path from "node:path";

// Standalone, headless Playwright config for the WP-KERNEL-005 Atelier
// front-end navigation proof. Unlike playwright.config.ts (WebView2/CDP visual
// matrix against a running Tauri window), this drives the real React app served
// by Vite in headless Chromium against the backend HTTP API on :37501 -- no
// desktop window is opened (honors the non-intrusive/QUIET operation rule).
// `app` is an ESM package, so use import.meta.dirname (Node 20+) not __dirname.
const here = import.meta.dirname;
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT ??
  path.join(here, "..", "..", "..", "Handshake_Artifacts");

export default defineConfig({
  testDir: path.join(here, "tests", "atelier"),
  testMatch: ["**/*.spec.ts"],
  timeout: 60_000,
  workers: 1,
  fullyParallel: false,
  reporter: [["list"]],
  outputDir: path.join(artifactRoot, "playwright-atelier-results"),
  use: {
    baseURL: process.env.ATELIER_BASE_URL ?? "http://localhost:1420",
    headless: true,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "pnpm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: true,
    timeout: 120_000,
    stdout: "ignore",
    stderr: "pipe",
  },
});
