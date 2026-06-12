// WP-KERNEL-009 / MT-164 — TiptapAutoCodeBlockRules.
//
// A Tiptap Extension that wires the auto-code-block detection (auto_code_block.ts)
// into the editor so prose becomes an embedded Monaco code block (MT-165):
//   - INPUT RULE: typing ```ts<space> at the start of a line replaces the line
//     with a monacoCodeBlock of the detected language,
//   - PASTE: a real ProseMirror handlePaste plugin (iteration-3 H6 — the
//     helper previously had ZERO product callers) converts pasted blobs that
//     contain ```fenced``` or indented code into monacoCodeBlocks, preserving
//     any prose interleaved between the fences (no data loss),
//   - COMMANDS: insertCodeBlockFromSlash(language) for the slash-menu/command
//     palette (MT-170), proseToCodeBlock for prose→code, and codeToProse
//     (iteration-3 M4) as the reverse conversion — the round-trip the module
//     contract always promised.
//
// Requires the MonacoCodeBlockNode (MT-165) to be present in the editor schema;
// it constructs that node type. Kept additive — it does not replace StarterKit's
// own codeBlock; the operator gets the Monaco-backed block via these rules and
// the toolbar.

import { Extension, nodeInputRule, type Editor } from "@tiptap/core";
import type { JSONContent } from "@tiptap/core";
import { Plugin, PluginKey, NodeSelection } from "@tiptap/pm/state";
import {
  detectFenceOpener,
  detectFencedBlocks,
  detectIndentedCodeBlock,
  segmentPasteText,
} from "../editor/auto_code_block";
import { makeCodeBlockAttrs } from "../editor/code_block_serialization";
import { FENCE_OPEN_REGEX } from "../editor/auto_code_block";
import { DEFAULT_CODE_LANGUAGE } from "../monaco/language_registry";

const MONACO_CODE_BLOCK_NODE = "monacoCodeBlock";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    autoCodeBlock: {
      /** Inserts a Monaco code block (slash menu / command palette). */
      insertCodeBlockFromSlash: (language?: string) => ReturnType;
      /** Converts the current paragraph's text into a Monaco code block. */
      proseToCodeBlock: (language?: string) => ReturnType;
      /** Converts the selected Monaco code block back to prose paragraphs. */
      codeToProse: () => ReturnType;
    };
  }
}

export const AutoCodeBlockRules = Extension.create({
  name: "autoCodeBlockRules",

  addInputRules() {
    // Type-as-you-go: ```lang<space> at line start → a Monaco code block of the
    // detected language. nodeInputRule replaces the matched text with the node.
    return [
      nodeInputRule({
        find: FENCE_OPEN_REGEX,
        type: this.editor.schema.nodes[MONACO_CODE_BLOCK_NODE],
        getAttributes: (match) => {
          const language = detectFenceOpener(match[0]) ?? DEFAULT_CODE_LANGUAGE;
          return makeCodeBlockAttrs(language, "");
        },
      }),
    ];
  },

  addProseMirrorPlugins() {
    // Iteration-3 H6: the REAL paste wiring. ProseMirror calls handlePaste for
    // pastes landing in the prose surface (Monaco owns pastes inside the code
    // island); returning true consumes the paste.
    const editor = this.editor;
    return [
      new Plugin({
        key: new PluginKey("autoCodeBlockPaste"),
        props: {
          handlePaste: (_view, event) => {
            const text = event.clipboardData?.getData("text/plain") ?? "";
            if (!text) return false;
            return handleCodeBlockPaste(editor, text);
          },
        },
      }),
    ];
  },

  addCommands() {
    return {
      insertCodeBlockFromSlash:
        (language) =>
        ({ commands }) =>
          commands.insertContent({
            type: MONACO_CODE_BLOCK_NODE,
            attrs: makeCodeBlockAttrs(language, ""),
          }),
      proseToCodeBlock:
        (language) =>
        ({ state, chain }) => {
          const { from, to } = state.selection;
          const text = state.doc.textBetween(from, to, "\n");
          return chain()
            .insertContent({
              type: MONACO_CODE_BLOCK_NODE,
              attrs: makeCodeBlockAttrs(language, text),
            })
            .run();
        },
      codeToProse:
        () =>
        ({ state, chain }) => {
          // Iteration-3 M4: the reverse conversion. Resolve the code block the
          // selection is ON (NodeSelection) or AT (caret position).
          const selection = state.selection;
          let pos = -1;
          let node = null;
          if (
            selection instanceof NodeSelection &&
            selection.node.type.name === MONACO_CODE_BLOCK_NODE
          ) {
            pos = selection.from;
            node = selection.node;
          } else {
            const at = state.doc.nodeAt(selection.from);
            if (at && at.type.name === MONACO_CODE_BLOCK_NODE) {
              pos = selection.from;
              node = at;
            }
          }
          if (!node) return false;
          const code = String(node.attrs.code ?? "");
          const paragraphs: JSONContent[] = code.split("\n").map((line) => ({
            type: "paragraph",
            ...(line.length > 0 ? { content: [{ type: "text", text: line }] } : {}),
          }));
          return chain()
            .insertContentAt(
              { from: pos, to: pos + node.nodeSize },
              paragraphs.length > 0 ? paragraphs : [{ type: "paragraph" }],
            )
            .run();
        },
    };
  },
});

/**
 * Pure helper the paste path uses: given pasted text, returns the ordered list
 * of code blocks to create (fenced regions first; otherwise a single indented
 * block; otherwise none). Exposed for direct unit testing of the paste decision
 * without driving the clipboard through jsdom.
 */
export function codeBlocksFromPaste(
  text: string,
): Array<{ language: string; code: string }> {
  const fenced = detectFencedBlocks(text);
  if (fenced.length > 0) return fenced;
  const indented = detectIndentedCodeBlock(text);
  if (indented !== null) return [{ language: DEFAULT_CODE_LANGUAGE, code: indented }];
  return [];
}

/**
 * Converts a pasted plain-text blob into editor content when (and only when)
 * it contains fenced or indented code. Prose interleaved between fences is
 * PRESERVED as paragraphs (iteration-3 H6 — the earlier version dropped it).
 * Returns true when it consumed the paste, false to let the default paste run.
 * Wired into the live editor via this extension's handlePaste plugin.
 */
export function handleCodeBlockPaste(
  editor: Editor,
  pastedText: string,
): boolean {
  const segments = segmentPasteText(pastedText);
  if (segments === null) return false;
  const content: JSONContent[] = [];
  for (const segment of segments) {
    if (segment.kind === "code") {
      content.push({
        type: MONACO_CODE_BLOCK_NODE,
        attrs: makeCodeBlockAttrs(segment.language, segment.text),
      });
    } else {
      for (const line of segment.text.split("\n")) {
        if (line.trim().length === 0) continue;
        content.push({ type: "paragraph", content: [{ type: "text", text: line }] });
      }
    }
  }
  if (content.length === 0) return false;
  return editor.chain().insertContent(content).run();
}
