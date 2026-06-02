import { expect, test } from "./console_error_scan";

import fs from "node:fs/promises";
import path from "node:path";

import { buildSwarmBoardHarness } from "./build_swarm_board_harness";
import {
  assertRenderedPngDelta,
  assertRenderedPngQuality,
  type RenderedPngDeltaMetrics,
  type RenderedPngQualityMetrics,
} from "./image_quality";

// WP-KERNEL-004 live SwarmBoard visual smoke. jsdom cannot judge column
// readability, lane affordance overlap, or the real board projection, so this
// spec mounts the actual <SwarmBoard> in Chromium with deterministic IPC/event
// mocks, writes per-run screenshot evidence into Playwright's output dir, and
// appends a structured HBR row when HANDSHAKE_VISUAL_SMOKE_REPORT is set.

type SwarmBoardSmokeReportRow = {
  schema_id: "hsk.visual.swarm_board_smoke_report@1";
  report_version: 1;
  run_id: string;
  project_name: string;
  viewport: { width: number; height: number } | null;
  pre_event_screenshot_path: string;
  screenshot_path: string;
  columns_count: number;
  capture_root_width: number;
  board_scroll_width: number;
  board_client_width: number;
  constrained_scroll_verified: boolean;
  image_quality_status: "passed";
  image_quality: RenderedPngQualityMetrics;
  image_delta_status: "passed";
  image_delta: RenderedPngDeltaMetrics;
  geometry_status: "passed";
  event_delta_status: "passed";
  total_duration_ms: number;
};

function smokeRunId(): string {
  return process.env.HANDSHAKE_VISUAL_SMOKE_RUN_ID || `swarm-board-smoke-${Date.now()}`;
}

function smokeReportPath(): string | null {
  return process.env.HANDSHAKE_VISUAL_SMOKE_REPORT || null;
}

async function appendSwarmBoardSmokeReport(row: SwarmBoardSmokeReportRow): Promise<void> {
  const reportPath = smokeReportPath();
  if (!reportPath) return;
  await fs.mkdir(path.dirname(reportPath), { recursive: true });
  await fs.appendFile(reportPath, `${JSON.stringify(row)}\n`, "utf8");
}

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    :root {
      --hs-color-surface: #ffffff;
      --hs-color-text: #111827;
      --hs-color-text-subtle: #6b7280;
    }
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; }
    button { font: inherit; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="box-sizing:border-box; padding:16px; background:#fff; color:#111; width:100%; max-width:1280px; min-width:0;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function mountBoard(page: any): Promise<void> {
  const { js, css } = await buildSwarmBoardHarness();
  await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
  await page.addScriptTag({ content: js });
  await page.locator("[data-testid='swarm-board']").waitFor({ state: "attached" });
}

test("live SwarmBoard renders grouped swarm/worktree lanes, enabled affordances, and non-overlapping columns", async ({ page }, testInfo) => {
  const startedAt = Date.now();
  await mountBoard(page);

  const board = page.locator("[data-testid='swarm-board']");
  await expect(board).toBeVisible();
  await expect(board).toContainText("2 booting");
  await expect(board).toContainText("1 running");
  await expect(board).toContainText("4 total");
  await expect(board).toContainText("swarm: swarm-alpha");
  await expect(board).toContainText("worktree: wt-beta");

  const inspect = page.locator("[data-testid='swarm-inspect-terminal-swarm-alpha']");
  await expect(inspect).toBeVisible();
  await expect(inspect).toBeEnabled();
  await inspect.click();
  await expect
    .poll(async () =>
      page.evaluate(() => (window as Window & { __HS_SWARM_BOARD_INSPECTED__?: string | null }).__HS_SWARM_BOARD_INSPECTED__),
    )
    .toBe("swarm-alpha");

  const review = page.locator("[data-testid='swarm-review-session-alpha-cloud#0']");
  await expect(review).toBeVisible();
  await expect(review).toBeEnabled();
  await review.click();
  await expect
    .poll(async () =>
      page.evaluate(() => (window as Window & { __HS_SWARM_BOARD_REVIEWED__?: string | null }).__HS_SWARM_BOARD_REVIEWED__),
    )
    .toBe("alpha-cloud#0");

  const laneBox = await page.locator("section", { hasText: "swarm: swarm-alpha" }).boundingBox();
  const captureBox = await page.locator("[data-testid='capture-root']").boundingBox();
  const columns = page.locator(".board-column");
  await expect(columns).toHaveCount(14);
  const first = await columns.nth(0).boundingBox();
  const second = await columns.nth(1).boundingBox();
  expect(laneBox).not.toBeNull();
  expect(captureBox).not.toBeNull();
  expect(first).not.toBeNull();
  expect(second).not.toBeNull();
  if (laneBox && captureBox && first && second) {
    expect(first.width).toBeGreaterThan(80);
    expect(first.height).toBeGreaterThan(20);
    const overlapX = Math.max(0, Math.min(first.x + first.width, second.x + second.width) - Math.max(first.x, second.x));
    const overlapY = Math.max(0, Math.min(first.y + first.height, second.y + second.height) - Math.max(first.y, second.y));
    expect(overlapX * overlapY).toBe(0);

    if (testInfo.project.name.includes("constrained")) {
      expect(captureBox.width).toBeLessThanOrEqual(390);
      expect(laneBox.width).toBeLessThanOrEqual(390);
    } else {
      expect(laneBox.width).toBeGreaterThan(900);
    }
  }

  const scrollGeometry = await page.locator("section", { hasText: "swarm: swarm-alpha" }).locator(".board-columns").evaluate((node) => ({
    clientWidth: node.clientWidth,
    scrollWidth: node.scrollWidth,
  }));
  const constrainedScrollVerified = testInfo.project.name.includes("constrained");
  if (constrainedScrollVerified) {
    expect(scrollGeometry.clientWidth).toBeLessThanOrEqual(390);
    expect(scrollGeometry.scrollWidth).toBeGreaterThan(scrollGeometry.clientWidth);
  }

  const captureRoot = page.locator("[data-testid='capture-root']");
  const preEventScreenshotPath = testInfo.outputPath("swarm-board-before-ready.png");
  const preEventScreenshotBytes = await captureRoot.screenshot({ path: preEventScreenshotPath });

  // Exercise the real mocked Tauri event path: LOADING -> READY must move the
  // alpha-local card without a full static remount. Emit during the poll so an
  // early async-listener race cannot make this a fixed-delay flake.
  await expect
    .poll(async () => {
      await page.evaluate(() =>
        (window as Window & { __HS_SWARM_BOARD_EMIT_READY__?: () => Promise<void> }).__HS_SWARM_BOARD_EMIT_READY__?.(),
      );
      return page.locator("[data-testid='swarm-board']").textContent();
    })
    .toContain("1 booting");
  await expect(board).toContainText("1 booting");
  await expect(board).toContainText("alpha-lo#0");

  const screenshotPath = testInfo.outputPath("swarm-board-live.png");
  const screenshotBytes = await captureRoot.screenshot({ path: screenshotPath });
  const imageQuality = assertRenderedPngQuality(
    screenshotBytes,
    {
      minWidth: testInfo.project.name.includes("constrained") ? 300 : 900,
      minHeight: 180,
      minDistinctColorBuckets: 32,
      maxBackgroundPixelRatio: 0.99,
      minNonBackgroundPixelRatio: 0.01,
      minSaturatedPixels: 120,
      minSaturatedPixelRatio: 0.0005,
    },
    `SwarmBoard ${testInfo.project.name}`,
  );
  const imageDelta = assertRenderedPngDelta(
    preEventScreenshotBytes,
    screenshotBytes,
    {
      minChangedPixels: 24,
      minChangedPixelRatio: 0.00001,
    },
    `SwarmBoard READY transition ${testInfo.project.name}`,
  );
  await appendSwarmBoardSmokeReport({
    schema_id: "hsk.visual.swarm_board_smoke_report@1",
    report_version: 1,
    run_id: smokeRunId(),
    project_name: testInfo.project.name,
    viewport: testInfo.project.use.viewport ?? null,
    pre_event_screenshot_path: preEventScreenshotPath,
    screenshot_path: screenshotPath,
    columns_count: await columns.count(),
    capture_root_width: Math.round(captureBox?.width ?? 0),
    board_scroll_width: scrollGeometry.scrollWidth,
    board_client_width: scrollGeometry.clientWidth,
    constrained_scroll_verified: constrainedScrollVerified,
    image_quality_status: "passed",
    image_quality: imageQuality,
    image_delta_status: "passed",
    image_delta: imageDelta,
    geometry_status: "passed",
    event_delta_status: "passed",
    total_duration_ms: Date.now() - startedAt,
  });
});
