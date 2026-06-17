// WP-KERNEL-009 MT-254 DebugAdapterCore — breakpoint gutter unit proofs.

import { describe, expect, it } from "vitest";
import {
  buildBreakpointDecorations,
  buildCurrentStopDecoration,
  resolveGutterToggleLine,
  BREAKPOINT_GLYPH_CLASS,
  BREAKPOINT_UNVERIFIED_GLYPH_CLASS,
  CURRENT_STOP_LINE_CLASS,
  GUTTER_GLYPH_MARGIN,
} from "./breakpoint_gutter";

// Minimal monaco facade: only what the pure helpers touch.
class FakeRange {
  constructor(
    public startLineNumber: number,
    public startColumn: number,
    public endLineNumber: number,
    public endColumn: number,
  ) {}
}

const monaco = {
  Range: FakeRange,
  editor: {
    TrackedRangeStickiness: { NeverGrowsWhenTypingAtEdges: 1 },
  },
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
} as any;

describe("MT-254 breakpoint gutter decorations", () => {
  it("renders verified breakpoints with the solid glyph and unverified with the hollow glyph", () => {
    const decos = buildBreakpointDecorations(monaco, [
      { line: 2, verified: true },
      { line: 7, verified: false },
    ]);
    expect(decos).toHaveLength(2);
    expect(decos[0].options.glyphMarginClassName).toBe(BREAKPOINT_GLYPH_CLASS);
    expect((decos[0].range as unknown as FakeRange).startLineNumber).toBe(2);
    expect(decos[1].options.glyphMarginClassName).toBe(BREAKPOINT_UNVERIFIED_GLYPH_CLASS);
  });

  it("builds the current-stop line decoration and clears when no stop", () => {
    const stop = buildCurrentStopDecoration(monaco, 3);
    expect(stop).toHaveLength(1);
    expect(stop[0].options.isWholeLine).toBe(true);
    expect(stop[0].options.className).toBe(CURRENT_STOP_LINE_CLASS);
    expect(buildCurrentStopDecoration(monaco, null)).toEqual([]);
    expect(buildCurrentStopDecoration(monaco, 0)).toEqual([]);
  });
});

describe("MT-254 gutter mouse-down resolution", () => {
  it("toggles only on glyph-margin clicks", () => {
    expect(
      resolveGutterToggleLine({ type: GUTTER_GLYPH_MARGIN, position: { lineNumber: 5 } }),
    ).toBe(5);
    // A click in the text body (not the glyph margin) does not toggle.
    expect(resolveGutterToggleLine({ type: 6, position: { lineNumber: 5 } })).toBeNull();
    expect(resolveGutterToggleLine({ type: GUTTER_GLYPH_MARGIN, position: null })).toBeNull();
  });
});
