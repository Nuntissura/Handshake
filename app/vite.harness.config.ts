// WP-KERNEL-009 / MT-020 (+ MT-175/176) — offline harness build.
//
// Builds the REAL product harness surfaces into app/dist-harness with relative
// asset paths, so the offline Playwright specs operate on genuine built
// artifacts. Workers are emitted as local chunks by Vite's `?worker` handling —
// no CDN, no runtime downloads.
//   - harness/dependency-policy.html: bundled Monaco + Tiptap stack (MT-020/027/030).
//   - harness/rich-editor.html: the INTEGRATED RichTextEditor with an embedded
//     Monaco code block + typed wikilinks (MT-175 no-external-app proof +
//     MT-176 round-trip offline visual test).
//
// Build:  pnpm run build:harness
// Output: app/dist-harness/harness/{dependency-policy,rich-editor}.html (+ assets/)

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
      input: {
        "dependency-policy": resolve(appDir, "harness", "dependency-policy.html"),
        "rich-editor": resolve(appDir, "harness", "rich-editor.html"),
        // MT-244: embeds + save-to-format + find/replace offline proofs.
        "rich-editor-embeds": resolve(appDir, "harness", "rich-editor-embeds.html"),
      },
    },
  },
});
