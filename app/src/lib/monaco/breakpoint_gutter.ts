// WP-KERNEL-009 MT-254 DebugAdapterCore — Monaco breakpoint gutter.
//
// Pure, testable helpers for the glyph-margin breakpoint UX on a
// MonacoCodeBlockView: build the breakpoint glyph decorations + the
// current-stop line decoration, and resolve a glyph-margin mouse-down into a
// breakpoint toggle. The decoration option shapes are exactly what
// `editor.deltaDecorations` consumes, so the wiring layer can apply them
// directly; keeping the model pure lets the toggle/decoration logic be proven
// without a browser.

import type * as Monaco from "monaco-editor";

type MonacoApi = typeof Monaco;
type IModelDeltaDecoration = Monaco.editor.IModelDeltaDecoration;

/** CSS class names the gutter decorations use (styled in the editor CSS). */
export const BREAKPOINT_GLYPH_CLASS = "hsk-debug-breakpoint-glyph";
export const BREAKPOINT_UNVERIFIED_GLYPH_CLASS = "hsk-debug-breakpoint-glyph-unverified";
export const CURRENT_STOP_LINE_CLASS = "hsk-debug-current-stop-line";
export const CURRENT_STOP_GLYPH_CLASS = "hsk-debug-current-stop-glyph";

/** A breakpoint as the gutter renders it (line + verified state). */
export type GutterBreakpoint = {
  line: number;
  verified: boolean;
};

/** Monaco's `MouseTargetType.GUTTER_GLYPH_MARGIN` numeric value. */
export const GUTTER_GLYPH_MARGIN = 2;

/**
 * Build the glyph-margin decorations for the breakpoint set. A verified
 * breakpoint gets the solid glyph; an unverified one gets the hollow glyph so
 * the operator can SEE that the adapter did not bind it (never faked verified).
 */
export function buildBreakpointDecorations(
  monaco: MonacoApi,
  breakpoints: GutterBreakpoint[],
): IModelDeltaDecoration[] {
  return breakpoints.map((bp) => ({
    range: new monaco.Range(bp.line, 1, bp.line, 1),
    options: {
      isWholeLine: false,
      glyphMarginClassName: bp.verified
        ? BREAKPOINT_GLYPH_CLASS
        : BREAKPOINT_UNVERIFIED_GLYPH_CLASS,
      glyphMarginHoverMessage: {
        value: bp.verified
          ? "Breakpoint (verified)"
          : "Breakpoint (not bound — will not pause)",
      },
      stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
    },
  }));
}

/**
 * Build the current-stop decoration: a whole-line highlight + a glyph at the
 * paused line. Returns an empty array when there is no current stop in this
 * source (so the wiring can clear the decoration by passing `null`).
 */
export function buildCurrentStopDecoration(
  monaco: MonacoApi,
  line: number | null,
): IModelDeltaDecoration[] {
  if (line === null || line < 1) return [];
  return [
    {
      range: new monaco.Range(line, 1, line, 1),
      options: {
        isWholeLine: true,
        className: CURRENT_STOP_LINE_CLASS,
        glyphMarginClassName: CURRENT_STOP_GLYPH_CLASS,
        glyphMarginHoverMessage: { value: "Paused here" },
        stickiness: monaco.editor.TrackedRangeStickiness.NeverGrowsWhenTypingAtEdges,
      },
    },
  ];
}

/** A Monaco editor mouse-down target (the bits the gutter handler reads). */
export type GutterMouseTarget = {
  type: number;
  position: { lineNumber: number } | null;
};

/**
 * Resolve a Monaco editor mouse-down into the line whose breakpoint should
 * toggle, or null if the click was not on the glyph margin. This is the
 * `editor.onMouseDown` handler's pure core.
 */
export function resolveGutterToggleLine(target: GutterMouseTarget): number | null {
  if (target.type !== GUTTER_GLYPH_MARGIN) return null;
  const line = target.position?.lineNumber;
  if (typeof line !== "number" || line < 1) return null;
  return line;
}
