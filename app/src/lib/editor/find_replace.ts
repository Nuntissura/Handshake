// WP-KERNEL-009 / MT-244 — document-wide find/replace (prose + code blocks).
//
// VS Code-class find/replace over the WHOLE rich document: prose text nodes
// AND the text inside embedded Monaco code blocks (the code lives in the
// monacoCodeBlock node's `code` attribute — MT-165/168). Pure scan/plan logic
// over a real ProseMirror document plus a single-transaction applier:
//
//   scan      buildFindMatches(doc, query)   → ordered typed matches
//   navigate  selectMatchIndex / nextIndex   → wrap-around navigation
//   replace   applyReplaceAll / applyReplaceMatch → ONE transaction (one undo
//             step: prose insertText edits and code-block setNodeMarkup attr
//             rewrites are applied in DESCENDING position order so earlier
//             positions stay valid without remapping)
//
// Query semantics (VS Code parity):
//   - caseSensitive, wholeWord (word-boundary post-filter), regex mode,
//   - regex replacements expand $1..$9, $& and $$,
//   - invalid regex → typed error result (never a throw into the UI),
//   - zero-length regex matches advance by one (no infinite loop).
//
// DoS guards (documented adversarial decision): the search term is capped at
// MAX_TERM_LENGTH, matches cap at MAX_MATCHES (result carries `truncated`),
// and the segment scan stops past a time budget. RESIDUAL RISK (accepted +
// documented): one catastrophic-backtracking regex exec on ONE huge segment
// cannot be interrupted from JS — the same residual VS Code accepts; the
// budget bounds the damage across segments.

import type { Editor } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import type { Transaction } from "@tiptap/pm/state";
import { TextSelection } from "@tiptap/pm/state";
import { codeBlockHash } from "./code_block_serialization";

export const MONACO_CODE_BLOCK_NODE_NAME = "monacoCodeBlock";
export const MAX_TERM_LENGTH = 1000;
export const MAX_MATCHES = 2000;
export const SCAN_TIME_BUDGET_MS = 250;

export interface FindQuery {
  term: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  isRegex: boolean;
}

export const EMPTY_FIND_QUERY: FindQuery = {
  term: "",
  caseSensitive: false,
  wholeWord: false,
  isRegex: false,
};

/** A match inside a prose text run (absolute ProseMirror positions). */
export interface ProseMatch {
  kind: "prose";
  from: number;
  to: number;
  /** Captured regex groups (regex mode) for replacement expansion. */
  groups: string[];
  matchText: string;
}

/** A match inside an embedded Monaco code block's `code` attribute. */
export interface CodeMatch {
  kind: "code";
  /** Position of the monacoCodeBlock node itself. */
  nodePos: number;
  /** Character offsets within the code attribute text. */
  start: number;
  end: number;
  groups: string[];
  matchText: string;
}

export type FindMatch = ProseMatch | CodeMatch;

export interface FindScanResult {
  matches: FindMatch[];
  /** True when MAX_MATCHES or the time budget cut the scan short. */
  truncated: boolean;
  /** Typed query error (invalid regex / term too long); empty scan when set. */
  error: string | null;
}

export const EMPTY_SCAN: FindScanResult = { matches: [], truncated: false, error: null };

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Compiles the query to a global RegExp, or a typed error. */
export function compileFindQuery(query: FindQuery): { regex: RegExp } | { error: string } {
  const term = query.term;
  if (term.length === 0) return { error: "empty" };
  if (term.length > MAX_TERM_LENGTH) {
    return { error: `search term longer than ${MAX_TERM_LENGTH} characters` };
  }
  const source = query.isRegex ? term : escapeRegExp(term);
  const flags = query.caseSensitive ? "gu" : "giu";
  try {
    return { regex: new RegExp(source, flags) };
  } catch (error) {
    return { error: `invalid regular expression: ${error instanceof Error ? error.message : String(error)}` };
  }
}

const WORD_CHAR = /[\p{L}\p{N}_]/u;

function isWordBoundary(text: string, start: number, end: number): boolean {
  const before = start > 0 ? text[start - 1] : "";
  const after = end < text.length ? text[end] : "";
  const startsOnWord = text.length > start && WORD_CHAR.test(text[start] ?? "");
  const endsOnWord = end > 0 && WORD_CHAR.test(text[end - 1] ?? "");
  if (startsOnWord && before && WORD_CHAR.test(before)) return false;
  if (endsOnWord && after && WORD_CHAR.test(after)) return false;
  return true;
}

interface SegmentMatch {
  start: number;
  end: number;
  groups: string[];
  matchText: string;
}

/** Runs the compiled regex over one text segment with the documented guards. */
function scanSegment(
  text: string,
  regex: RegExp,
  wholeWord: boolean,
  remaining: number,
): { matches: SegmentMatch[]; truncated: boolean } {
  const matches: SegmentMatch[] = [];
  regex.lastIndex = 0;
  let result: RegExpExecArray | null;
  while ((result = regex.exec(text)) !== null) {
    const start = result.index;
    const end = start + result[0].length;
    if (result[0].length === 0) {
      // Zero-length match: advance manually (no infinite loop).
      regex.lastIndex = start + 1;
      if (regex.lastIndex > text.length) break;
      continue;
    }
    if (!wholeWord || isWordBoundary(text, start, end)) {
      matches.push({ start, end, groups: result.slice(1).map((g) => g ?? ""), matchText: result[0] });
      if (matches.length >= remaining) return { matches, truncated: true };
    }
  }
  return { matches, truncated: false };
}

/**
 * Scans the whole document: every prose text run (per parent block, so a
 * match never spans block boundaries — VS Code behavior) and every Monaco
 * code block's code text. Returns matches in document order.
 */
export function buildFindMatches(doc: PMNode, query: FindQuery): FindScanResult {
  const compiled = compileFindQuery(query);
  if ("error" in compiled) {
    return compiled.error === "empty" ? EMPTY_SCAN : { matches: [], truncated: false, error: compiled.error };
  }
  const { regex } = compiled;
  const matches: FindMatch[] = [];
  let truncated = false;
  const startedAt = Date.now();

  doc.descendants((node, pos) => {
    if (truncated || Date.now() - startedAt > SCAN_TIME_BUDGET_MS) {
      truncated = true;
      return false;
    }
    if (node.type.name === MONACO_CODE_BLOCK_NODE_NAME) {
      const code = String(node.attrs.code ?? "");
      const segment = scanSegment(code, regex, query.wholeWord, MAX_MATCHES - matches.length);
      for (const m of segment.matches) {
        matches.push({ kind: "code", nodePos: pos, start: m.start, end: m.end, groups: m.groups, matchText: m.matchText });
      }
      truncated = truncated || segment.truncated;
      return false; // atom: nothing to descend into
    }
    if (node.isText && node.text) {
      const segment = scanSegment(node.text, regex, query.wholeWord, MAX_MATCHES - matches.length);
      for (const m of segment.matches) {
        matches.push({ kind: "prose", from: pos + m.start, to: pos + m.end, groups: m.groups, matchText: m.matchText });
      }
      truncated = truncated || segment.truncated;
    }
    return true;
  });

  return { matches, truncated, error: null };
}

/**
 * Document-order sort key of a match. A monacoCodeBlock is an ATOM (nodeSize
 * 1), so its inner code offsets must map into the open interval
 * (nodePos, nodePos+1) — never past the node — to stay document-ordered
 * against following blocks: nodePos + 1 - 1/(start+2) is strictly increasing
 * in `start` and strictly below nodePos+1.
 */
export function matchOrderKey(match: FindMatch): number {
  return match.kind === "prose" ? match.from : match.nodePos + 1 - 1 / (match.start + 2);
}

/**
 * The index of the first match at-or-after the selection head (wraps to 0).
 * Returns -1 when there are no matches.
 */
export function indexAfterSelection(matches: FindMatch[], selectionHead: number): number {
  if (matches.length === 0) return -1;
  for (let i = 0; i < matches.length; i++) {
    if (matchOrderKey(matches[i]) >= selectionHead) return i;
  }
  return 0;
}

/** Expands $1..$9, $& and $$ in a replacement template (regex mode). */
export function expandReplacement(template: string, match: FindMatch): string {
  return template.replace(/\$(\$|&|[1-9])/g, (_, token: string) => {
    if (token === "$") return "$";
    if (token === "&") return match.matchText;
    const index = Number(token) - 1;
    return match.groups[index] ?? "";
  });
}

/** Replacement text for one match honoring regex group expansion. */
export function replacementFor(match: FindMatch, replacement: string, isRegex: boolean): string {
  return isRegex ? expandReplacement(replacement, match) : replacement;
}

function applyCodeEdits(
  tr: Transaction,
  nodePos: number,
  node: PMNode,
  edits: Array<{ start: number; end: number; text: string }>,
): void {
  const language = String(node.attrs.language ?? "plaintext");
  let code = String(node.attrs.code ?? "");
  // Descending offsets: earlier edit offsets stay valid.
  const ordered = [...edits].sort((a, b) => b.start - a.start);
  for (const edit of ordered) {
    code = code.slice(0, edit.start) + edit.text + code.slice(edit.end);
  }
  tr.setNodeMarkup(nodePos, undefined, {
    ...node.attrs,
    code,
    contentHash: codeBlockHash(language, code),
  });
}

export interface ReplaceOutcome {
  replacedProse: number;
  replacedCode: number;
}

/**
 * Replaces EVERY match in ONE transaction (single undo step). Prose edits and
 * per-node code rewrites are applied in descending document order so no
 * position remapping is needed; the selection is preserved via the
 * transaction mapping (no selection loss on replace-all).
 */
export function applyReplaceAll(
  editor: Editor,
  matches: FindMatch[],
  replacement: string,
  isRegex: boolean,
): ReplaceOutcome {
  const { state } = editor;
  const tr = state.tr;

  const prose = matches.filter((m): m is ProseMatch => m.kind === "prose");
  const code = matches.filter((m): m is CodeMatch => m.kind === "code");

  // Group code edits per node so each node is rewritten exactly once.
  const codeByNode = new Map<number, CodeMatch[]>();
  for (const match of code) {
    const list = codeByNode.get(match.nodePos) ?? [];
    list.push(match);
    codeByNode.set(match.nodePos, list);
  }

  type Op =
    | { pos: number; run: () => void };
  const ops: Op[] = [];

  for (const match of prose) {
    ops.push({
      pos: match.from,
      run: () => tr.insertText(replacementFor(match, replacement, isRegex), match.from, match.to),
    });
  }
  for (const [nodePos, nodeMatches] of codeByNode) {
    ops.push({
      pos: nodePos,
      run: () => {
        const node = state.doc.nodeAt(nodePos);
        if (!node || node.type.name !== MONACO_CODE_BLOCK_NODE_NAME) return;
        applyCodeEdits(
          tr,
          nodePos,
          node,
          nodeMatches.map((m) => ({ start: m.start, end: m.end, text: replacementFor(m, replacement, isRegex) })),
        );
      },
    });
  }

  // Descending document order: later edits first, earlier positions stable.
  ops.sort((a, b) => b.pos - a.pos);
  for (const op of ops) op.run();

  if (tr.docChanged) {
    // Keep the caret where the (mapped) selection head lands — no jump to 0.
    const mappedHead = tr.mapping.map(state.selection.head);
    const clamped = Math.min(mappedHead, tr.doc.content.size);
    tr.setSelection(TextSelection.near(tr.doc.resolve(clamped)));
    editor.view.dispatch(tr);
  }
  return { replacedProse: prose.length, replacedCode: code.length };
}

/** Replaces a single match (one transaction). */
export function applyReplaceMatch(
  editor: Editor,
  match: FindMatch,
  replacement: string,
  isRegex: boolean,
): ReplaceOutcome {
  return applyReplaceAll(editor, [match], replacement, isRegex);
}

/**
 * Moves the editor selection to a match: prose matches get a text selection
 * (scrolled into view); code matches select the code block node so the
 * NodeView can take over the in-block reveal (code_block_find_registry).
 * The scroll is applied separately and guarded: a layout-less environment
 * (jsdom) has no client rects, and a failed scroll must never break find.
 */
export function selectMatch(editor: Editor, match: FindMatch): void {
  if (match.kind === "prose") {
    editor.chain().setTextSelection({ from: match.from, to: match.to }).run();
  } else {
    editor.chain().setNodeSelection(match.nodePos).run();
  }
  try {
    editor.commands.scrollIntoView();
  } catch {
    // No layout engine (headless tests): selection landed; scroll is cosmetic.
  }
}
