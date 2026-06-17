// WP-KERNEL-009 / MT-251 — prose multi-range selection fallback.
//
// ProseMirror owns a single native selection. This plugin provides the honest
// MT-251 fallback: extra prose ranges are stored in plugin state, rendered as
// decorations, and edited together in one transaction when text is typed.

import { Extension, type Editor } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import type { Mapping } from "@tiptap/pm/transform";
import { Decoration, DecorationSet, type EditorView } from "@tiptap/pm/view";

export interface ProseMultiRange {
  from: number;
  to: number;
}

export interface ProseMultiRangeState {
  ranges: ProseMultiRange[];
}

type ProseMultiRangeMeta =
  | { type: "set"; ranges: ProseMultiRange[] }
  | { type: "clear" };

export const PROSE_MULTI_RANGE_META = "hsProseMultiRangeSelection";
export const proseMultiRangePluginKey = new PluginKey<ProseMultiRangeState>(
  "hsProseMultiRangeSelection",
);

const EMPTY_STATE: ProseMultiRangeState = { ranges: [] };

function orderedRange(range: ProseMultiRange): ProseMultiRange {
  return range.from <= range.to
    ? { from: range.from, to: range.to }
    : { from: range.to, to: range.from };
}

function rangeContainsOnlyProseText(doc: PMNode, range: ProseMultiRange): boolean {
  let hasText = false;
  let blocked = false;
  doc.nodesBetween(range.from, range.to, (node) => {
    if (!node.isText && (node.type.name === "monacoCodeBlock" || node.isAtom)) {
      blocked = true;
      return false;
    }
    if (node.isText) hasText = true;
    return true;
  });
  return hasText && !blocked;
}

function normalizeRanges(
  doc: PMNode,
  ranges: readonly ProseMultiRange[],
  options: { allowCollapsed: boolean },
): ProseMultiRange[] {
  const docEnd = doc.content.size;
  const normalized: ProseMultiRange[] = [];
  for (const raw of ranges) {
    const ordered = orderedRange(raw);
    const from = Math.max(0, Math.min(docEnd, ordered.from));
    const to = Math.max(0, Math.min(docEnd, ordered.to));
    if (!options.allowCollapsed && from === to) continue;
    if (from !== to && !rangeContainsOnlyProseText(doc, { from, to })) continue;
    normalized.push({ from, to });
  }
  normalized.sort((a, b) => a.from - b.from || a.to - b.to);

  const nonOverlapping: ProseMultiRange[] = [];
  for (const range of normalized) {
    const previous = nonOverlapping[nonOverlapping.length - 1];
    if (previous && range.from < previous.to) continue;
    if (previous && range.from === previous.from && range.to === previous.to) continue;
    nonOverlapping.push(range);
  }
  return nonOverlapping;
}

function mapRanges(
  doc: PMNode,
  ranges: readonly ProseMultiRange[],
  trMapping: Mapping,
): ProseMultiRange[] {
  return normalizeRanges(
    doc,
    ranges.map((range) => ({
      from: trMapping.map(range.from, -1),
      to: trMapping.map(range.to, 1),
    })),
    { allowCollapsed: true },
  );
}

function decorationSet(doc: PMNode, ranges: readonly ProseMultiRange[]): DecorationSet {
  const decorations = ranges
    .filter((range) => range.from !== range.to)
    .map((range, index) =>
      Decoration.inline(range.from, range.to, {
        class: "hs-multi-range-selection",
        "data-hs-multi-range-index": String(index),
      }),
    );
  return DecorationSet.create(doc, decorations);
}

function getPluginStateFromView(view: EditorView): ProseMultiRangeState {
  return proseMultiRangePluginKey.getState(view.state) ?? EMPTY_STATE;
}

function dispatchSetRanges(view: EditorView, ranges: ProseMultiRange[]): void {
  const normalized = normalizeRanges(view.state.doc, ranges, { allowCollapsed: true });
  view.dispatch(
    view.state.tr.setMeta(PROSE_MULTI_RANGE_META, {
      type: "set",
      ranges: normalized,
    } satisfies ProseMultiRangeMeta),
  );
}

function nextCollapsedRanges(
  ranges: readonly ProseMultiRange[],
  replacementLength: number,
): ProseMultiRange[] {
  let deltaBefore = 0;
  return ranges.map((range) => {
    const caret = range.from + deltaBefore + replacementLength;
    deltaBefore += replacementLength - (range.to - range.from);
    return { from: caret, to: caret };
  });
}

function applyProseMultiRangeTextToView(view: EditorView, text: string): boolean {
  if (text.length === 0) return false;
  const ranges = normalizeRanges(view.state.doc, getPluginStateFromView(view).ranges, {
    allowCollapsed: true,
  });
  if (ranges.length === 0) return false;

  const tr = view.state.tr;
  for (const range of [...ranges].sort((a, b) => b.from - a.from || b.to - a.to)) {
    tr.insertText(text, range.from, range.to);
  }

  const nextRanges = nextCollapsedRanges(ranges, text.length);
  const last = nextRanges[nextRanges.length - 1];
  tr.setMeta(PROSE_MULTI_RANGE_META, {
    type: "set",
    ranges: nextRanges,
  } satisfies ProseMultiRangeMeta);
  tr.setSelection(TextSelection.create(tr.doc, last.from, last.to));
  tr.scrollIntoView();
  view.dispatch(tr);
  return true;
}

export function getProseMultiRangeState(editor: Editor): ProseMultiRangeState {
  const state = proseMultiRangePluginKey.getState(editor.state);
  return state ? { ranges: state.ranges.map((range) => ({ ...range })) } : EMPTY_STATE;
}

export function canAddCurrentProseMultiRange(editor: Editor): boolean {
  const { selection } = editor.state;
  if (selection.empty) return false;
  const current = { from: selection.from, to: selection.to };
  const normalized = normalizeRanges(editor.state.doc, [current], { allowCollapsed: false });
  if (normalized.length !== 1) return false;
  const existing = getProseMultiRangeState(editor).ranges;
  return normalizeRanges(editor.state.doc, [...existing, normalized[0]], {
    allowCollapsed: false,
  }).length === existing.length + 1;
}

export function addCurrentProseMultiRange(editor: Editor): boolean {
  if (!canAddCurrentProseMultiRange(editor)) return false;
  return addProseMultiRange(editor, { from: editor.state.selection.from, to: editor.state.selection.to });
}

export function addProseMultiRange(editor: Editor, range: ProseMultiRange): boolean {
  const current = getProseMultiRangeState(editor).ranges;
  const normalized = normalizeRanges(editor.state.doc, [range], { allowCollapsed: false });
  if (normalized.length !== 1) return false;
  const next = normalizeRanges(editor.state.doc, [...current, normalized[0]], {
    allowCollapsed: false,
  });
  if (next.length !== current.length + 1) return false;
  dispatchSetRanges(editor.view, next);
  return true;
}

export function clearProseMultiRanges(editor: Editor): boolean {
  if (getProseMultiRangeState(editor).ranges.length === 0) return false;
  editor.view.dispatch(
    editor.state.tr.setMeta(PROSE_MULTI_RANGE_META, { type: "clear" } satisfies ProseMultiRangeMeta),
  );
  return true;
}

export function applyProseMultiRangeText(editor: Editor, text: string): boolean {
  return applyProseMultiRangeTextToView(editor.view, text);
}

export const ProseMultiRangeSelection = Extension.create({
  name: "proseMultiRangeSelection",

  addProseMirrorPlugins() {
    return [
      new Plugin<ProseMultiRangeState>({
        key: proseMultiRangePluginKey,
        state: {
          init: () => EMPTY_STATE,
          apply: (tr, value) => {
            const meta = tr.getMeta(PROSE_MULTI_RANGE_META) as ProseMultiRangeMeta | undefined;
            if (meta?.type === "clear") return EMPTY_STATE;
            if (meta?.type === "set") {
              return {
                ranges: normalizeRanges(tr.doc, meta.ranges, { allowCollapsed: true }),
              };
            }
            if (tr.docChanged && value.ranges.length > 0) {
              return { ranges: mapRanges(tr.doc, value.ranges, tr.mapping) };
            }
            return value;
          },
        },
        props: {
          decorations(state) {
            return decorationSet(state.doc, proseMultiRangePluginKey.getState(state)?.ranges ?? []);
          },
          handleTextInput(view, _from, _to, text) {
            return applyProseMultiRangeTextToView(view, text);
          },
        },
      }),
    ];
  },
});
