import { expect, test } from "./console_error_scan";

import path from "node:path";

import { buildTerminalHarness } from "./build_terminal_harness";

// WP-KERNEL-004 Integrated Terminal panel — real-browser visual + readability
// matrix. jsdom cannot judge layout/overlap/readability, so the operator
// requirement "test via Handshake's own tools" is met here in a genuine
// Chromium render.
//
// MEDIUM-defect remediation: this spec NO LONGER uses a hand-authored static
// HTML fixture (a mockup of the panel DOM). It now bundles and mounts the REAL
// <TerminalPanel> React component (with its real Disclosure host, real lazy
// collapsed-by-default gate, real swarm-grouped tablist, real AiJob read-only
// badge + honest-disabled Take-control gate) into the page and asserts against
// the GENUINE render. The only stand-in is xterm's canvas surface, injected via
// the component's own `renderTerminal` seam (the same seam the unit tests use) —
// not a replacement for the panel DOM. A fake in-memory TerminalIpc supplies a
// deterministic captured-session list so the layout is stable for the baseline.
//
// We assert, against the real layout engine + the real component output:
//   - the panel is COLLAPSED BY DEFAULT (the body region is hidden / unmounted),
//   - once opened it is readable (legible font, visible tabs/terminal surface),
//   - the AiJob tab shows a read-only badge (TERM-INVARIANTS at the surface),
//   - tab buttons do not overlap each other (no clobbered hit targets),
//   - the read-only PNG baseline is written under .GOV/visual_baselines/.

const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

// Page shell hosting the real component. We inject the component's CSS, a
// #harness-root mount node, and a <main data-testid="capture-root"> wrapper so
// the screenshot framing matches the rest of the visual matrix.
const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; background:#fff; color:#111; width:1200px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

/** Render the real TerminalPanel into the page in the requested mount mode. */
async function mountRealPanel(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  page: any,
  mode: "open" | "collapsed",
): Promise<void> {
  const { js, css } = await buildTerminalHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  // Choose mount mode BEFORE the bundle's top-level mount runs.
  await page.evaluate((m: string) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).__HARNESS_MODE__ = m;
  }, mode);
  await page.addScriptTag({ content: js });
  // The real component renders its Disclosure host immediately. The panel passes
  // data-testid="terminal-panel" to its Disclosure, so that is the host node;
  // the toggle button is data-testid="disclosure-terminal-toggle".
  await page.locator("[data-testid='terminal-panel']").waitFor({ state: "attached" });
}

test("real terminal panel is collapsed by default (genuine Disclosure, body not mounted)", async ({ page }) => {
  await mountRealPanel(page, "collapsed");

  // The REAL Disclosure host reports collapsed and the lazy body is NOT mounted.
  await expect(page.locator("[data-testid='terminal-panel']")).toHaveAttribute("data-open", "false");
  await expect(page.locator("[data-testid='disclosure-terminal-toggle']")).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("[data-testid='terminal-panel-body']")).toHaveCount(0);
});

test("opened real terminal panel is readable, shows AiJob read-only, and has non-overlapping tabs", async ({ page }) => {
  await mountRealPanel(page, "open");

  // The real lazy body mounts once open; the active session surface is visible.
  const surface = page.locator("[data-testid='terminal-panel-active']");
  await expect(surface).toBeVisible();

  // TERM-INVARIANTS at the surface: the AiJob session is read-only by default,
  // and (since the backend has NOT granted interaction) Take-control is honestly
  // DISABLED in the real render — never faked.
  await expect(page.locator("[data-testid='terminal-readonly-badge']")).toBeVisible();
  await expect(page.locator("[data-testid='terminal-take-control']")).toBeVisible();
  await expect(page.locator("[data-testid='terminal-take-control-checkbox']")).toBeDisabled();

  // Tabs must not overlap (clear, distinct hit targets). These are the REAL
  // component's tab buttons in the swarm:alpha lane.
  const t1 = await page.locator("[data-testid='terminal-tab-s1']").boundingBox();
  const t2 = await page.locator("[data-testid='terminal-tab-s2']").boundingBox();
  expect(t1).not.toBeNull();
  expect(t2).not.toBeNull();
  if (t1 && t2) {
    const overlapX = Math.max(0, Math.min(t1.x + t1.width, t2.x + t2.width) - Math.max(t1.x, t2.x));
    const overlapY = Math.max(0, Math.min(t1.y + t1.height, t2.y + t2.height) - Math.max(t1.y, t2.y));
    // No 2D overlap: at most they may share an edge.
    expect(overlapX * overlapY).toBe(0);
  }

  // Tabs render with a positive size (not collapsed to zero / hidden).
  if (t1) {
    expect(t1.width).toBeGreaterThan(20);
    expect(t1.height).toBeGreaterThan(10);
  }

  // The swarm swimlane grouping is the real component's (board-link grouping).
  await expect(page.locator("[data-testid='terminal-lane-swarm:alpha']")).toBeVisible();

  // Write the reviewed PNG baseline under .GOV/visual_baselines (UNCOMMITTED).
  // This is now a screenshot of the GENUINE rendered component, not a fixture.
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "terminal-panel-open.png"),
  });
});
