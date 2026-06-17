// WP-KERNEL-009 / MT-258 — the loomTransclusion inline-block atom node.
//
// Obsidian-parity note transclusion (`![[block]]`): a host document EMBEDS a
// LoomBlock by reference and renders the SOURCE document's live content
// read-through. The node is an `atom` block carrying ONLY { refValue } (the
// source block_id) — it never holds a copy of the source body. The host
// document's persisted ProseMirror JSON therefore contains a single
// `loomTransclusion` node, not the transcluded content (NO-COPY invariant).
//
// The NodeView (LoomTransclusionView) fetches the source document through the
// backend transclusion endpoint (getLoomBlockTransclusion in api.ts) and routes
// "Edit source" edits to the SOURCE document via saveRichDocument(source id), so
// there is exactly one authority document for the content. renderHTML emits a
// stable, content-free placeholder span carrying the attrs so getHTML/exports/
// copy-paste round-trip preserve the reference (and stay copy-free).

import { Node, mergeAttributes, nodeInputRule, nodePasteRule } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
import {
  LoomTransclusionView,
  type LoomTransclusionNodeOptions,
} from "../../components/LoomTransclusionView";
import type { EmbedResolverContext } from "./hs_link_node";

export interface LoomTransclusionAttributes {
  /** The SOURCE LoomBlock id this host node read-throughs. */
  refValue: string;
}

// `![[block-id]]` and `![[block-id|label]]` (label is ignored for resolution;
// the source title is authoritative) — the leading `!` distinguishes a
// transclusion embed from a plain `[[...]]` wikilink.
export const LOOM_TRANSCLUSION_REGEX = /!\[\[([^\]|]+)(?:\|[^\]]+)?\]\]/;
export const LOOM_TRANSCLUSION_REGEX_GLOBAL = /!\[\[([^\]|]+)(?:\|[^\]]+)?\]\]/g;

function readRefValue(element: HTMLElement): string {
  return (
    element.getAttribute("data-ref-value") ??
    element.getAttribute("data-hs-ref-value") ??
    ""
  );
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    loomTransclusion: {
      /** Inserts a transclusion (read-through) node referencing a source block. */
      insertLoomTransclusion: (attrs: LoomTransclusionAttributes) => ReturnType;
    };
  }
}

/**
 * The loomTransclusion node. `atom: true` (the source content lives elsewhere;
 * the node is an indivisible reference) and a block-group node so it can stand
 * on its own line like an embedded note.
 */
export const LoomTransclusionNode = Node.create<LoomTransclusionNodeOptions>({
  name: "loomTransclusion",
  group: "block",
  atom: true,
  selectable: true,
  draggable: true,

  addOptions() {
    return {
      // Workspace/transport context for resolving the source document.
      // Null = no workspace bound; the NodeView renders a typed fail-closed
      // "no workspace" state (never blank, never copied content).
      embedContext: null,
    };
  },

  addAttributes() {
    return {
      refValue: {
        default: "",
        parseHTML: (element) => readRefValue(element),
        renderHTML: (attributes) => ({ "data-ref-value": String(attributes.refValue) }),
      },
    };
  },

  parseHTML() {
    return [
      { tag: "div[data-testid='loom-transclusion']" },
      { tag: "div[data-hs-node='loomTransclusion']" },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    // Content-free placeholder: the persisted/exported HTML carries ONLY the
    // reference, never the source body. This is the NO-COPY serialization shape.
    return [
      "div",
      mergeAttributes(HTMLAttributes, {
        "data-testid": "loom-transclusion",
        "data-hs-node": "loomTransclusion",
        class: "loom-transclusion",
      }),
    ];
  },

  renderText({ node }) {
    return `![[${node.attrs.refValue}]]`;
  },

  addNodeView() {
    return ReactNodeViewRenderer(LoomTransclusionView);
  },

  addCommands() {
    return {
      insertLoomTransclusion:
        (attrs: LoomTransclusionAttributes) =>
        ({ commands }) =>
          commands.insertContent({ type: this.name, attrs }),
    };
  },

  addInputRules() {
    return [
      nodeInputRule({
        find: new RegExp(`${LOOM_TRANSCLUSION_REGEX.source}$`),
        type: this.type,
        getAttributes: (match) => ({ refValue: (match[1] ?? "").trim() }),
      }),
    ];
  },

  addPasteRules() {
    return [
      nodePasteRule({
        find: LOOM_TRANSCLUSION_REGEX_GLOBAL,
        type: this.type,
        getAttributes: (match) => ({ refValue: (match[1] ?? "").trim() }),
      }),
    ];
  },
});

export type { EmbedResolverContext };
