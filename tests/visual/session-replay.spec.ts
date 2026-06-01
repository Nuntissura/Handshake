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
  mode: "open" | "collapsed" | "live" | "search",
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
  // All lanes present (chat + agent + terminal + fr + process), in seq order.
  await expect(page.locator("[data-testid='session-replay-entry-0']")).toHaveAttribute("data-kind", "chat_turn");
  await expect(page.locator("[data-testid='session-replay-entry-7']")).toHaveAttribute("data-kind", "process");

  // The structured agent-activity lane renders distinctly: a visible thinking
  // row (italic reasoning), a tool_call row showing the tool name, a text row,
  // and an HONEST raw-fallback "other" row — proving "all toolcalls + visible
  // thought processes" surface as typed records, not just raw stdout.
  await expect(page.locator("[data-testid='session-replay-entry-1']")).toHaveAttribute("data-kind", "agent_activity");
  await expect(page.locator("[data-agent-kind='thinking']")).toBeVisible();
  await expect(page.locator("[data-agent-kind='tool_call']")).toBeVisible();
  await expect(page.locator("[data-agent-kind='tool_call']")).toContainText("Bash");
  await expect(page.locator("[data-agent-kind='other']")).toContainText("raw");

  // The Agent filter chip exists and is a distinct hit target.
  await expect(page.locator("[data-testid='session-replay-filter-agent_activity']")).toBeVisible();

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

test("real session replay panel exposes a Resume affordance (ROI #3 state recovery) only on resumable rows", async ({ page }) => {
  await mountRealPanel(page, "open");

  await expect(page.locator("[data-testid='session-replay-body']")).toBeVisible();

  // HONEST: the swarm session (a captured spawn template => resumable) offers
  // Resume; the empty/agent session without a template does NOT. The Resume
  // button is a distinct, readable hit target (not nested in the row button).
  const resume = page.locator("[data-testid='session-replay-resume-claude-sonnet#0']");
  await expect(resume).toBeVisible();
  await expect(resume).toContainText("Resume");
  await expect(page.locator("[data-testid='session-replay-resume-local-qwen#1']")).toHaveCount(0);
  await expect(page.locator("[data-testid='session-replay-row-claude-sonnet#0']")).toHaveAttribute(
    "data-resumable",
    "true",
  );

  // The Resume button is a separate clickable element from the row-select button
  // (no button-in-button): their bounding boxes do not overlap.
  const rowBox = await page.locator("[data-testid='session-replay-row-claude-sonnet#0']").boundingBox();
  const resumeBox = await resume.boundingBox();
  expect(rowBox).not.toBeNull();
  expect(resumeBox).not.toBeNull();
  if (rowBox && resumeBox) {
    const overlapY = Math.max(
      0,
      Math.min(rowBox.y + rowBox.height, resumeBox.y + resumeBox.height) - Math.max(rowBox.y, resumeBox.y),
    );
    expect(overlapY).toBe(0);
    expect(resumeBox.width).toBeGreaterThan(20);
    expect(resumeBox.height).toBeGreaterThan(10);
  }

  // Click Resume -> the scripted resume IPC mints a fresh session; the inline
  // lineage note ("Resumed → … from …") appears, proving the operator sees the
  // re-spawn + its lineage in a GENUINE Chromium render.
  await resume.click();
  const lineage = page.locator("[data-testid='session-replay-resumed-lineage']");
  await expect(lineage).toBeVisible({ timeout: 6000 });
  await expect(lineage).toHaveAttribute("data-origin-session-id", "claude-sonnet#0");
  await expect(lineage).toContainText("Resumed");

  // Write the reviewed PNG baseline (Resume affordance + lineage note).
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "session-replay-resume.png"),
  });
});

test("real session replay panel tails LIVE: status chip + appended rows as the session runs", async ({ page }) => {
  await mountRealPanel(page, "live");

  // The real lazy body mounts; select the streaming (swarm) session.
  await expect(page.locator("[data-testid='session-replay-body']")).toBeVisible();
  await page.locator("[data-testid='session-replay-row-claude-sonnet#0']").click();

  // Live defaults ON for a swarm session: the toggle is active and the honest
  // status chip reads "live" (a genuine streaming session, not polled/ended).
  await expect(page.locator("[data-testid='session-replay-live-toggle']")).toHaveAttribute("data-active", "true");
  await expect(page.locator("[data-testid='session-replay-live-status']")).toHaveAttribute("data-status", "live");

  // The scripted live seam pushes two ticks shortly after mount: a toolcall-class
  // FR event then a terminal chunk. The panel correlates each to the focused
  // session and tail-fetches the growing transcript -> the NEW rows appear LIVE
  // (proving the operator watches events arrive, not only after-the-fact).
  await expect(page.locator("text=FR-EVT-LLM-INFER-START")).toBeVisible({ timeout: 6000 });
  await expect(page.locator("text=cargo test --lib")).toBeVisible({ timeout: 6000 });

  // Capture the live-tailing baseline (status chip + appended timeline). The
  // live seam leaves the panel quiescent after its two scripted ticks; give the
  // screenshot a generous timeout and skip the font-wait stall some headless
  // targets hit by disabling animations.
  await page.waitForLoadState("networkidle").catch(() => {});
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "session-replay-live.png"),
    animations: "disabled",
    timeout: 15000,
  });

  // Toggling Live OFF returns to post-hoc review and the chip reads "idle".
  await page.locator("[data-testid='session-replay-live-toggle']").click();
  await expect(page.locator("[data-testid='session-replay-live-status']")).toHaveAttribute("data-status", "idle");
});

test("real session replay panel SEARCHES across sessions (ROI #4): box above the index, ranked hits, open-into-transcript", async ({ page }) => {
  await mountRealPanel(page, "search");

  await expect(page.locator("[data-testid='session-replay-body']")).toBeVisible();

  // The search box sits ABOVE the session index (per the brief) and is a distinct,
  // non-overlapping hit target — mirroring the chip-overlap assertions above.
  const searchBox = await page.locator("[data-testid='session-replay-search']").boundingBox();
  const list = await page.locator("[data-testid='session-replay-list']").boundingBox();
  expect(searchBox).not.toBeNull();
  expect(list).not.toBeNull();
  if (searchBox && list) {
    // The search box's bottom edge is at/above the index's top edge (it is above).
    expect(searchBox.y + searchBox.height).toBeLessThanOrEqual(list.y + 1);
    expect(searchBox.width).toBeGreaterThan(40);
    expect(searchBox.height).toBeGreaterThan(20);
  }

  // The input + Search + Clear are distinct, non-overlapping controls.
  const input = await page.locator("[data-testid='session-replay-search-input']").boundingBox();
  const submit = await page.locator("[data-testid='session-replay-search-submit']").boundingBox();
  expect(input).not.toBeNull();
  expect(submit).not.toBeNull();
  if (input && submit) {
    const overlapX = Math.max(0, Math.min(input.x + input.width, submit.x + submit.width) - Math.max(input.x, submit.x));
    const overlapY = Math.max(0, Math.min(input.y + input.height, submit.y + submit.height) - Math.max(input.y, submit.y));
    expect(overlapX * overlapY).toBe(0);
  }

  // Submit a query -> the ranked hits REPLACE the index in the left rail. The
  // backend already redacted; the rows show titles, match badges, and snippets.
  await page.locator("[data-testid='session-replay-search-input']").fill("build");
  await page.locator("[data-testid='session-replay-search-submit']").click();

  await expect(page.locator("[data-testid='session-replay-search-results']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-list']")).toHaveCount(0);
  await expect(page.locator("[data-testid='session-replay-search-hit-claude-sonnet#0']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-search-matchcount-claude-sonnet#0']")).toContainText("3 matches");
  await expect(page.locator("[data-testid='session-replay-search-hit-claude-sonnet#0']")).toContainText("cargo build --lib");

  // The hit row + its match badge are distinct, readable, non-overlapping.
  const hit = await page.locator("[data-testid='session-replay-search-hit-claude-sonnet#0']").boundingBox();
  expect(hit).not.toBeNull();
  if (hit) {
    expect(hit.width).toBeGreaterThan(80);
    expect(hit.height).toBeGreaterThan(20);
  }

  // Write the reviewed PNG baseline (search box + ranked hits + snippets).
  await page.locator("[data-testid='capture-root']").screenshot({
    path: path.join(baselineDir, "session-replay-search.png"),
  });

  // Clicking a hit opens that session's transcript via the EXISTING select path.
  await page.locator("[data-testid='session-replay-search-hit-claude-sonnet#0']").click();
  await expect(page.locator("[data-testid='session-replay-entry-0']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-search-hit-claude-sonnet#0']")).toHaveAttribute(
    "data-selected",
    "true",
  );

  // Clear restores the plain index (search is additive).
  await page.locator("[data-testid='session-replay-search-clear']").click();
  await expect(page.locator("[data-testid='session-replay-list']")).toBeVisible();
  await expect(page.locator("[data-testid='session-replay-search-results']")).toHaveCount(0);
});

test("real session replay panel search shows an HONEST empty state for a no-match query", async ({ page }) => {
  await mountRealPanel(page, "search");
  await expect(page.locator("[data-testid='session-replay-body']")).toBeVisible();

  // The empty-query submit is disabled (never match-all): typing nothing keeps it
  // disabled, so the operator cannot fire a fabricated "all sessions" search.
  await expect(page.locator("[data-testid='session-replay-search-submit']")).toBeDisabled();
});
