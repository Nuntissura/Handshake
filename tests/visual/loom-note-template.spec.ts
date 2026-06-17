// WP-KERNEL-009 / MT-258 - note template insert browser proof.
//
// Drives the production-built rich-editor harness offline: opens the command
// palette, runs the daily-note template command with a {{title}} variable, and
// proves the editor stamps the structured template (title heading, the {{date}}
// expansion, and the Notes/Links section headings) into the live document with
// ZERO external network requests. Templates are data, not code (no eval).

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { expect, test } from "./console_error_scan";

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

test.describe("WP-KERNEL-009 MT-258 note template insert", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "rich-editor.html")),
      "dist-harness missing rich-editor.html; run pnpm run build:harness first",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("inserts the daily-note template with variable expansion", async ({ page }) => {
    const externalRequests: string[] = [];
    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      if (!url.startsWith("about:")) externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/rich-editor.html`);
    await expect(page.getByTestId("rich-editor-harness-root")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();

    // Open the command palette and run the daily-note template command.
    await page.getByTestId("editor-open-palette").click();
    await expect(page.getByTestId("editor-command-palette")).toBeVisible();
    await page.getByTestId("editor-command-palette-input").fill("daily");
    await page.getByTestId("palette-cmd-template.note.daily").click();

    // The command requires a title arg — fill it and confirm.
    await expect(page.getByTestId("editor-arg-prompt")).toBeVisible();
    await page.getByTestId("editor-arg-title").fill("Sprint planning note");
    await page.getByTestId("editor-arg-confirm").click();

    // The structured template is stamped into the live document.
    const editor = page.getByTestId("rich-text-editor");
    await expect(editor.getByRole("heading", { name: "Sprint planning note" })).toBeVisible();
    await expect(editor.getByRole("heading", { name: "Notes" })).toBeVisible();
    await expect(editor.getByRole("heading", { name: "Links" })).toBeVisible();
    // {{date}} expanded to an ISO date (YYYY-MM-DD), not the literal token.
    await expect(editor.getByText(/Date: \d{4}-\d{2}-\d{2}/)).toBeVisible();
    await expect(editor).not.toContainText("{{title}}");
    await expect(editor).not.toContainText("{{date}}");

    expect(externalRequests).toEqual([]);
  });
});
