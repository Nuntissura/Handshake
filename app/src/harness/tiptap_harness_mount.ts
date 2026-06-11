// WP-KERNEL-009 / MT-021 — Tiptap mount for the dependency-policy harness.
//
// Mounts a REAL @tiptap/core Editor with the full WP-009 extension set
// (tables, task lists, links, mentions, tags) so the offline Playwright spec
// proves the complete bundled rich-editor stack boots without network.

import { Editor } from "@tiptap/core";
import { buildWp009ExtensionSet } from "../lib/tiptap/extension_set";

export interface TiptapProofMount {
  /** Active extension names (validator-readable instantiation proof). */
  extensionNames: string[];
  /** Plain-text snapshot of the live document model (MT-030 typing proof). */
  docText: () => string;
  /**
   * Places the caret in a fresh empty top-level paragraph at the document
   * start (MT-030). Keyboard caret-navigation chords (Ctrl+Home/End) are not
   * reliably handled across contenteditable/headless combinations, so the
   * offline spec positions deterministically through the model API and then
   * performs REAL keyboard typing.
   */
  focusFreshLeadingParagraph: () => void;
}

/**
 * Mounts a real Tiptap editor into `host` and returns the active extension
 * names plus a document-text accessor (proof that typed input reaches the
 * ProseMirror model, not just the DOM).
 */
export function mountWp009TiptapProof(host: HTMLElement): TiptapProofMount {
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
  return {
    extensionNames: editor.extensionManager.extensions.map((extension) => extension.name),
    docText: () => editor.getText(),
    focusFreshLeadingParagraph: () => {
      editor
        .chain()
        .insertContentAt(0, { type: "paragraph" })
        .setTextSelection(1)
        .focus()
        .run();
    },
  };
}
