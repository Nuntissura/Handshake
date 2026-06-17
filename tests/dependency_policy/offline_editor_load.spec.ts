// WP-KERNEL-009 / MT-020 + MT-027 + MT-030 — offline editor load proof.
//
// Serves the BUILT harness (app/dist-harness) from a loopback-only static
// server, blocks every non-loopback request at the browser network layer,
// and proves:
//   - the app shell renders (no blank screen),
//   - Monaco mounts from bundled assets and its TypeScript worker performs a
//     REAL round-trip (web worker booted from a locally served chunk),
//   - the Tiptap editor instantiates,
//   - REAL keyboard interaction works offline (MT-030): text typed into both
//     editors renders, and the Tiptap ``` input rule creates a code block,
//   - ZERO external network requests are attempted (request ledger empty).
//
// This is the runtime counterpart of the static MT-027 worker-bundling check:
// `pnpm run check:worker-bundling` proves the built chunks reference only
// local URLs; this spec proves the chunks actually BOOT with the network cut.

import { expect, test } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { createReadStream, existsSync, statSync } from "node:fs";
import type { AddressInfo } from "node:net";
import path from "node:path";

// CJS context (Playwright transpiles spec files to CJS; import.meta unavailable).
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
    // Loopback only: the proof must not depend on (or expose) the LAN.
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

test.describe("WP-KERNEL-009 offline editor load (built assets, network blocked)", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "dependency-policy.html")),
      "dist-harness missing — global setup should have built it (pnpm run build:harness)",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("Monaco + Tiptap boot offline with zero external requests", async ({ page }, testInfo) => {
    const externalRequests: string[] = [];
    const servedRequests: string[] = [];

    // Network kill-switch: anything not addressed to our loopback server is
    // aborted and recorded. A non-empty ledger fails the proof.
    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(baseUrl)) {
        servedRequests.push(url);
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(`${baseUrl}/harness/dependency-policy.html`);

    // App shell renders — no blank screen.
    await expect(page.getByTestId("dependency-policy-harness-root")).toBeVisible();

    // Monaco boots and its TypeScript worker answers a real request.
    await page.waitForFunction(
      () => window.__HARNESS_STATE__?.monacoReady === true,
      undefined,
      { timeout: 60_000 },
    );
    const state = await page.evaluate(() => window.__HARNESS_STATE__);
    expect(state?.monacoWorkerProof).toBe("ts-worker-responded");
    expect(state?.monacoWorkerProofs).toEqual({
      editor: "worker-constructed",
      typescript: "ts-worker-responded",
      json: "worker-constructed",
      css: "worker-constructed",
      html: "worker-constructed",
    });
    expect(state?.errors ?? []).toEqual([]);
    expect(state?.failures ?? []).toEqual([]);

    // Monaco actually rendered into the host (real DOM, not just state flags).
    await expect(page.locator("[data-testid='monaco-host'] .monaco-editor").first()).toBeVisible();
    await expect(page.getByText("knowledgeIndexEntry").first()).toBeVisible();

    // Tiptap instantiated with its extension set.
    await page.waitForFunction(() => window.__HARNESS_STATE__?.tiptapReady === true);
    const tiptapExtensions = await page.evaluate(
      () => window.__HARNESS_STATE__?.tiptapExtensions ?? [],
    );
    expect(tiptapExtensions.length).toBeGreaterThan(5);
    await expect(page.locator("[data-testid='tiptap-host'] .tiptap").first()).toBeVisible();

    // ---- MT-030: the editors must WORK offline, not merely render. ----

    // Real typing into Tiptap (ProseMirror contenteditable). Caret placement
    // goes through the editor model API (deterministic across headless
    // keyboard-navigation quirks; Ctrl+Home/End landed inside the harness
    // task list, whose taskItem schema cannot host a code block) — the
    // TYPING itself is real keyboard input.
    const tiptapSurface = page.locator("[data-testid='tiptap-host'] .tiptap").first();
    await page.evaluate(() => window.__HARNESS_STATE__?.tiptapFocusFreshLeadingParagraph?.());
    await page.keyboard.type("Typed offline through the bundled Tiptap stack.");
    await expect(
      tiptapSurface.getByText("Typed offline through the bundled Tiptap stack."),
    ).toBeVisible();

    // Create a code block via the StarterKit ``` input rule and type into it.
    await page.keyboard.press("Enter");
    await page.keyboard.type("```ts ");
    await page.keyboard.type('const offlineProof = "code block created";');
    const codeBlock = page.locator("[data-testid='tiptap-host'] .tiptap pre code").first();
    await expect(codeBlock).toBeVisible();
    await expect(codeBlock).toContainText('const offlineProof = "code block created";');

    // The typed content landed in the document model, not just the DOM.
    const tiptapDocText = await page.evaluate(
      () => window.__HARNESS_STATE__?.tiptapDocText?.() ?? "",
    );
    expect(tiptapDocText).toContain("Typed offline through the bundled Tiptap stack.");
    expect(tiptapDocText).toContain('const offlineProof = "code block created";');

    // Real typing into Monaco: the bundled editor accepts input and renders it.
    await page.locator("[data-testid='monaco-host'] .view-lines").first().click();
    await page.keyboard.press("Control+End");
    await page.keyboard.press("Enter");
    await page.keyboard.type("// offline monaco typing proof");
    await expect(page.getByText("// offline monaco typing proof").first()).toBeVisible();

    // Worker chunks were really fetched from the loopback server.
    expect(
      servedRequests.some((url) => /worker-[^/]*\.js/.test(url)),
      `no worker chunk request seen; served: ${servedRequests.join(", ")}`,
    ).toBe(true);
    for (const workerChunk of [
      "editor.worker-",
      "ts.worker-",
      "json.worker-",
      "css.worker-",
      "html.worker-",
    ]) {
      await expect
        .poll(
          async () => servedRequests.some((url) => url.includes(workerChunk)),
          { message: `missing loopback-served ${workerChunk} chunk` },
        )
        .toBe(true);
    }

    // THE offline guarantee: zero external requests attempted.
    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);

    // Visual evidence for the validator.
    await page.screenshot({
      path: testInfo.outputPath("offline-editor-load.png"),
      fullPage: true,
    });
  });
});

declare global {
  interface Window {
    __HARNESS_STATE__?: {
      monacoReady: boolean;
      monacoWorkerProof: string | null;
      monacoWorkerProofs?: Record<"editor" | "typescript" | "json" | "css" | "html", string>;
      tiptapReady: boolean;
      tiptapExtensions: string[];
      tiptapDocText?: () => string;
      tiptapFocusFreshLeadingParagraph?: () => void;
      failures: Array<{ dependency: string; component: string; message: string }>;
      errors: string[];
    };
  }
}
