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
        // MT-234: compact editor visual regression fixture.
        "editor-visual-regression": resolve(appDir, "harness", "editor-visual-regression.html"),
        // MT-246: same-document split editor CRDT consistency proof.
        "rich-editor-collaboration": resolve(appDir, "harness", "rich-editor-collaboration.html"),
        // MT-247: history-pair rich-document diff + Monaco diff proof.
        "rich-document-diff": resolve(appDir, "harness", "rich-document-diff.html"),
        // MT-255: backend-backed rich editor draft recovery proof.
        "editor-draft-recovery": resolve(appDir, "harness", "editor-draft-recovery.html"),
        // MT-249: real-backend Monaco code-intelligence provider proof.
        "mt249-code-intelligence": resolve(appDir, "harness", "mt249-code-intelligence.html"),
        // MT-258: Loom hsLink hover-preview visual proof.
        "loom-hover-preview": resolve(appDir, "harness", "loom-hover-preview.html"),
        // MT-258: backend-backed Loom bookmarks add/remove/navigation proof.
        "loom-bookmarks": resolve(appDir, "harness", "loom-bookmarks.html"),
      },
    },
  },
});
