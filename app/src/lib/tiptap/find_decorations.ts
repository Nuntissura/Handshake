// WP-KERNEL-009 / MT-244 — find-match highlight decorations.
//
// A Tiptap extension owning the ProseMirror decoration layer of the document-
// wide find/replace (MT-244 deliverable 3). The FindReplacePanel computes the
// matches (lib/editor/find_replace.ts) and publishes them into this plugin via
// a transaction meta payload; the plugin renders:
//   - an inline decoration per PROSE match (class hs-find-match, the current
//     match additionally hs-find-match--current + data-find-current for the
//     visual lane to assert),
//   - a node decoration per Monaco CODE BLOCK that contains matches (class
//     hs-find-match-block + data-find-code-matches count attribute) — the
//     in-block highlight itself is owned by the Monaco instance through the
//     code_block_find_registry (a ProseMirror inline decoration cannot reach
//     inside an atom node's Monaco model).
//
// Decorations map across unrelated document edits (DecorationSet.map) so stale
// highlights never point at the wrong text between recomputes; the panel
// recomputes + republishes on every document change while open.
//
// @tiptap/pm is the lockfile-governed ProseMirror surface re-exported by
// Tiptap (same version as @tiptap/core's own internals) — bundled, no CDN.

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { Node as PMNode } from "@tiptap/pm/model";

/** One prose highlight range (absolute ProseMirror positions). */
export interface ProseHighlight {
  from: number;
  to: number;
  current: boolean;
}

/** One code-block highlight marker (node position + match count). */
export interface CodeBlockHighlight {
  pos: number;
  count: number;
  current: boolean;
}

export interface FindHighlightPayload {
  prose: ProseHighlight[];
  codeBlocks: CodeBlockHighlight[];
}

/** Meta key the panel dispatches highlight payloads under. */
export const FIND_DECORATIONS_META = "hsFindDecorations";

export const findDecorationsPluginKey = new PluginKey<DecorationSet>("hsFindDecorations");

/** Clears every find highlight (panel close / query cleared). */
export const EMPTY_FIND_HIGHLIGHTS: FindHighlightPayload = { prose: [], codeBlocks: [] };

function buildDecorationSet(doc: PMNode, payload: FindHighlightPayload): DecorationSet {
  const decorations: Decoration[] = [];
  for (const range of payload.prose) {
    decorations.push(
      Decoration.inline(range.from, range.to, {
        class: range.current ? "hs-find-match hs-find-match--current" : "hs-find-match",
        ...(range.current ? { "data-find-current": "true" } : {}),
      }),
    );
  }
  for (const block of payload.codeBlocks) {
    const node = doc.nodeAt(block.pos);
    if (!node) continue;
    decorations.push(
      Decoration.node(block.pos, block.pos + node.nodeSize, {
        class: block.current ? "hs-find-match-block hs-find-match-block--current" : "hs-find-match-block",
        "data-find-code-matches": String(block.count),
      }),
    );
  }
  return DecorationSet.create(doc, decorations);
}

export const FindDecorations = Extension.create({
  name: "findDecorations",

  addProseMirrorPlugins() {
    return [
      new Plugin<DecorationSet>({
        key: findDecorationsPluginKey,
        state: {
          init: () => DecorationSet.empty,
          apply: (tr, value) => {
            const payload = tr.getMeta(FIND_DECORATIONS_META) as FindHighlightPayload | undefined;
            if (payload) return buildDecorationSet(tr.doc, payload);
            if (tr.docChanged) return value.map(tr.mapping, tr.doc);
            return value;
          },
        },
        props: {
          decorations(state) {
            return findDecorationsPluginKey.getState(state) ?? DecorationSet.empty;
          },
        },
      }),
    ];
  },
});
