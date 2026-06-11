// WP-KERNEL-009 / MT-020 (baseline) — Tiptap mount for the dependency-policy
// harness. MT-021 upgrades this to the full WP-009 extension set
// (tables, task lists, links, mentions/tags) via buildWp009ExtensionSet.

import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";

/**
 * Mounts a real Tiptap editor into `host` and returns the active extension
 * names (validator-readable proof of which extensions instantiated).
 */
export function mountWp009TiptapProof(host: HTMLElement): string[] {
  const editor = new Editor({
    element: host,
    extensions: [StarterKit.configure({ heading: { levels: [1, 2, 3] } })],
    content: {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [{ type: "text", text: "WP-KERNEL-009 offline Tiptap proof" }],
        },
      ],
    },
  });
  return editor.extensionManager.extensions.map((extension) => extension.name);
}
