// WP-KERNEL-009 / MT-253 — unified-patch -> diff-side reconstruction.
//
// The source-control backend returns a git unified diff (`git diff --no-color`).
// To render it in a REAL Monaco diff editor (reusing the MT-247 diff seam) we
// reconstruct the two file sides from the patch hunks: removed/context lines
// rebuild the "original" side, added/context lines rebuild the "modified" side.
//
// This is a faithful projection of the hunks the backend already produced; it
// never invents content. Lines outside hunks are not reconstructed (a unified
// diff only carries the changed regions plus their context windows), which is
// exactly what a per-file change review needs.

export interface UnifiedPatchSides {
  /** Reconstructed pre-change content (context + removed lines). */
  original: string;
  /** Reconstructed post-change content (context + added lines). */
  modified: string;
  /** Number of hunk headers (`@@ ... @@`) parsed. */
  hunkCount: number;
  /** Added line count (lines beginning with a single `+`, excluding the +++ header). */
  addedLines: number;
  /** Removed line count (lines beginning with a single `-`, excluding the --- header). */
  removedLines: number;
}

const HUNK_HEADER = /^@@ /;
const FILE_HEADER_OLD = /^--- /;
const FILE_HEADER_NEW = /^\+\+\+ /;

/**
 * Parses a git unified diff into reconstructed original/modified sides.
 *
 * Robust to: CRLF, `\ No newline at end of file` markers, file/hunk headers,
 * `diff --git`/`index`/mode metadata lines, and empty patches. Only the hunk
 * body lines contribute to the reconstructed sides.
 */
export function parseUnifiedPatchSides(patch: string): UnifiedPatchSides {
  const original: string[] = [];
  const modified: string[] = [];
  let hunkCount = 0;
  let addedLines = 0;
  let removedLines = 0;
  let inHunk = false;

  const lines = patch.split(/\r?\n/);
  for (const line of lines) {
    if (HUNK_HEADER.test(line)) {
      inHunk = true;
      hunkCount += 1;
      continue;
    }
    if (!inHunk) {
      // diff --git / index / old mode / file headers — skip until first hunk.
      continue;
    }
    if (FILE_HEADER_OLD.test(line) || FILE_HEADER_NEW.test(line)) {
      // Defensive: file headers should precede the first hunk, but skip if seen.
      continue;
    }
    if (line.startsWith("\\")) {
      // "\ No newline at end of file" — not a content line.
      continue;
    }
    if (line.startsWith("+")) {
      modified.push(line.slice(1));
      addedLines += 1;
      continue;
    }
    if (line.startsWith("-")) {
      original.push(line.slice(1));
      removedLines += 1;
      continue;
    }
    // Context line (leading space) or a bare empty trailing line.
    const content = line.startsWith(" ") ? line.slice(1) : line;
    original.push(content);
    modified.push(content);
  }

  return {
    original: original.join("\n"),
    modified: modified.join("\n"),
    hunkCount,
    addedLines,
    removedLines,
  };
}
