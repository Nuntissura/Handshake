// WP-KERNEL-009 / MT-258 - Loom link hover-preview browser proof.
//
// Serves the production-built Loom hover-preview harness, fulfills only the
// Loom-block API request needed by the hsLink preview, blocks other network,
// and checks the preview's rendered geometry plus a screenshot quality gate.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import type { Route } from "@playwright/test";
import { expect, test } from "./console_error_scan";
import { assertRenderedPngQuality } from "./image_quality";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");
const apiBaseUrl = "http://127.0.0.1:37501";
const workspaceId = "ws-mt258-preview";
const blockId = "block-alpha";

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

const CORS_HEADERS = {
  "access-control-allow-origin": "*",
  "access-control-allow-methods": "GET, OPTIONS",
  "access-control-allow-headers": "content-type",
};

function serveDistHarness(): Promise<Server> {
  const server = createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url ?? "/").split("?")[0]);
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

function loomBlockResponse() {
  return {
    block_id: blockId,
    workspace_id: workspaceId,
    content_type: "note",
    document_id: null,
    asset_id: null,
    title: "Alpha Loom note",
    original_filename: null,
    content_hash: null,
    pinned: false,
    favorite: false,
    pin_order: null,
    journal_date: null,
    created_at: "2026-06-16T00:00:00Z",
    updated_at: "2026-06-16T00:00:00Z",
    imported_at: null,
    derived: {
      full_text_index: "Alpha hover preview text from the indexed Loom block body.",
      backlink_count: 2,
      mention_count: 3,
      tag_count: 4,
      preview_status: "ready",
    },
  };
}

function jsonResponse(route: Route, body: unknown) {
  return route.fulfill({
    status: 200,
    headers: {
      ...CORS_HEADERS,
      "content-type": "application/json",
    },
    body: JSON.stringify(body),
  });
}

test.describe("WP-KERNEL-009 MT-258 Loom hover preview", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "loom-hover-preview.html")),
      "dist-harness missing loom-hover-preview.html; run pnpm run build:harness first",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("renders a readable Loom-block preview on hsLink hover without external network", async ({ page }, testInfo) => {
    const previewRequests: string[] = [];
    const externalRequests: string[] = [];

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      if (url.startsWith(apiBaseUrl)) {
        const parsed = new URL(url);
        if (
          parsed.pathname === `/workspaces/${workspaceId}/loom/blocks/${blockId}` &&
          route.request().method() === "OPTIONS"
        ) {
          await route.fulfill({ status: 204, headers: CORS_HEADERS, body: "" });
          return;
        }
        if (
          parsed.pathname === `/workspaces/${workspaceId}/loom/blocks/${blockId}` &&
          route.request().method() === "GET"
        ) {
          previewRequests.push(url);
          await jsonResponse(route, loomBlockResponse());
          return;
        }
      }
      if (!url.startsWith("about:")) externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/loom-hover-preview.html`);
    await expect(page.getByTestId("loom-hover-preview-harness-root")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();

    const chip = page.getByTestId("hs-link").filter({ hasText: "Alpha Loom note" });
    await expect(chip).toHaveAttribute("data-previewable", "true");
    await chip.hover();

    const preview = page.getByTestId("hs-link-preview");
    await expect(preview).toContainText("Alpha Loom note");
    await expect(preview).toContainText("Alpha hover preview text");
    await expect(preview).toContainText("4 tags");
    await expect(preview).toContainText("3 mentions");
    await expect(preview).toContainText("2 backlinks");
    expect(previewRequests).toEqual([`${apiBaseUrl}/workspaces/${workspaceId}/loom/blocks/${blockId}`]);

    const captureBox = await page.getByTestId("loom-hover-preview-capture").boundingBox();
    const previewBox = await preview.boundingBox();
    const viewport = page.viewportSize();
    expect(captureBox, "capture surface should have layout bounds").toBeTruthy();
    expect(previewBox, "preview should have layout bounds").toBeTruthy();
    expect(viewport, "visual project should define a viewport").toBeTruthy();
    const previewLeft = previewBox!.x;
    const previewRight = previewBox!.x + previewBox!.width;
    const previewTop = previewBox!.y;
    const previewBottom = previewBox!.y + previewBox!.height;
    expect(previewLeft).toBeGreaterThanOrEqual(0);
    expect(previewRight).toBeLessThanOrEqual(viewport!.width);
    expect(previewTop).toBeGreaterThanOrEqual(0);
    expect(previewBottom).toBeLessThanOrEqual(viewport!.height);
    expect(previewBox!.width).toBeGreaterThan(220);
    expect(previewBox!.height).toBeGreaterThan(64);

    const screenshot = await page.getByTestId("loom-hover-preview-capture").screenshot({
      path: testInfo.outputPath("loom-hover-preview.png"),
    });
    assertRenderedPngQuality(
      screenshot,
      {
        minWidth: 300,
        minHeight: 160,
        minDistinctColorBuckets: 8,
        minSaturatedPixels: 0,
        minSaturatedPixelRatio: 0,
      },
      "MT-258 Loom hover-preview screenshot",
    );
    expect(externalRequests).toEqual([]);
  });
});
