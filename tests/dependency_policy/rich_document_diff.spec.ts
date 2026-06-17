// WP-KERNEL-009 / MT-247 — rich-document history diff + Monaco diff proof.
//
// Serves the BUILT harness (app/dist-harness/harness/rich-document-diff.html)
// from loopback, blocks non-loopback requests, and proves the diff surface
// renders prose block diffs plus a REAL Monaco diff editor offline.

import { expect, test } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { createReadStream, existsSync, statSync } from "node:fs";
import type { AddressInfo } from "node:net";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");

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

test.describe("WP-KERNEL-009 MT-247 rich-document diff proof (offline, network blocked)", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "rich-document-diff.html")),
      "dist-harness missing rich-document-diff.html; global setup should build it",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    if (server) await new Promise((resolve) => server.close(resolve));
  });

  test("history-pair diff renders prose rows and a real Monaco code diff offline", async ({ page }, testInfo) => {
    const externalRequests: string[] = [];
    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/rich-document-diff.html`);
    await expect(page.getByTestId("rich-document-diff-harness-root")).toBeVisible();
    await page.waitForFunction(
      () => window.__HS_RICH_DOCUMENT_DIFF_HARNESS__?.monacoDiffReady === true,
      undefined,
      { timeout: 60_000 },
    );

    const state = await page.evaluate(() => window.__HS_RICH_DOCUMENT_DIFF_HARNESS__);
    expect(state?.errors ?? []).toEqual([]);
    expect(state?.blockKinds).toEqual(["prose", "code"]);
    expect(state?.blockStatuses).toEqual(["modified", "modified"]);
    expect(state?.leftCode).toContain("const count = 1;");
    expect(state?.rightCode).toContain("const count = 2;");
    expect(state?.diffLineChanges ?? 0).toBeGreaterThan(0);

    await expect(page.getByTestId("rich-document-diff-block")).toHaveCount(2);
    await expect(page.getByTestId("rich-document-code-diff-monaco").locator(".monaco-diff-editor")).toBeVisible();
    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);

    await page.screenshot({ path: testInfo.outputPath("mt247-rich-document-diff.png"), fullPage: true });
  });
});

declare global {
  interface Window {
    __HS_RICH_DOCUMENT_DIFF_HARNESS__?: {
      monacoDiffReady: boolean;
      diffLineChanges: number | null;
      blockKinds: string[];
      blockStatuses: string[];
      leftCode: string;
      rightCode: string;
      errors: string[];
    };
  }
}
