// WP-KERNEL-009 / MT-164 — auto-code-block detection (pure logic).
//
// The rules that turn prose into an embedded Monaco code block (MT-165):
//   - a fenced opener ```lang at the start of a line,
//   - a pasted blob containing one or more ```lang … ``` fenced regions,
//   - a pasted indented (4-space / tab) code region,
//   - a language hint resolved to a canonical id (MT-166).
// Reversible: a code block serializes back to a fenced block in plain text
// (monacoCodeBlock.renderText), so prose↔code conversion round-trips.
//
// Split out of the Tiptap extension so the detection is unit-testable in jsdom
// without the editor runtime. The extension (auto_code_block_rules.ts) wires
// these into input rules / paste rules that construct monacoCodeBlock nodes.

import { languageFromFenceInfo, normalizeLanguageHint } from "../monaco/language_registry";

/** A detected fenced code region inside a larger text blob. */
export interface DetectedFencedBlock {
  language: string;
  code: string;
}

/** Matches the opening fence of a code block at the start of a line: ```lang */
export const FENCE_OPEN_REGEX = /^```([a-zA-Z0-9_+-]*)\s$/;

/** Global fenced-region matcher: ```lang\n<code>\n``` (non-greedy body). */
export const FENCED_REGION_REGEX = /```([a-zA-Z0-9_+-]*)[ \t]*\n([\s\S]*?)\n?```/g;

/**
 * Detects whether a single-line input is a fenced-code opener (the trigger for
 * the type-as-you-go input rule). Returns the normalized language id, or null
 * when the line is not a fence opener.
 */
export function detectFenceOpener(line: string): string | null {
  const match = FENCE_OPEN_REGEX.exec(line);
  if (!match) return null;
  return languageFromFenceInfo(match[1]);
}

/**
 * Extracts every fenced code region from a pasted text blob, in order. Used by
 * the paste rule to convert pasted markdown-style fenced blocks into code
 * blocks. Returns an empty array when no fenced region is present.
 */
export function detectFencedBlocks(text: string): DetectedFencedBlock[] {
  const blocks: DetectedFencedBlock[] = [];
  for (const match of text.matchAll(FENCED_REGION_REGEX)) {
    blocks.push({
      language: normalizeLanguageHint(match[1]),
      code: match[2] ?? "",
    });
  }
  return blocks;
}

/**
 * Decides whether a pasted multi-line blob looks like an INDENTED code block
 * (every non-empty line begins with 4 spaces or a tab, and there are at least
 * `minLines` such lines). Returns the de-indented code when it qualifies, else
 * null. Language is unknown for indented blocks → caller uses plaintext.
 */
export function detectIndentedCodeBlock(
  text: string,
  minLines = 2,
): string | null {
  const lines = text.split("\n");
  const nonEmpty = lines.filter((l) => l.trim().length > 0);
  if (nonEmpty.length < minLines) return null;
  const allIndented = nonEmpty.every((l) => /^( {4}|\t)/.test(l));
  if (!allIndented) return null;
  const deindented = lines
    .map((l) => l.replace(/^( {4}|\t)/, ""))
    .join("\n")
    .replace(/\n+$/, "");
  return deindented;
}

/**
 * Reverses a code block to a fenced markdown string (the inverse of detection),
 * used by prose↔code reversibility. Mirrors monacoCodeBlock.renderText so the
 * two stay consistent.
 */
export function codeBlockToFenced(language: string, code: string): string {
  return "```" + (language || "") + "\n" + code + "\n```";
}

/** One ordered segment of a pasted blob (iteration-3 H6 paste pipeline). */
export interface PasteSegment {
  kind: "prose" | "code";
  /** Canonical language id (code segments only). */
  language?: string;
  text: string;
}

/**
 * Splits a pasted text blob into ORDERED prose/code segments so a paste that
 * mixes prose and ```fenced``` regions loses nothing (iteration-3 H6: the
 * previous helper extracted only the code blocks, silently dropping the prose
 * around them). Returns null when the blob contains no code at all — the
 * caller then lets the default paste run.
 */
export function segmentPasteText(text: string): PasteSegment[] | null {
  const segments: PasteSegment[] = [];
  let cursor = 0;
  let foundFence = false;
  for (const match of text.matchAll(FENCED_REGION_REGEX)) {
    foundFence = true;
    const start = match.index ?? 0;
    if (start > cursor) {
      const prose = text.slice(cursor, start);
      if (prose.trim().length > 0) segments.push({ kind: "prose", text: prose });
    }
    segments.push({
      kind: "code",
      language: normalizeLanguageHint(match[1]),
      text: match[2] ?? "",
    });
    cursor = start + match[0].length;
  }
  if (foundFence) {
    const tail = text.slice(cursor);
    if (tail.trim().length > 0) segments.push({ kind: "prose", text: tail });
    return segments;
  }
  const indented = detectIndentedCodeBlock(text);
  if (indented !== null) {
    return [{ kind: "code", language: normalizeLanguageHint(undefined), text: indented }];
  }
  return null;
}
