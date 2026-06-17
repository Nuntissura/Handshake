// WP-KERNEL-009 / MT-246 - same-document split editor CRDT consistency proof.
//
// Serves the built RichTextEditor collaboration harness from loopback, blocks
// every non-loopback request, and proves two mounted editor groups bound to the
// same Yjs document converge after editing one group.

import { expect, test } from "@playwright/test";
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
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

type HarnessSnapshot = {
  ready: boolean;
  editorBMounted: boolean;
  consistent: boolean;
  editorAText: string;
  editorBText: string;
  collaborationDocumentGuid: string;
};

type HarnessWindow = Window & {
  __MT246_COLLAB_HARNESS__?: {
    getState: () => HarnessSnapshot;
    applyEdit: (text?: string) => HarnessSnapshot;
    openSecondEditor: () => HarnessSnapshot;
  };
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

test.describe("WP-KERNEL-009 MT-246 rich-editor same-document collaboration", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "rich-editor-collaboration.html")),
      "dist-harness missing rich-editor-collaboration.html; global setup should build it",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("a late-joining editor group bound to one Yjs document converges with zero external requests", async ({
    page,
  }, testInfo) => {
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

    await page.goto(`${baseUrl}/harness/rich-editor-collaboration.html`);

    await expect(page.getByTestId("mt246-collaboration-root")).toBeVisible();
    await expect(page.getByTestId("mt246-editor-group-a").getByTestId("rich-text-editor")).toBeVisible();
    await expect(page.getByTestId("mt246-editor-group-b-placeholder")).toBeVisible();
    await expect(page.getByTestId("mt246-editor-group-a")).toContainText("shared alpha");

    await expect
      .poll(async () =>
        page.evaluate(() => (window as HarnessWindow).__MT246_COLLAB_HARNESS__?.getState().ready ?? false),
      )
      .toBe(true);

    const editText = "shared beta browser proof";
    await page.evaluate(
      (text) => (window as HarnessWindow).__MT246_COLLAB_HARNESS__?.applyEdit(text),
      editText,
    );
    await expect(page.getByTestId("mt246-editor-group-a")).toContainText(editText);
    await page.evaluate(() => (window as HarnessWindow).__MT246_COLLAB_HARNESS__?.openSecondEditor());
    await expect(page.getByTestId("mt246-editor-group-b").getByTestId("rich-text-editor")).toBeVisible();

    await expect
      .poll(async () =>
        page.evaluate(() => (window as HarnessWindow).__MT246_COLLAB_HARNESS__?.getState().editorBText ?? ""),
      )
      .toContain(editText);
    await expect
      .poll(async () =>
        page.evaluate(() => (window as HarnessWindow).__MT246_COLLAB_HARNESS__?.getState().consistent ?? false),
      )
      .toBe(true);

    const snapshot = await page.evaluate(
      () => (window as HarnessWindow).__MT246_COLLAB_HARNESS__?.getState(),
    );
    expect(snapshot?.editorAText).toContain(editText);
    expect(snapshot?.editorBText).toContain(editText);
    expect(snapshot?.editorBMounted).toBe(true);
    expect(snapshot?.collaborationDocumentGuid).toBe("mt246-same-document-proof");
    await expect(page.getByTestId("mt246-editor-group-a")).toContainText(editText);
    await expect(page.getByTestId("mt246-editor-group-b")).toContainText(editText);
    await expect(page.getByTestId("mt246-consistency-status")).toHaveAttribute("data-consistent", "true");

    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);
    await page.screenshot({ path: testInfo.outputPath("mt246-same-document-collaboration.png"), fullPage: true });
  });
});
