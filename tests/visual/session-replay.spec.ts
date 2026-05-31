import { expect, test } from "./console_error_scan";

import path from "node:path";

import { buildSessionReplayHarness } from "./build_session_replay_harness";

// WP-KERNEL-004 Session Replay panel — real-browser visual + readability matrix.
// jsdom cannot judge layout/overlap/readability, so the operator requirement
// "test via Handshake's own tools" is met here in a GENUINE Chromium render of
// the REAL <SessionReplayPanel> (its real Disclosure host, real lazy
// collapsed-by-default gate, real session index, real consolidated typed
// timeline, real kind-filter bar, real honest empty/unavailable states). The
// only stand-in is an in-memory SessionTranscriptIpc (one populated + one empty
// session) supplied via the component's own `ipc` seam — not a replacement for
// the panel DOM.
//
// We assert, against the real layout engine + the real component output:
//   - the panel is COLLAPSED BY DEFAULT (the body region is not mounted),
//   - once opened it is readable (legible index rows + typed timeline rows),
//   - selecting the populated session renders its consolidated timeline,
//   - filter chips are distinct, non-overlapping hit targets,
//   - selecting the EMPTY session shows the honest per-lane empty state,
//   - the read-only PNG baseline is written under .GOV/visual_baselines/.

const repoRoot = path.resolve(__dirname, "..", "..");
const baselineDir = path.join(repoRoot, ".GOV", "visual_baselines");

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

/** Render the real SessionReplayPanel into the page in the requested mount mode. */
async function mountRealPanel(
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  page: any,
  mode: "open" | "collapsed",
): Promise<void> {
  const { js, css } = await buildSessionReplayHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.evaluate((m: string) => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).__HARNESS_MODE__ = m;
  }, mode);
  await page.addScriptTag({ content: js });
  await page.locator("[data-testid='session-replay-panel']").waitFor({ state: "attached" });
}

test("real session replay panel is collapsed by default (genuine Disclosure, body not mounted)", async ({ page }) => {
  await mountRealPanel(page, "collapsed");

  await expect(page.locator("[data-testid='session-replay-panel']")).toHaveAttribute("data-open", "false");
  await expect(page.locator("[data-testid='disclosure-session-replay-toggle']")).toHaveAttribute("aria-expanded", "false");
  await expect(page.locator("[data-testid='session-replay-body']")).toHaveCount(0);
});

test("opened real session replay panel is readable: index + consolidated timeline + honest empty lane", async ({ page }) => {
  await mountRealPanel(page, "open");

  // The real lazy body mounts once open; the session index lists both sessions.
  await expect(page.locator("[data-testid='session-replay-body']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-row-claude-sonnet#0']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-row-local-qwen#1']")).toBeVisible();

  // Select the populated session -> the consolidated, typed, ordered timeline.
  await page.locator("[data-testid='session-replay-row-claude-sonnet#0']").click();
  await expect(page.locator("[data-testid='session-replay-entry-0']")).toBeVisible();
  // All four lanes present (chat + terminal + fr + process), in seq order.
  await expect(page.locator("[data-testid='session-replay-entry-0']")).toHaveAttribute("data-kind", "chat_turn");
  await expect(page.locator("[data-testid='session-replay-entry-3']")).toHaveAttribute("data-kind", "process");

  // Filter chips are distinct, non-overlapping hit targets (no clobbered chips).
  const chat = await page.locator("[data-testid='session-replay-filter-chat_turn']").boundingBox();
  const term = await page.locator("[data-testid='session-replay-filter-terminal_chunk']").boundingBox();
  expect(chat).not.toBeNull();
  expect(term).not.toBeNull();
  if (chat && term) {
    const overlapX = Math.max(0, Math.min(chat.x + chat.width, term.x + term.width) - Math.max(chat.x, term.x));
    const overlapY = Math.max(0, Math.min(chat.y + chat.height, term.y + term.height) - Math.max(chat.y, term.y));
    expect(overlapX * overlapY).toBe(0);
    expect(chat.width).toBeGreaterThan(20);
    expect(chat.height).toBeGreaterThan(10);
  }

  // Write the reviewed PNG baseline (populated timeline) under .GOV/visual_baselines.
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "session-replay-open.png"),
  });

  // Select the EMPTY session -> honest per-lane empty state, never fabricated rows.
  await page.locator("[data-testid='session-replay-row-local-qwen#1']").click();
  await expect(page.locator("[data-testid='session-replay-empty']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-empty-chat_turn']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-entry-0']")).toHaveCount(0);
});
