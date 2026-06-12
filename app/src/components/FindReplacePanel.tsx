// WP-KERNEL-009 / MT-244 — document-wide find/replace panel.
//
// VS Code-class find/replace over prose AND embedded Monaco code blocks:
//   - query options: case-sensitive (Aa), whole-word (W), regex (.*) toggles
//     with aria-pressed state,
//   - live match count ("3 of 17", + truncation marker on capped scans),
//   - find next/prev with wrap-around; the current match is highlighted via
//     the FindDecorations extension and scrolled into view; a code-block
//     match additionally reveals + selects the range inside the real Monaco
//     instance (code_block_find_registry),
//   - replace one / replace all (code-block text included) in a SINGLE
//     transaction → one undo step, selection preserved (find_replace.ts),
//   - typed inline error for an invalid regex (role=alert, never a throw),
//   - Enter = next, Shift+Enter = previous, Escape closes and returns focus
//     to the editor.
//
// The panel recomputes matches on every document change while open (the
// decorations extension maps stale highlights between recomputes) and clears
// every highlight when closed. Stable selectors: find-panel, find-input,
// find-count, find-next, find-prev, find-toggle-case, find-toggle-word,
// find-toggle-regex, find-error, replace-input, replace-one, replace-all,
// find-close (visual-debug suite, MT-172 spirit).

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import type { Editor } from "@tiptap/core";
import {
  buildFindMatches,
  indexAfterSelection,
  applyReplaceAll,
  applyReplaceMatch,
  selectMatch,
  EMPTY_SCAN,
  type FindQuery,
  type FindScanResult,
} from "../lib/editor/find_replace";
import { revealCodeBlockRange } from "../lib/editor/code_block_find_registry";
import {
  FIND_DECORATIONS_META,
  EMPTY_FIND_HIGHLIGHTS,
  type FindHighlightPayload,
} from "../lib/tiptap/find_decorations";

export interface FindReplacePanelProps {
  editor: Editor;
  /** Show the replace row (Mod-h) or find-only (Mod-f). */
  withReplace: boolean;
  onClose: () => void;
}

/** Publishes highlight decorations for the current scan into the plugin. */
function publishHighlights(editor: Editor, scan: FindScanResult, activeIndex: number): void {
  if (editor.isDestroyed) return;
  const payload: FindHighlightPayload = { prose: [], codeBlocks: [] };
  const codeCounts = new Map<number, { count: number; current: boolean }>();
  scan.matches.forEach((match, index) => {
    if (match.kind === "prose") {
      payload.prose.push({ from: match.from, to: match.to, current: index === activeIndex });
    } else {
      const entry = codeCounts.get(match.nodePos) ?? { count: 0, current: false };
      entry.count += 1;
      entry.current = entry.current || index === activeIndex;
      codeCounts.set(match.nodePos, entry);
    }
  });
  for (const [pos, entry] of codeCounts) {
    payload.codeBlocks.push({ pos, count: entry.count, current: entry.current });
  }
  editor.view.dispatch(editor.state.tr.setMeta(FIND_DECORATIONS_META, payload));
}

function clearHighlights(editor: Editor): void {
  if (editor.isDestroyed) return;
  editor.view.dispatch(editor.state.tr.setMeta(FIND_DECORATIONS_META, EMPTY_FIND_HIGHLIGHTS));
}

export function FindReplacePanel({ editor, withReplace, onClose }: FindReplacePanelProps) {
  const [term, setTerm] = useState("");
  const [replacement, setReplacement] = useState("");
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [isRegex, setIsRegex] = useState(false);
  const [activeIndex, setActiveIndex] = useState(-1);
  const [docVersion, setDocVersion] = useState(0);
  const [lastOutcome, setLastOutcome] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  const query: FindQuery = useMemo(
    () => ({ term, caseSensitive, wholeWord, isRegex }),
    [term, caseSensitive, wholeWord, isRegex],
  );

  // Recompute matches when the query or the document changes while open.
  const scan: FindScanResult = useMemo(() => {
    void docVersion; // doc edits invalidate this memo
    if (query.term.length === 0) return EMPTY_SCAN;
    return buildFindMatches(editor.state.doc, query);
  }, [editor, query, docVersion]);

  // Track document changes (replace/typing) → rescan.
  useEffect(() => {
    const onUpdate = () => setDocVersion((value) => value + 1);
    editor.on("update", onUpdate);
    return () => {
      editor.off("update", onUpdate);
    };
  }, [editor]);

  // Clamp the active index when the match set shrinks.
  const safeIndex = scan.matches.length === 0 ? -1 : Math.min(Math.max(activeIndex, 0), scan.matches.length - 1);

  // Publish highlight decorations on every scan/index change; clear on close.
  useEffect(() => {
    publishHighlights(editor, scan, safeIndex);
  }, [editor, scan, safeIndex]);
  useEffect(() => () => clearHighlights(editor), [editor]);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  const goTo = useCallback(
    (index: number) => {
      if (scan.matches.length === 0) return;
      const wrapped = ((index % scan.matches.length) + scan.matches.length) % scan.matches.length;
      setActiveIndex(wrapped);
      const match = scan.matches[wrapped];
      selectMatch(editor, match);
      if (match.kind === "code") {
        revealCodeBlockRange(match.nodePos, match.start, match.end);
      }
    },
    [editor, scan.matches],
  );

  const findNext = useCallback(() => {
    if (scan.matches.length === 0) return;
    if (safeIndex === -1) {
      goTo(indexAfterSelection(scan.matches, editor.state.selection.head));
      return;
    }
    goTo(safeIndex + 1);
  }, [editor, goTo, safeIndex, scan.matches]);

  const findPrevious = useCallback(() => {
    if (scan.matches.length === 0) return;
    goTo((safeIndex === -1 ? 0 : safeIndex) - 1);
  }, [goTo, safeIndex, scan.matches.length]);

  const replaceCurrent = useCallback(() => {
    if (scan.matches.length === 0) return;
    const index = safeIndex === -1 ? 0 : safeIndex;
    const outcome = applyReplaceMatch(editor, scan.matches[index], replacement, isRegex);
    setLastOutcome(`Replaced 1 match (${outcome.replacedCode > 0 ? "code" : "prose"}).`);
    // The doc-update listener rescans; keep the index so "next" continues.
    setActiveIndex(index);
  }, [editor, isRegex, replacement, safeIndex, scan.matches]);

  const replaceEverything = useCallback(() => {
    if (scan.matches.length === 0) return;
    const outcome = applyReplaceAll(editor, scan.matches, replacement, isRegex);
    setLastOutcome(
      `Replaced ${outcome.replacedProse + outcome.replacedCode} matches ` +
        `(${outcome.replacedProse} prose, ${outcome.replacedCode} in code blocks).`,
    );
    setActiveIndex(-1);
  }, [editor, isRegex, replacement, scan.matches]);

  const close = useCallback(() => {
    clearHighlights(editor);
    onClose();
    editor.commands.focus();
  }, [editor, onClose]);

  const onInputKeyDown = (event: ReactKeyboardEvent) => {
    if (event.key === "Enter") {
      event.preventDefault();
      if (event.shiftKey) findPrevious();
      else findNext();
    }
    if (event.key === "Escape") {
      event.preventDefault();
      close();
    }
  };

  const countLabel =
    scan.matches.length === 0
      ? term.length === 0
        ? ""
        : "No matches"
      : `${(safeIndex === -1 ? 0 : safeIndex) + 1} of ${scan.matches.length}${scan.truncated ? "+" : ""}`;

  return (
    <div
      className="find-replace-panel"
      role="search"
      aria-label={withReplace ? "Find and replace" : "Find in document"}
      data-testid="find-panel"
      data-with-replace={withReplace ? "true" : "false"}
      data-match-count={String(scan.matches.length)}
      data-active-index={String(safeIndex)}
      data-truncated={scan.truncated ? "true" : "false"}
    >
      <div className="find-replace-panel__row">
        <input
          ref={inputRef}
          type="text"
          className="find-replace-panel__input"
          data-testid="find-input"
          aria-label="Find"
          placeholder="Find (prose + code blocks)…"
          value={term}
          onChange={(event) => {
            setTerm(event.target.value);
            setActiveIndex(-1);
            setLastOutcome(null);
          }}
          onKeyDown={onInputKeyDown}
        />
        <button
          type="button"
          className={caseSensitive ? "tt-button tt-button--active" : "tt-button"}
          data-testid="find-toggle-case"
          aria-pressed={caseSensitive}
          aria-label="Match case"
          title="Match case"
          onClick={() => setCaseSensitive((value) => !value)}
        >
          Aa
        </button>
        <button
          type="button"
          className={wholeWord ? "tt-button tt-button--active" : "tt-button"}
          data-testid="find-toggle-word"
          aria-pressed={wholeWord}
          aria-label="Match whole word"
          title="Match whole word"
          onClick={() => setWholeWord((value) => !value)}
        >
          W
        </button>
        <button
          type="button"
          className={isRegex ? "tt-button tt-button--active" : "tt-button"}
          data-testid="find-toggle-regex"
          aria-pressed={isRegex}
          aria-label="Use regular expression"
          title="Use regular expression"
          onClick={() => setIsRegex((value) => !value)}
        >
          .*
        </button>
        <span className="find-replace-panel__count muted" data-testid="find-count" aria-live="polite">
          {countLabel}
        </span>
        <button type="button" className="tt-button" data-testid="find-prev" aria-label="Previous match" onClick={findPrevious}>
          ↑
        </button>
        <button type="button" className="tt-button" data-testid="find-next" aria-label="Next match" onClick={findNext}>
          ↓
        </button>
        <button type="button" className="tt-button" data-testid="find-close" aria-label="Close find" onClick={close}>
          ✕
        </button>
      </div>

      {withReplace && (
        <div className="find-replace-panel__row">
          <input
            type="text"
            className="find-replace-panel__input"
            data-testid="replace-input"
            aria-label="Replace with"
            placeholder={isRegex ? "Replace ($1…$9, $& supported)…" : "Replace…"}
            value={replacement}
            onChange={(event) => setReplacement(event.target.value)}
            onKeyDown={onInputKeyDown}
          />
          <button
            type="button"
            className="tt-button"
            data-testid="replace-one"
            aria-label="Replace current match"
            disabled={scan.matches.length === 0}
            onClick={replaceCurrent}
          >
            Replace
          </button>
          <button
            type="button"
            className="tt-button"
            data-testid="replace-all"
            aria-label="Replace all matches"
            disabled={scan.matches.length === 0}
            onClick={replaceEverything}
          >
            Replace all
          </button>
        </div>
      )}

      {scan.error && (
        <div className="find-replace-panel__error" role="alert" data-testid="find-error">
          {scan.error}
        </div>
      )}
      {lastOutcome && (
        <div className="find-replace-panel__outcome muted" data-testid="find-outcome" aria-live="polite">
          {lastOutcome}
        </div>
      )}
    </div>
  );
}
