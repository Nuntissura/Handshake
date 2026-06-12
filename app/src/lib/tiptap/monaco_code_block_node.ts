// WP-KERNEL-009 / MT-165 — MonacoEmbeddedCodeBlock node.
//
// The Tiptap/ProseMirror node (`monacoCodeBlock`, matching the MT-161 inventory)
// whose NodeView (MonacoCodeBlockView.tsx) mounts a real bundled Monaco editor.
// Code + language + round-trip hash live in the node attrs (MT-168 bridge), so
// the whole document content_json is one RichDocument authority record persisted
// through the rich-doc save API — no external editor, no sidecar.
//
// The node is a leaf (atom) block: ProseMirror does not own the code text (Monaco
// does, inside the NodeView), it only owns the {language, code, contentHash}
// attrs. parseHTML/renderHTML round-trip the attrs through a <pre data-*> element
// so a non-NodeView render (server projection, copy/paste as HTML) preserves the
// code, and renderText emits a fenced code block so plain-text/markdown
// projections round-trip.

import { Node, mergeAttributes } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
import { MonacoCodeBlockView } from "../../components/MonacoCodeBlockView";
import {
  makeCodeBlockAttrs,
  type MonacoCodeBlockAttrs,
} from "../editor/code_block_serialization";
import { DEFAULT_CODE_LANGUAGE } from "../monaco/language_registry";

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    monacoCodeBlock: {
      /** Inserts an embedded Monaco code block with optional language + code. */
      insertMonacoCodeBlock: (options?: {
        language?: string;
        code?: string;
      }) => ReturnType;
      /** Sets the language of the code block at the current selection. */
      setMonacoCodeBlockLanguage: (language: string) => ReturnType;
    };
  }
}

/**
 * Reads code+language out of a DOM element on parse and returns normalized
 * attrs (with a freshly computed/repaired hash). Shared by parseHTML.
 */
function parseAttrsFromElement(element: HTMLElement): MonacoCodeBlockAttrs {
  const language =
    element.getAttribute("data-language") ??
    element.querySelector("code")?.getAttribute("data-language") ??
    DEFAULT_CODE_LANGUAGE;
  const code =
    element.getAttribute("data-code") ??
    element.querySelector("code")?.textContent ??
    element.textContent ??
    "";
  return makeCodeBlockAttrs(language, code);
}

export const MonacoCodeBlockNode = Node.create({
  name: "monacoCodeBlock",
  group: "block",
  atom: true,
  // No editable ProseMirror content: Monaco owns the text via the NodeView.
  selectable: true,
  draggable: false,
  defining: true,

  addAttributes() {
    return {
      language: {
        default: DEFAULT_CODE_LANGUAGE,
        parseHTML: (element) => element.getAttribute("data-language") ?? DEFAULT_CODE_LANGUAGE,
        renderHTML: (attributes) => ({ "data-language": String(attributes.language) }),
      },
      code: {
        default: "",
        parseHTML: (element) => element.getAttribute("data-code") ?? "",
        renderHTML: (attributes) => ({ "data-code": String(attributes.code) }),
      },
      contentHash: {
        default: "",
        parseHTML: (element) => element.getAttribute("data-rt-hash") ?? "",
        renderHTML: (attributes) => ({ "data-rt-hash": String(attributes.contentHash) }),
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: "pre[data-testid='monaco-code-block-serialized']",
        // Must outrank StarterKit's generic `pre` codeBlock rule (default
        // priority 50), otherwise a serialized Monaco block re-imports as a
        // plain code block and loses language/hash (MT-244 round-trip).
        priority: 100,
        getAttrs: (node) => (node instanceof HTMLElement ? parseAttrsFromElement(node) : false),
      },
    ];
  },

  renderHTML({ node, HTMLAttributes }) {
    const language = String(node.attrs.language || DEFAULT_CODE_LANGUAGE);
    const code = String(node.attrs.code ?? "");
    // <pre><code> with data-* so a plain-HTML render (no NodeView) still carries
    // the code text and language for round-trip.
    return [
      "pre",
      mergeAttributes(HTMLAttributes, {
        "data-testid": "monaco-code-block-serialized",
        "data-language": language,
      }),
      ["code", { class: `language-${language}`, "data-language": language }, code],
    ];
  },

  renderText({ node }) {
    const language = String(node.attrs.language || "");
    const code = String(node.attrs.code ?? "");
    // Fenced code block so markdown/plain-text projection round-trips.
    return "```" + language + "\n" + code + "\n```";
  },

  addNodeView() {
    return ReactNodeViewRenderer(MonacoCodeBlockView);
  },

  addCommands() {
    return {
      insertMonacoCodeBlock:
        (options) =>
        ({ commands }) => {
          const attrs = makeCodeBlockAttrs(options?.language, options?.code ?? "");
          return commands.insertContent({ type: this.name, attrs });
        },
      setMonacoCodeBlockLanguage:
        (language) =>
        ({ commands, state }) => {
          const { from } = state.selection;
          const nodeAt = state.doc.nodeAt(from);
          const code = nodeAt && nodeAt.type.name === this.name ? String(nodeAt.attrs.code ?? "") : "";
          // Iteration-3 M10: mint through the single minting point so the
          // language is normalized and the hash always matches {language, code}.
          return commands.updateAttributes(this.name, makeCodeBlockAttrs(language, code));
        },
    };
  },
});
