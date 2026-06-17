// WP-KERNEL-009 / MT-234 - editor visual regression fixture.
//
// Serves the production-built editor visual regression harness from loopback,
// blocks non-loopback requests, and proves that the real rich editor controls,
// embeds, graph/backlink panes, and UserManual links are visible without
// incoherent overlap on the committed desktop/narrow Playwright visual matrix.

import { expect, test, type Page } from "@playwright/test";
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { assertRenderedPngQuality } from "./image_quality";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");
const fixturePath = path.join(distHarness, "harness", "editor-visual-regression.html");
const fixturePng = Buffer.from(
  "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAIAAAAmkwkpAAAAGUlEQVR42mP8z8AARLJgwiSgQpgYVQAANe8CFcNvUBsAAAAASUVORK5CYII=",
  "base64",
);

const CONTENT_TYPES: Record<string, string> = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".mjs": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".json": "application/json",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ttf": "font/ttf",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".wasm": "application/wasm",
};

type Rect = {
  testId: string;
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
};

type VisualHarnessWindow = Window & {
  __HS_EDITOR_DEBUG__?: {
    codeBlocks?: Array<{ language: string; contentHash: string; codeLength: number }>;
    links?: Array<{ refKind: string; refValue: string; resolved: boolean }>;
    nodeCounts?: Record<string, number>;
  };
};

type TableCellRect = {
  left: number;
  top: number;
  width: number;
  height: number;
};

function serveDistHarness(): Promise<Server> {
  const server = createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url ?? "/").split("?")[0]);
    const assetMatch = /^\/workspaces\/ws-mt234-visual\/assets\/([^/]+)(\/content)?$/.exec(urlPath);
    if (assetMatch) {
      const assetId = assetMatch[1];
      if (!["img-ok", "slide-a", "slide-b"].includes(assetId)) {
        res.writeHead(404, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: "unknown fixture asset" }));
        return;
      }
      if (assetMatch[2] === "/content") {
        res.writeHead(200, { "content-type": "image/png", "content-length": String(fixturePng.byteLength) });
        res.end(fixturePng);
        return;
      }
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          asset_id: assetId,
          workspace_id: "ws-mt234-visual",
          kind: "image",
          mime: "image/png",
          original_filename: `${assetId}.png`,
          content_hash: assetId.padEnd(64, "0").slice(0, 64),
          size_bytes: fixturePng.byteLength,
          width: 4,
          height: 4,
        }),
      );
      return;
    }
    const safePath = path
      .normalize(urlPath)
      .replace(/^([/\\])+/, "")
      .replace(/^(\.\.([/\\]|$))+/, "");
    const filePath = path.join(distHarness, safePath);
    if (!filePath.startsWith(distHarness) || !existsSync(filePath) || !statSync(filePath).isFile()) {
      res.writeHead(404);
      res.end("not found");
      return;
    }
    res.writeHead(200, {
      "content-type": CONTENT_TYPES[path.extname(filePath).toLowerCase()] ?? "application/octet-stream",
    });
    createReadStream(filePath).pipe(res);
  });
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

async function rectByTestId(page: import("@playwright/test").Page, testId: string): Promise<Rect> {
  const locator = page.getByTestId(testId).first();
  return rectByLocator(locator, testId);
}

async function rectByLocator(locator: import("@playwright/test").Locator, testId: string): Promise<Rect> {
  await expect(locator, `${testId} should be visible before geometry capture`).toBeVisible();
  const box = await locator.boundingBox();
  expect(box, `${testId} should have layout bounds`).toBeTruthy();
  return {
    testId,
    left: box!.x,
    top: box!.y,
    right: box!.x + box!.width,
    bottom: box!.y + box!.height,
    width: box!.width,
    height: box!.height,
  };
}

function overlapArea(a: Rect, b: Rect): number {
  const width = Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left));
  const height = Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));
  return width * height;
}

function assertVisibleInsideViewport(rect: Rect, viewport: { width: number; height: number }): void {
  expect(rect.left, `${rect.testId} clips past left viewport edge`).toBeGreaterThanOrEqual(0);
  expect(rect.top, `${rect.testId} clips past top viewport edge`).toBeGreaterThanOrEqual(0);
  expect(rect.right, `${rect.testId} clips past right viewport edge`).toBeLessThanOrEqual(viewport.width + 1);
  expect(rect.bottom, `${rect.testId} clips past bottom viewport edge`).toBeLessThanOrEqual(viewport.height + 1);
  expect(rect.width, `${rect.testId} should have nonzero width`).toBeGreaterThan(0);
  expect(rect.height, `${rect.testId} should have nonzero height`).toBeGreaterThan(0);
}

function assertInsideViewport(rect: Rect, viewport: { width: number; height: number }): void {
  assertVisibleInsideViewport(rect, viewport);
  expect(rect.width, `${rect.testId} should not collapse`).toBeGreaterThan(16);
  expect(rect.height, `${rect.testId} should not collapse`).toBeGreaterThan(16);
}

function assertNoOverlap(a: Rect, b: Rect): void {
  expect(overlapArea(a, b), `${a.testId} overlaps ${b.testId}`).toBe(0);
}

function assertContains(outer: Rect, inner: Rect): void {
  expect(inner.left, `${inner.testId} clips past ${outer.testId} left`).toBeGreaterThanOrEqual(outer.left - 1);
  expect(inner.top, `${inner.testId} clips past ${outer.testId} top`).toBeGreaterThanOrEqual(outer.top - 1);
  expect(inner.right, `${inner.testId} clips past ${outer.testId} right`).toBeLessThanOrEqual(outer.right + 1);
  expect(inner.bottom, `${inner.testId} clips past ${outer.testId} bottom`).toBeLessThanOrEqual(outer.bottom + 1);
}

function originOf(url: string): string | null {
  try {
    return new URL(url).origin;
  } catch {
    return null;
  }
}

async function editorNodeCount(page: Page, type: string): Promise<number> {
  return page.evaluate((nodeType) => {
    return (window as VisualHarnessWindow).__HS_EDITOR_DEBUG__?.nodeCounts?.[nodeType] ?? 0;
  }, type);
}

async function editorTableCellCount(page: Page): Promise<number> {
  return page.evaluate(() => {
    const counts = (window as VisualHarnessWindow).__HS_EDITOR_DEBUG__?.nodeCounts ?? {};
    return (counts.tableCell ?? 0) + (counts.tableHeader ?? 0);
  });
}

async function expectNodeCount(page: Page, type: string, count: number): Promise<void> {
  await expect.poll(() => editorNodeCount(page, type), {
    message: `${type} node count should be ${count}`,
  }).toBe(count);
}

async function expectTableCellCount(page: Page, count: number): Promise<void> {
  await expect.poll(() => editorTableCellCount(page), {
    message: `total table cell count should be ${count}`,
  }).toBe(count);
}

async function openOverflow(page: Page): Promise<void> {
  await page.getByTestId("editor-open-overflow").click();
  await expect(page.getByTestId("rich-text-editor-overflow")).toBeVisible();
}

async function runOverflowCommand(page: Page, id: string): Promise<void> {
  await openOverflow(page);
  const button = page.getByTestId(`overflow-cmd-${id}`);
  await expect(button, `${id} overflow command should be visible`).toBeVisible();
  await expect(button, `${id} overflow command should be enabled`).toBeEnabled();
  await button.click();
  await expect(page.getByTestId("rich-text-editor-overflow")).toHaveCount(0);
}

async function clickTableCell(page: Page, index = 0): Promise<void> {
  const cells = page.locator(".ProseMirror table th, .ProseMirror table td");
  await expect.poll(() => cells.count(), {
    message: "table cells should be rendered before selecting one",
  }).toBeGreaterThan(index);
  await cells.nth(index).click();
}

async function dragSelectAdjacentTableCells(page: Page): Promise<void> {
  const cells = page.locator(".ProseMirror table th, .ProseMirror table td");
  await expect.poll(() => cells.count(), {
    message: "table should expose at least two rendered cells before merge",
  }).toBeGreaterThanOrEqual(2);
  const rects = await cells.evaluateAll((nodes) =>
    nodes.slice(0, 2).map((node) => {
      const rect = (node as HTMLElement).getBoundingClientRect();
      return { left: rect.left, top: rect.top, width: rect.width, height: rect.height };
    }),
  ) as TableCellRect[];
  expect(rects.length, "adjacent cells should have layout boxes").toBeGreaterThanOrEqual(2);
  const [first, second] = rects;
  await page.mouse.move(first.left + first.width / 2, first.top + first.height / 2);
  await page.mouse.down();
  await page.mouse.move(second.left + second.width / 2, second.top + second.height / 2, { steps: 8 });
  await page.mouse.up();
  await expect.poll(async () => page.getByTestId("rich-text-editor").evaluate((root) =>
    root.querySelectorAll(".selectedCell, .ProseMirror-selectedcell").length,
  ), {
    message: "dragging adjacent cells should create a ProseMirror table cell selection",
  }).toBeGreaterThanOrEqual(2);
}

test.describe("WP-KERNEL-009 MT-234 editor visual regression fixture", () => {
  let server: Server | null = null;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(fixturePath),
      "dist-harness missing editor-visual-regression.html; run pnpm run build:harness",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
    expect(originOf(`${baseUrl}/harness/editor-visual-regression.html`)).toBe(baseUrl);
    expect(originOf(`${baseUrl}@example.invalid/harness/editor-visual-regression.html`)).not.toBe(baseUrl);
    expect(originOf(`${baseUrl}1/harness/editor-visual-regression.html`)).not.toBe(baseUrl);
  });

  test.afterAll(async () => {
    if (!server) return;
    await new Promise((resolve) => server?.close(resolve));
  });

  test("editor text, controls, embeds, graph/backlink panes, and UserManual links do not overlap", async ({
    page,
  }, testInfo) => {
    const externalRequests: string[] = [];
    const failedSameOriginResponses: string[] = [];

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (originOf(url) === baseUrl) {
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });
    page.on("response", (response) => {
      const url = new URL(response.url());
      if (`${url.protocol}//${url.host}` !== baseUrl) return;
      if (response.status() < 400) return;
      failedSameOriginResponses.push(`${response.status()} ${url.pathname}`);
    });

    await page.goto(`${baseUrl}/harness/editor-visual-regression.html`);

    await expect(page.getByTestId("editor-visual-regression-root")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor-toolbar")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor-surface")).toBeVisible();
    await expect(page.getByTestId("hs-link").first()).toBeVisible();
    await expect(page.getByTestId("editor-visual-regression-embeds")).toBeVisible();
    await expect(page.getByTestId("editor-visual-regression-backlinks")).toBeVisible();
    await expect(page.getByTestId("editor-visual-regression-usermanual")).toBeVisible();
    await expect(page.getByTestId("editor-visual-regression-usermanual-link")).toBeVisible();

    await expect.poll(async () => page.getByTestId("hs-link").count()).toBeGreaterThanOrEqual(4);
    await expect(page.getByTestId("hs-embed-image").first()).toBeVisible();
    await expect(page.getByTestId("hs-embed-sequence").first()).toBeVisible();
    await expect(page.getByTestId("hs-embed-error")).toHaveCount(0);
    await expect
      .poll(async () =>
        page.evaluate(
          () => (window as VisualHarnessWindow).__HS_EDITOR_DEBUG__?.nodeCounts?.paragraph ?? 0,
        ),
      )
      .toBeGreaterThanOrEqual(1);
    const debug = await page.evaluate(() => (window as VisualHarnessWindow).__HS_EDITOR_DEBUG__);
    expect(debug?.links?.some((link) => link.refKind === "spec")).toBe(true);
    expect(debug?.links?.some((link) => link.refKind === "note")).toBe(true);
    expect(debug?.links?.some((link) => link.refKind === "images")).toBe(true);
    expect(debug?.links?.some((link) => link.refKind === "slideshow")).toBe(true);

    const viewport = page.viewportSize();
    expect(viewport, "visual project must define a viewport").toBeTruthy();
    const [
      layout,
      editorPanel,
      sidecar,
      toolbar,
      surface,
      embeds,
      backlinks,
      userManual,
    ] = await Promise.all([
      rectByTestId(page, "editor-visual-regression-layout"),
      rectByTestId(page, "editor-visual-regression-editor-panel"),
      rectByTestId(page, "editor-visual-regression-sidecar"),
      rectByTestId(page, "rich-text-editor-toolbar"),
      rectByTestId(page, "rich-text-editor-surface"),
      rectByTestId(page, "editor-visual-regression-embeds"),
      rectByTestId(page, "editor-visual-regression-backlinks"),
      rectByTestId(page, "editor-visual-regression-usermanual"),
    ]);

    for (const rect of [layout, editorPanel, sidecar, toolbar, surface, embeds, backlinks, userManual]) {
      assertInsideViewport(rect, viewport!);
    }
    assertContains(layout, editorPanel);
    assertContains(layout, sidecar);
    assertContains(editorPanel, toolbar);
    assertContains(editorPanel, surface);
    assertNoOverlap(toolbar, surface);
    assertNoOverlap(editorPanel, sidecar);
    assertNoOverlap(embeds, backlinks);
    assertNoOverlap(embeds, userManual);
    assertNoOverlap(backlinks, userManual);
    if (viewport!.width <= 700) {
      expect(editorPanel.bottom, "narrow layout should stack editor above sidecar").toBeLessThanOrEqual(
        sidecar.top + 1,
      );
    } else {
      expect(editorPanel.right, "desktop layout should place editor beside sidecar").toBeLessThanOrEqual(
        sidecar.left + 1,
      );
    }

    await page.getByTestId("editor-toggle-focus-mode").click();
    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-focus-mode", "true");
    await expect(page.getByTestId("rich-text-editor-toolbar")).toHaveAttribute("data-focus-mode", "true");
    await expect(page.getByTestId("rich-text-editor-surface")).toHaveAttribute("data-focus-mode", "true");
    await expect(page.getByTestId("editor-toggle-focus-mode")).toHaveAttribute("aria-pressed", "true");
    await expect(page.getByTestId("editor-cmd-format.bold")).toHaveAttribute("data-focus-mode-hidden", "true");
    await expect(page.getByTestId("editor-cmd-format.bold")).toBeHidden();
    const focusEditorPanel = await rectByTestId(page, "editor-visual-regression-editor-panel");
    const focusSidecar = await rectByTestId(page, "editor-visual-regression-sidecar");
    const focusToolbar = await rectByTestId(page, "rich-text-editor-toolbar");
    const focusSurface = await rectByTestId(page, "rich-text-editor-surface");
    assertInsideViewport(focusEditorPanel, viewport!);
    assertInsideViewport(focusToolbar, viewport!);
    assertInsideViewport(focusSurface, viewport!);
    assertContains(focusEditorPanel, focusToolbar);
    assertContains(focusEditorPanel, focusSurface);
    assertNoOverlap(focusToolbar, focusSurface);
    assertNoOverlap(focusEditorPanel, focusSidecar);
    await expect(page.getByTestId("editor-command-palette")).toHaveCount(0);
    await expect(page.getByTestId("editor-arg-prompt")).toHaveCount(0);
    await expect(page.getByTestId("find-panel")).toHaveCount(0);
    const focusScreenshot = await page.screenshot({
      path: testInfo.outputPath(`editor-focus-mode-${testInfo.project.name}.png`),
      fullPage: true,
    });
    assertRenderedPngQuality(
      focusScreenshot,
      {
        minWidth: viewport!.width,
        minHeight: viewport!.height,
        minDistinctColorBuckets: 24,
        maxBackgroundPixelRatio: 0.995,
        minNonBackgroundPixelRatio: 0.01,
        minSaturatedPixels: 80,
        minSaturatedPixelRatio: 0.0002,
      },
      `MT-251 editor focus mode ${testInfo.project.name}`,
    );
    await page.getByTestId("editor-toggle-focus-mode").click();
    await expect(page.getByTestId("rich-text-editor")).toHaveAttribute("data-focus-mode", "false");
    await expect(page.getByTestId("editor-cmd-format.bold")).toHaveAttribute("data-focus-mode-hidden", "false");

    const linkLocator = page.getByTestId("hs-link");
    const regularLinkLocator = page.locator("[data-testid='hs-link'].hs-link--resolved:not(.hs-embed)");
    const linkCount = await linkLocator.count();
    const regularLinkCount = await regularLinkLocator.count();
    const allLinkRects: Rect[] = [];
    for (let index = 0; index < linkCount; index += 1) {
      allLinkRects.push(await rectByLocator(linkLocator.nth(index), `hs-link:${index}`));
    }
    expect(regularLinkCount, "regular inline editor links should stay present").toBeGreaterThanOrEqual(3);
    const regularLinkRects: Rect[] = [];
    for (let index = 0; index < regularLinkCount; index += 1) {
      regularLinkRects.push(await rectByLocator(regularLinkLocator.nth(index), `hs-link:regular:${index}`));
    }
    const firstEmbedImage = await rectByLocator(page.getByTestId("hs-embed-image").first(), "hs-embed-image:first");
    const firstEmbedSequence = await rectByLocator(
      page.getByTestId("hs-embed-sequence").first(),
      "hs-embed-sequence:first",
    );
    const editorText = await rectByLocator(
      page.getByText("Operator-facing links:", { exact: false }).first(),
      "editor-text:operator-links",
    );
    const manualLink = await rectByTestId(page, "editor-visual-regression-usermanual-link");
    assertContains(surface, editorText);
    for (const linkRect of allLinkRects) {
      assertVisibleInsideViewport(linkRect, viewport!);
      assertContains(surface, linkRect);
    }
    for (const linkRect of regularLinkRects) {
      assertNoOverlap(linkRect, firstEmbedImage);
      assertNoOverlap(linkRect, firstEmbedSequence);
    }
    for (let leftIndex = 0; leftIndex < regularLinkRects.length; leftIndex += 1) {
      for (let rightIndex = leftIndex + 1; rightIndex < regularLinkRects.length; rightIndex += 1) {
        assertNoOverlap(regularLinkRects[leftIndex], regularLinkRects[rightIndex]);
      }
    }
    assertContains(surface, firstEmbedImage);
    assertContains(surface, firstEmbedSequence);
    assertContains(userManual, manualLink);
    assertNoOverlap(editorText, firstEmbedImage);

    const sidecarOverflow = await page
      .locator("[data-testid^='editor-visual-regression-']")
      .evaluateAll((nodes) =>
        nodes
          .filter((node) =>
            [
              "editor-visual-regression-embeds",
              "editor-visual-regression-backlinks",
              "editor-visual-regression-usermanual",
            ].includes(node.getAttribute("data-testid") ?? ""),
          )
          .map((node) => ({
            testId: node.getAttribute("data-testid"),
            clientHeight: (node as HTMLElement).clientHeight,
            scrollHeight: (node as HTMLElement).scrollHeight,
            clientWidth: (node as HTMLElement).clientWidth,
            scrollWidth: (node as HTMLElement).scrollWidth,
          })),
      );
    for (const entry of sidecarOverflow) {
      expect(entry.scrollHeight, `${entry.testId} vertical content clips`).toBeLessThanOrEqual(
        entry.clientHeight + 1,
      );
      expect(entry.scrollWidth, `${entry.testId} horizontal content clips`).toBeLessThanOrEqual(
        entry.clientWidth + 1,
      );
    }

    await page.getByTestId("editor-open-overflow").click();
    await expect(page.getByTestId("rich-text-editor-overflow")).toBeVisible();
    await expect(page.getByTestId("overflow-cmd-graph.backlink")).toBeVisible();
    await expect(page.getByTestId("overflow-cmd-manual.insert")).toBeVisible();
    const overflow = await rectByTestId(page, "rich-text-editor-overflow");
    assertInsideViewport(overflow, viewport!);
    assertContains(editorPanel, overflow);
    await page.getByTestId("overflow-close").click();

    await page.getByTestId("editor-open-palette").click();
    await expect(page.getByTestId("editor-command-palette")).toBeVisible();
    await page.getByTestId("editor-command-palette-input").fill("manual");
    await expect(page.getByTestId("palette-cmd-manual.insert")).toBeVisible();
    await page.getByTestId("editor-command-palette-input").fill("backlink");
    await expect(page.getByTestId("palette-cmd-graph.backlink")).toBeVisible();
    const palette = await rectByTestId(page, "editor-command-palette");
    assertInsideViewport(palette, viewport!);
    assertContains(editorPanel, palette);

    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);
    expect(
      failedSameOriginResponses,
      `unexpected same-origin HTTP failures: ${failedSameOriginResponses.join(", ")}`,
    ).toEqual([]);

    const screenshot = await page.screenshot({
      path: testInfo.outputPath(`editor-visual-regression-${testInfo.project.name}.png`),
      fullPage: true,
    });
    assertRenderedPngQuality(
      screenshot,
      {
        minWidth: viewport!.width,
        minHeight: viewport!.height,
        minDistinctColorBuckets: 24,
        maxBackgroundPixelRatio: 0.995,
        minNonBackgroundPixelRatio: 0.01,
        minSaturatedPixels: 80,
        minSaturatedPixelRatio: 0.0002,
      },
      `MT-234 editor visual regression ${testInfo.project.name}`,
    );
  });

  test("MT-251 table operations run through real editor command surfaces", async ({ page }) => {
    const externalRequests: string[] = [];
    const failedSameOriginResponses: string[] = [];

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (originOf(url) === baseUrl) {
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });
    page.on("response", (response) => {
      const url = new URL(response.url());
      if (`${url.protocol}//${url.host}` !== baseUrl) return;
      if (response.status() < 400) return;
      failedSameOriginResponses.push(`${response.status()} ${url.pathname}`);
    });

    await page.goto(`${baseUrl}/harness/editor-visual-regression.html`);
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor-toolbar")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor-surface")).toBeVisible();

    await openOverflow(page);
    await expect(page.getByTestId("overflow-cmd-table.addRowBefore")).toBeDisabled();
    await expect(page.getByTestId("overflow-cmd-table.addColumnBefore")).toBeDisabled();
    await expect(page.getByTestId("overflow-cmd-table.mergeCells")).toBeDisabled();
    await page.getByTestId("overflow-close").click();

    await page.getByTestId("editor-cmd-table.insert").click();
    await expectNodeCount(page, "table", 1);
    await expectNodeCount(page, "tableRow", 3);
    await expectNodeCount(page, "tableHeader", 3);
    await expectNodeCount(page, "tableCell", 6);
    await expectTableCellCount(page, 9);
    await expect(page.locator(".ProseMirror table")).toBeVisible();

    await page.waitForTimeout(700);
    await runOverflowCommand(page, "table.addRowBefore");
    await expectNodeCount(page, "tableRow", 4);
    await expectTableCellCount(page, 12);
    await expect(page.getByTestId("editor-cmd-history.undo")).toBeEnabled();
    await page.getByTestId("editor-cmd-history.undo").click();
    await expectNodeCount(page, "tableRow", 3);
    await expectTableCellCount(page, 9);

    await clickTableCell(page);
    await page.waitForTimeout(700);
    await runOverflowCommand(page, "table.addColumnBefore");
    await expectTableCellCount(page, 12);
    await page.getByTestId("editor-cmd-history.undo").click();
    await expectTableCellCount(page, 9);

    await clickTableCell(page);
    await page.waitForTimeout(700);
    await runOverflowCommand(page, "table.toggleHeaderCell");
    await expectNodeCount(page, "tableHeader", 2);
    await page.getByTestId("editor-cmd-history.undo").click();
    await expectNodeCount(page, "tableHeader", 3);

    await clickTableCell(page);
    await page.waitForTimeout(700);
    await runOverflowCommand(page, "table.toggleHeaderColumn");
    await expect.poll(() => editorNodeCount(page, "tableHeader"), {
      message: "toggleHeaderColumn should increase header cells for the selected column",
    }).toBeGreaterThan(3);
    await page.getByTestId("editor-cmd-history.undo").click();
    await expectNodeCount(page, "tableHeader", 3);

    await clickTableCell(page);
    await page.waitForTimeout(700);
    await dragSelectAdjacentTableCells(page);
    await runOverflowCommand(page, "table.mergeCells");
    await expectTableCellCount(page, 8);
    await runOverflowCommand(page, "table.splitCell");
    await expectTableCellCount(page, 9);

    await clickTableCell(page);
    await page.waitForTimeout(700);
    await runOverflowCommand(page, "table.deleteRow");
    await expectNodeCount(page, "tableRow", 2);

    await clickTableCell(page);
    await page.waitForTimeout(700);
    await runOverflowCommand(page, "table.deleteColumn");
    await expectTableCellCount(page, 4);

    await runOverflowCommand(page, "table.delete");
    await expectNodeCount(page, "table", 0);

    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);
    expect(
      failedSameOriginResponses,
      `unexpected same-origin HTTP failures: ${failedSameOriginResponses.join(", ")}`,
    ).toEqual([]);
  });
});
