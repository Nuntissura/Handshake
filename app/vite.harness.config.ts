// WP-KERNEL-009 / MT-020 — dependency-policy harness build.
//
// Builds harness/dependency-policy.html (a REAL product surface mounting the
// bundled Monaco + Tiptap stack) into app/dist-harness with relative asset
// paths, so the offline Playwright spec (MT-030) and the worker-bundling
// check (MT-027) operate on genuine built artifacts. Workers are emitted as
// local chunks by Vite's `?worker` handling — no CDN, no runtime downloads.
//
// Build:  pnpm run build:harness
// Output: app/dist-harness/harness/dependency-policy.html (+ assets/)

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const appDir = dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  // Relative base: the harness must load from any local static server or
  // packaged path without host assumptions (disk-agnostic, offline).
  base: "./",
  build: {
    outDir: "dist-harness",
    emptyOutDir: true,
    rollupOptions: {
      input: resolve(appDir, "harness", "dependency-policy.html"),
    },
  },
});
