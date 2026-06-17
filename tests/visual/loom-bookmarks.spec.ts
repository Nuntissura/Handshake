// WP-KERNEL-009 / MT-258 - Loom bookmarks browser proof.
//
// Serves the production-built Loom bookmarks harness, fulfills the real
// WorkspaceSidebar/LoomBlockPanel API routes, blocks other network, and checks
// bookmark add/remove/navigation against backend request evidence.

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
const workspaceId = "w1";

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
  "access-control-allow-methods": "GET, PATCH, PUT, OPTIONS",
  "access-control-allow-headers": "content-type",
};

type LoomBlockFixture = {
  block_id: string;
  workspace_id: string;
  content_type: string;
  document_id: string | null;
  asset_id: string | null;
  title: string | null;
  original_filename: string | null;
  content_hash: string | null;
  pinned: boolean;
  favorite: boolean;
  pin_order: number | null;
  journal_date: string | null;
  created_at: string;
  updated_at: string;
  imported_at: string | null;
  derived: {
    full_text_index: string;
    backlink_count: number;
    mention_count: number;
    tag_count: number;
    preview_status: string;
  };
};

type RequestEvidence = {
  method: string;
  path: string;
  body: unknown;
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

function makeBlock(blockId: string, title: string, pinned: boolean, pinOrder: number | null): LoomBlockFixture {
  return {
    block_id: blockId,
    workspace_id: workspaceId,
    content_type: "note",
    document_id: null,
    asset_id: null,
    title,
    original_filename: null,
    content_hash: null,
    pinned,
    favorite: false,
    pin_order: pinOrder,
    journal_date: null,
    created_at: "2026-06-16T00:00:00Z",
    updated_at: "2026-06-16T00:00:00Z",
    imported_at: null,
    derived: {
      full_text_index: `${title} indexed body text.`,
      backlink_count: 1,
      mention_count: 2,
      tag_count: 3,
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

test.describe("WP-KERNEL-009 MT-258 Loom bookmarks", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "loom-bookmarks.html")),
      "dist-harness missing loom-bookmarks.html; run pnpm run build:harness first",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("adds, removes, and navigates Loom bookmarks through backend routes", async ({ page }, testInfo) => {
    const blocks = new Map<string, LoomBlockFixture>([
      ["block-alpha", makeBlock("block-alpha", "Pinned Alpha", true, 10)],
      ["block-unpinned", makeBlock("block-unpinned", "Unpinned Source", false, null)],
    ]);
    const requestEvidence: RequestEvidence[] = [];
    const externalRequests: string[] = [];

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      if (url.startsWith(apiBaseUrl)) {
        const parsed = new URL(url);
        const method = route.request().method();
        if (method === "OPTIONS") {
          await route.fulfill({ status: 204, headers: CORS_HEADERS, body: "" });
          return;
        }
        if (parsed.pathname === "/workspaces" && method === "GET") {
          return jsonResponse(route, [
            {
              id: workspaceId,
              name: "MT-258 Workspace",
              created_at: "2026-06-16T00:00:00Z",
              updated_at: "2026-06-16T00:00:00Z",
            },
          ]);
        }
        if (parsed.pathname === `/workspaces/${workspaceId}/documents` && method === "GET") {
          return jsonResponse(route, []);
        }
        if (parsed.pathname === `/workspaces/${workspaceId}/canvases` && method === "GET") {
          return jsonResponse(route, []);
        }
        if (parsed.pathname === `/workspaces/${workspaceId}/loom/views/pins` && method === "GET") {
          const pinnedBlocks = [...blocks.values()]
            .filter((block) => block.pinned)
            .sort((left, right) => (left.pin_order ?? 999_999) - (right.pin_order ?? 999_999));
          return jsonResponse(route, { view_type: "pins", blocks: pinnedBlocks });
        }

        const blockMatch = parsed.pathname.match(/^\/workspaces\/w1\/loom\/blocks\/([^/]+)(?:\/pin-order)?$/);
        if (blockMatch) {
          const blockId = decodeURIComponent(blockMatch[1]);
          const current = blocks.get(blockId);
          if (!current) {
            return route.fulfill({ status: 404, headers: CORS_HEADERS, body: "not found" });
          }
          if (!parsed.pathname.endsWith("/pin-order") && method === "GET") {
            return jsonResponse(route, current);
          }
          if (!parsed.pathname.endsWith("/pin-order") && method === "PATCH") {
            const body = route.request().postDataJSON() as Partial<LoomBlockFixture>;
            requestEvidence.push({ method, path: parsed.pathname, body });
            const next = {
              ...current,
              ...body,
              updated_at: "2026-06-16T00:10:00Z",
            };
            blocks.set(blockId, next);
            return jsonResponse(route, next);
          }
          if (parsed.pathname.endsWith("/pin-order") && method === "PUT") {
            const body = route.request().postDataJSON() as { pin_order: number | null };
            requestEvidence.push({ method, path: parsed.pathname, body });
            const next = {
              ...current,
              pin_order: body.pin_order,
              updated_at: "2026-06-16T00:11:00Z",
            };
            blocks.set(blockId, next);
            return jsonResponse(route, next);
          }
        }
      }
      if (!url.startsWith("about:")) externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/loom-bookmarks.html`);
    await expect(page.getByTestId("loom-bookmarks-harness-root")).toBeVisible();
    await expect(page.getByTestId("loom-bookmarks-tree")).toBeVisible();
    await expect(page.getByTestId("loom-bookmark.block-alpha")).toContainText("Pinned Alpha");
    await expect(page.getByTestId("loom-bookmark.block-unpinned")).toHaveCount(0);
    await expect(page.getByTestId("loom-bookmarks.selected-block")).toHaveText("block-unpinned");
    await expect(page.getByTestId("loom-block-panel")).toContainText("Unpinned Source");

    await page.getByTestId("loom-block-properties.pinned").check();
    await page.getByTestId("loom-block-properties.save").click();
    await expect(page.getByTestId("loom-block-properties.status")).toContainText("Properties saved");
    await expect(page.getByTestId("loom-bookmark.block-unpinned")).toContainText("Unpinned Source");

    await page.getByTestId("loom-bookmark.block-alpha.open").click();
    await expect(page.getByTestId("loom-bookmarks.selected-block")).toHaveText("block-alpha");
    await expect(page.getByTestId("loom-bookmarks.open-log")).toContainText("block-alpha");
    await expect(page.getByTestId("loom-block-panel")).toContainText("Pinned Alpha");

    await page.getByTestId("loom-bookmark.block-alpha.remove").click();
    await expect(page.getByTestId("loom-bookmarks.status")).toContainText("Bookmark removed");
    await expect(page.getByTestId("loom-bookmark.block-alpha")).toHaveCount(0);
    await expect(page.getByTestId("loom-bookmark.block-unpinned")).toBeVisible();
    await expect(page.getByTestId("loom-block-properties.pinned")).not.toBeChecked();

    await page.getByTestId("loom-block-properties.title").fill("Pinned Alpha after removal");
    await page.getByTestId("loom-block-properties.save").click();
    await expect(page.getByTestId("loom-block-properties.status")).toContainText("Properties saved");

    expect(requestEvidence).toEqual(
      expect.arrayContaining([
        {
          method: "PATCH",
          path: `/workspaces/${workspaceId}/loom/blocks/block-unpinned`,
          body: expect.objectContaining({ pinned: true }),
        },
        {
          method: "PATCH",
          path: `/workspaces/${workspaceId}/loom/blocks/block-alpha`,
          body: { pinned: false },
        },
        {
          method: "PUT",
          path: `/workspaces/${workspaceId}/loom/blocks/block-alpha/pin-order`,
          body: { pin_order: null },
        },
        {
          method: "PATCH",
          path: `/workspaces/${workspaceId}/loom/blocks/block-alpha`,
          body: expect.objectContaining({ title: "Pinned Alpha after removal", pinned: false }),
        },
      ]),
    );

    const rowBox = await page.getByTestId("loom-bookmark.block-unpinned").boundingBox();
    const openBox = await page.getByTestId("loom-bookmark.block-unpinned.open").boundingBox();
    const removeBox = await page.getByTestId("loom-bookmark.block-unpinned.remove").boundingBox();
    expect(rowBox, "bookmark row should have layout bounds").toBeTruthy();
    expect(openBox, "bookmark open target should have layout bounds").toBeTruthy();
    expect(removeBox, "bookmark remove target should have layout bounds").toBeTruthy();
    expect(openBox!.x + openBox!.width).toBeLessThanOrEqual(removeBox!.x + 1);
    expect(rowBox!.width).toBeGreaterThan(200);

    const screenshot = await page.getByTestId("loom-bookmarks-capture").screenshot({
      path: testInfo.outputPath("loom-bookmarks.png"),
    });
    assertRenderedPngQuality(
      screenshot,
      {
        minWidth: 300,
        minHeight: 260,
        minDistinctColorBuckets: 8,
        minSaturatedPixels: 0,
        minSaturatedPixelRatio: 0,
      },
      "MT-258 Loom bookmarks screenshot",
    );
    expect(externalRequests).toEqual([]);
  });
});
