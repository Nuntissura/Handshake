// WP-KERNEL-009 / MT-163 — TiptapCustomLinkNodes: the hsLink inline node.
//
// A typed [[kind:value]] wikilink as a real Tiptap/ProseMirror inline atom node
// (named `hsLink`, matching the MT-161 inventory). Typing `[[wp:WP-KERNEL-009]]`
// (or pasting text containing wikilinks) converts the token into a typed link
// node carrying { refKind, refValue, label } — refKind is the backend ref kind
// (RichDocBacklink.link_kind / RichDocEmbed.ref_kind in app/src/lib/api.ts), so
// the saved document persists a typed `link` block the backend can resolve.
//
// The node round-trips: parseHTML reads the attributes back from
// data-ref-kind/data-ref-value/data-label and renderHTML re-emits a span with
// stable selectors (data-testid="hs-link", data-ref-kind, data-ref-value) for
// MT-172 visual debugging. An unresolved prefix is preserved as
// data-ref-kind="unknown" (never dropped) and styled distinctly so a broken
// link is visible, not silent (MT-174 spirit).
//
// MT-244 adds the NodeView (HsLinkView): media kinds (images/video/album/
// slideshow) render REAL backend assets fail-closed; all other kinds keep the
// MT-163 chip rendering. The embed workspace context flows through the
// extension options (embedContext) so the persisted document is unchanged.

import { Node, mergeAttributes, nodeInputRule, nodePasteRule } from "@tiptap/core";
import { ReactNodeViewRenderer } from "@tiptap/react";
import { WIKILINK_REGEX, WIKILINK_REGEX_GLOBAL, classifyWikilink } from "../editor/wikilink";
import { HsLinkView, type HsLinkNodeOptions } from "../../components/HsLinkView";
import type { EmbedResolverContext } from "../editor/embed_assets";

export interface HsLinkAttributes {
  refKind: string;
  refValue: string;
  label: string;
  resolved: boolean;
}

export type { EmbedResolverContext };

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    hsLink: {
      /** Inserts a typed wikilink node at the current selection. */
      insertHsLink: (attrs: HsLinkAttributes) => ReturnType;
    };
  }
}

/**
 * The hsLink inline atom node. `atom: true` + `inline: true` means the node is a
 * single indivisible inline token (you cannot place the caret inside it), which
 * is the correct model for a resolved link chip.
 */
export const HsLinkNode = Node.create<HsLinkNodeOptions>({
  name: "hsLink",
  group: "inline",
  inline: true,
  atom: true,
  selectable: true,

  addOptions() {
    return {
      // Workspace/transport context for media embed resolution (MT-244).
      // Null = no workspace bound; media embeds render a typed no_workspace
      // error (fail-closed) while non-media chips render unchanged.
      embedContext: null,
    };
  },

  addAttributes() {
    return {
      refKind: {
        default: "unknown",
        parseHTML: (element) => element.getAttribute("data-ref-kind") ?? "unknown",
        renderHTML: (attributes) => ({ "data-ref-kind": String(attributes.refKind) }),
      },
      refValue: {
        default: "",
        parseHTML: (element) => element.getAttribute("data-ref-value") ?? "",
        renderHTML: (attributes) => ({ "data-ref-value": String(attributes.refValue) }),
      },
      label: {
        default: "",
        parseHTML: (element) => element.getAttribute("data-label") ?? element.textContent ?? "",
        renderHTML: (attributes) => ({ "data-label": String(attributes.label) }),
      },
      resolved: {
        default: true,
        parseHTML: (element) => element.getAttribute("data-resolved") !== "false",
        renderHTML: (attributes) => ({ "data-resolved": attributes.resolved ? "true" : "false" }),
      },
    };
  },

  parseHTML() {
    return [{ tag: "span[data-testid='hs-link']" }];
  },

  renderHTML({ node, HTMLAttributes }) {
    const label = String(node.attrs.label || `${node.attrs.refKind}:${node.attrs.refValue}`);
    const resolved = node.attrs.resolved !== false;
    return [
      "span",
      mergeAttributes(HTMLAttributes, {
        "data-testid": "hs-link",
        class: resolved ? "hs-link hs-link--resolved" : "hs-link hs-link--unresolved",
        title: `${node.attrs.refKind}:${node.attrs.refValue}`,
      }),
      label,
    ];
  },

  renderText({ node }) {
    // Serializing back to plain text reproduces the wikilink token so a
    // markdown/plain-text projection round-trips.
    const label = String(node.attrs.label || "");
    const base = `[[${node.attrs.refKind}:${node.attrs.refValue}`;
    return label && label !== node.attrs.refValue ? `${base}|${label}]]` : `${base}]]`;
  },

  addNodeView() {
    // MT-244: media kinds (images/video/album/slideshow) render real embeds;
    // every other kind renders the exact MT-163 chip. renderHTML below stays
    // the serialization shape (getHTML/exports/copy-paste round-trip).
    return ReactNodeViewRenderer(HsLinkView);
  },

  addCommands() {
    return {
      insertHsLink:
        (attrs: HsLinkAttributes) =>
        ({ commands }) =>
          commands.insertContent({ type: this.name, attrs }),
    };
  },

  addInputRules() {
    return [
      nodeInputRule({
        find: new RegExp(`${WIKILINK_REGEX.source}$`),
        type: this.type,
        getAttributes: (match) => {
          const parsed = classifyWikilink(match[1], match[2], match[3]);
          return {
            refKind: parsed.refKind,
            refValue: parsed.refValue,
            label: parsed.label,
            resolved: parsed.resolved,
          };
        },
      }),
    ];
  },

  addPasteRules() {
    return [
      nodePasteRule({
        find: WIKILINK_REGEX_GLOBAL,
        type: this.type,
        getAttributes: (match) => {
          const parsed = classifyWikilink(match[1], match[2], match[3]);
          return {
            refKind: parsed.refKind,
            refValue: parsed.refValue,
            label: parsed.label,
            resolved: parsed.resolved,
          };
        },
      }),
    ];
  },
});
