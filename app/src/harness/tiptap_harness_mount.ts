// WP-KERNEL-009 / MT-021 — Tiptap mount for the dependency-policy harness.
//
// Mounts a REAL @tiptap/core Editor with the full WP-009 extension set
// (tables, task lists, links, mentions, tags) so the offline Playwright spec
// proves the complete bundled rich-editor stack boots without network.

import { Editor } from "@tiptap/core";
import { buildWp009ExtensionSet } from "../lib/tiptap/extension_set";

/**
 * Mounts a real Tiptap editor into `host` and returns the active extension
 * names (validator-readable proof of which extensions instantiated).
 */
export function mountWp009TiptapProof(host: HTMLElement): string[] {
  const editor = new Editor({
    element: host,
    extensions: buildWp009ExtensionSet({
      mentionItems: () => ["kernel-builder"],
      tagItems: () => ["wp-009"],
    }),
    content: {
      type: "doc",
      content: [
        {
          type: "paragraph",
          content: [{ type: "text", text: "WP-KERNEL-009 offline Tiptap proof" }],
        },
        {
          type: "taskList",
          content: [
            {
              type: "taskItem",
              attrs: { checked: true },
              content: [
                {
                  type: "paragraph",
                  content: [{ type: "text", text: "bundled extension set loaded" }],
                },
              ],
            },
          ],
        },
      ],
    },
  });
  return editor.extensionManager.extensions.map((extension) => extension.name);
}
