// WP-KERNEL-009 / MT-258 - Loom note transclusion browser proof.
//
// Serves the production-built transclusion harness, fulfills ONLY the
// transclusion read-through endpoint and the source-document save endpoint,
// blocks other network, and proves:
//   1. read-through renders the SOURCE document content,
//   2. "Edit source" routes the save to the SOURCE document (request evidence),
//   3. NO-COPY: the HOST doc onChange JSON keeps only the atom node (no copied
//      source body), even after the source is edited.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import type { Route } from "@playwright/test";
import { expect, test } from "./console_error_scan";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");
const apiBaseUrl = "http://127.0.0.1:37501";
const workspaceId = "ws-mt258-transclusion";
const blockId = "block-source";
const sourceDocumentId = "KRD-source-001";

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
  "access-control-allow-methods": "GET, PUT, OPTIONS",
  "access-control-allow-headers": "content-type, x-hsk-actor-id, x-hsk-actor-kind, x-hsk-kernel-task-run-id, x-hsk-session-run-id",
};

type RequestEvidence = { method: string; path: string; body: unknown };

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

function jsonResponse(route: Route, body: unknown) {
  return route.fulfill({
    status: 200,
    headers: { ...CORS_HEADERS, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
}

function doc(text: string) {
  return {
    type: "doc",
    content: [{ type: "paragraph", content: [{ type: "text", text }] }],
  };
}

test.describe("WP-KERNEL-009 MT-258 Loom transclusion", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "loom-transclusion.html")),
      "dist-harness missing loom-transclusion.html; run pnpm run build:harness first",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("renders source read-through, routes edits to source, host stays copy-free", async ({ page }) => {
    // Mutable source state — the backend mock returns the live source content
    // and version; a save advances it (single authority).
    const sourceState = { version: 1, body: "ORIGINAL source body" };
    const evidence: RequestEvidence[] = [];
    const externalRequests: string[] = [];

    const transclusionPath = `/workspaces/${workspaceId}/loom/blocks/${blockId}/transclusion`;
    const savePath = `/knowledge/documents/${sourceDocumentId}/save`;

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
        if (parsed.pathname === transclusionPath && method === "GET") {
          evidence.push({ method, path: parsed.pathname, body: null });
          return jsonResponse(route, {
            block_id: blockId,
            workspace_id: workspaceId,
            source_document_id: sourceDocumentId,
            source_doc_version: sourceState.version,
            content_json: doc(sourceState.body),
            resolved: true,
          });
        }
        if (parsed.pathname === savePath && method === "PUT") {
          const body = route.request().postDataJSON() as { content_json: unknown };
          evidence.push({ method, path: parsed.pathname, body });
          sourceState.version += 1;
          sourceState.body = "EDITED via transclusion source";
          return jsonResponse(route, {
            document: {
              rich_document_id: sourceDocumentId,
              doc_version: sourceState.version,
              content_json: body.content_json,
            },
            save_receipt_event_id: "EVT-MT258-SAVE",
            receipt_error: null,
          });
        }
      }
      if (!url.startsWith("about:")) externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/loom-transclusion.html`);
    await expect(page.getByTestId("loom-transclusion-harness-root")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();

    // 1. READ-THROUGH: the SOURCE content renders inside the transclusion node.
    const content = page.getByTestId("loom-transclusion-content");
    await expect(content).toBeVisible();
    await expect(content).toContainText("ORIGINAL source body");
    await expect(page.getByTestId("loom-transclusion-source")).toContainText(sourceDocumentId);

    // 2. EDIT-ROUTES-TO-SOURCE: edit the read-through and save.
    await page.getByTestId("loom-transclusion.edit-source").click();
    const editable = content.locator(".ProseMirror");
    await editable.click();
    await page.keyboard.press("Control+A");
    await page.keyboard.type("EDITED via transclusion source");
    await page.getByTestId("loom-transclusion.save-source").click();
    await expect(page.getByTestId("loom-transclusion.save-status")).toContainText("Source saved");

    // The save targeted the SOURCE document id (not the host doc).
    const saveCalls = evidence.filter((item) => item.path === savePath && item.method === "PUT");
    expect(saveCalls.length).toBe(1);
    const savedBody = saveCalls[0].body as { content_json: { content?: Array<{ content?: Array<{ text?: string }> }> } };
    const savedText = JSON.stringify(savedBody.content_json);
    expect(savedText).toContain("EDITED via transclusion source");

    // After save the read-through re-resolves to the NEW source content/version.
    await expect(content).toContainText("EDITED via transclusion source");
    expect(evidence.filter((item) => item.path === transclusionPath).length).toBeGreaterThanOrEqual(2);

    // 3. NO-COPY: the HOST document JSON (what the editor would persist) keeps
    // ONLY the atom node — no copied source body, before OR after the edit.
    const hostJson = await page.evaluate(() => window.__LOOM_TRANSCLUSION_HARNESS__?.docJson ?? null);
    const hostText = JSON.stringify(hostJson ?? {});
    expect(hostText).toContain("Host preamble paragraph.");
    expect(hostText).toContain("loomTransclusion");
    expect(hostText).toContain("block-source");
    expect(hostText).not.toContain("ORIGINAL source body");
    expect(hostText).not.toContain("EDITED via transclusion source");

    expect(externalRequests).toEqual([]);
  });
});
