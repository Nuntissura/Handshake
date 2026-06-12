// WP-KERNEL-009 / MT-164 — TiptapAutoCodeBlockRules.
//
// A Tiptap Extension that wires the auto-code-block detection (auto_code_block.ts)
// into the editor so prose becomes an embedded Monaco code block (MT-165):
//   - INPUT RULE: typing ```ts<space> at the start of a line replaces the line
//     with a monacoCodeBlock of the detected language,
//   - PASTE RULE: pasting text containing ```lang … ``` fenced regions converts
//     each region into a monacoCodeBlock,
//   - COMMAND: insertCodeBlockFromSlash(language) for the slash-menu / command
//     palette (MT-170) and toggleProseToCode/codeToProse for reversible
//     conversion.
//
// Requires the MonacoCodeBlockNode (MT-165) to be present in the editor schema;
// it constructs that node type. Kept additive — it does not replace StarterKit's
// own codeBlock; the operator gets the Monaco-backed block via these rules and
// the toolbar.

import { Extension, nodeInputRule, type Editor } from "@tiptap/core";
import {
  detectFenceOpener,
  detectFencedBlocks,
  detectIndentedCodeBlock,
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
 * Inserts Monaco code block(s) for a pasted plain-text blob that contains
 * fenced or indented code. Returns true when it handled the paste (so the
 * editorProps.handlePaste caller can preventDefault), false when the blob is
 * not code (let the default paste run). This is the function the new editor's
 * editorProps.handlePaste delegates to — wiring it there avoids importing the
 * non-direct-dependency @tiptap/pm Plugin while keeping paste→code-block real.
 */
export function handleCodeBlockPaste(
  editor: Editor,
  pastedText: string,
): boolean {
  const blocks = codeBlocksFromPaste(pastedText);
  if (blocks.length === 0) return false;
  const content = blocks.map((b) => ({
    type: MONACO_CODE_BLOCK_NODE,
    attrs: makeCodeBlockAttrs(b.language, b.code),
  }));
  return editor.chain().insertContent(content).run();
}
