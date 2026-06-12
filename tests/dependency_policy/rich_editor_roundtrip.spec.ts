// WP-KERNEL-009 / MT-175 + MT-176 — integrated rich-editor offline round-trip.
//
// Serves the BUILT harness (app/dist-harness/harness/rich-editor.html) from a
// loopback-only static server, blocks every non-loopback request at the browser
// network layer, and proves on the INTEGRATED RichTextEditor surface:
//   - the editor + embedded Monaco code block + typed wikilinks render from
//     Handshake-bundled assets with the network cut — NO external editor, NO
//     external server, NO CDN (MT-175),
//   - the stable visual-debug selectors are present and the code block mounts
//     (MT-172/176),
//   - editing the code block + a content round-trip preserves language + text +
//     hash (MT-176 deterministic round-trip),
//   - ZERO external network requests are attempted (MT-175).
//
// This is the integrated-editor counterpart of offline_editor_load.spec.ts
// (which proves the bare bundled stack boots). The backend save/load round-trip
// against the real authority API is covered by the RichDocumentView vitest
// suite; this spec must not require a server, so its "round-trip" is the
// in-harness serialize→re-hydrate the editor publishes on window.

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
    server.listen(0, "127.0.0.1", () => resolve(server));
  });
}

interface EditorDebugShape {
  codeBlocks: Array<{ language: string; contentHash: string; codeLength: number }>;
  links: Array<{ refKind: string; refValue: string; resolved: boolean }>;
  nodeCounts: Record<string, number>;
}

interface RichEditorHarnessWindow {
  // The editor publishes its visual-debug snapshot here (MT-172 contract).
  __HS_EDITOR_DEBUG__?: EditorDebugShape;
  __RICH_EDITOR_HARNESS__?: {
    docJson: unknown;
    debug: EditorDebugShape | null;
    roundTrip:
      | null
      | {
          ok: boolean;
          schemaRehydrated: boolean;
          docEqual: boolean;
          rehydrateError: string | null;
          beforeHash: string;
          afterHash: string;
          beforeLanguage: string;
          afterLanguage: string;
          beforeCode: string;
          afterCode: string;
        };
    runRoundTrip?: () => void;
  };
}

test.describe("WP-KERNEL-009 integrated rich-editor offline round-trip (network blocked)", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "rich-editor.html")),
      "dist-harness missing — global setup should have built it (pnpm run build:harness)",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    await new Promise((resolve) => server.close(resolve));
  });

  test("editor + Monaco code block + wikilinks boot offline and round-trip with zero external requests", async ({ page }, testInfo) => {
    const externalRequests: string[] = [];
    const servedRequests: string[] = [];

    // Network kill-switch: anything not addressed to our loopback server is
    // aborted and recorded. A non-empty ledger fails the proof (MT-175).
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

    await page.goto(`${baseUrl}/harness/rich-editor.html`);

    // App shell renders — no blank screen (MT-174 spirit).
    await expect(page.getByTestId("rich-editor-harness-root")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();

    // The integrated toolbar + stable selectors are present (MT-172).
    await expect(page.getByTestId("rich-text-editor-toolbar")).toBeVisible();
    await expect(page.getByTestId("editor-cmd-format.bold")).toBeVisible();

    // The embedded Monaco code block mounted from bundled assets (MT-165/175).
    const codeBlock = page.getByTestId("monaco-code-block").first();
    await expect(codeBlock).toBeVisible();
    await expect.poll(async () => codeBlock.getAttribute("data-monaco-mounted")).toBe("true");
    // Monaco actually rendered its editor DOM into the code-block host.
    await expect(
      page.locator("[data-testid='monaco-code-block-host'] .monaco-editor").first(),
    ).toBeVisible();

    // The typed wikilinks rendered as hs-link nodes (MT-163).
    await expect(page.getByTestId("hs-link").first()).toBeVisible();
    const linkCount = await page.getByTestId("hs-link").count();
    expect(linkCount).toBeGreaterThanOrEqual(2);

    // The visual-debug payload reports the code block + links (MT-172). Read the
    // editor's own published global and poll until it lands (published via a
    // microtask after mount).
    await expect
      .poll(async () =>
        page.evaluate(
          () => (window as unknown as RichEditorHarnessWindow).__HS_EDITOR_DEBUG__?.codeBlocks?.length ?? 0,
        ),
      )
      .toBeGreaterThanOrEqual(1);
    const debug = await page.evaluate(
      () => (window as unknown as RichEditorHarnessWindow).__HS_EDITOR_DEBUG__,
    );
    expect(debug?.codeBlocks?.[0]?.language).toBe("typescript");
    expect(debug?.links?.length).toBeGreaterThanOrEqual(2);

    // ---- MT-176: deterministic round-trip of the embedded code block. ----
    // Iteration-3 H7: this is now a REAL re-hydration through the editor
    // schema (nodeFromJSON + check + structural compare), not a clone-vs-clone
    // tautology — the assertions below pin that contract.
    await page.getByTestId("harness-run-roundtrip").click();
    const roundTrip = await page.evaluate(
      () => (window as unknown as RichEditorHarnessWindow).__RICH_EDITOR_HARNESS__?.roundTrip,
    );
    expect(roundTrip, "round-trip result missing").toBeTruthy();
    expect(roundTrip?.ok, `round-trip mismatch: ${JSON.stringify(roundTrip)}`).toBe(true);
    expect(roundTrip?.schemaRehydrated, `schema re-hydration failed: ${roundTrip?.rehydrateError}`).toBe(true);
    expect(roundTrip?.docEqual, "re-hydrated doc structurally diverged from the serialized doc").toBe(true);
    expect(roundTrip?.rehydrateError).toBeNull();
    expect(roundTrip?.beforeLanguage).toBe("typescript");
    expect(roundTrip?.afterLanguage).toBe("typescript");
    expect(roundTrip?.beforeHash).toBe(roundTrip?.afterHash);
    expect(roundTrip?.beforeCode).toContain("export const offline = true;");

    // Worker chunks were really fetched from the loopback server (local boot).
    expect(
      servedRequests.some((url) => /worker-[^/]*\.js/.test(url)),
      `no worker chunk request seen; served: ${servedRequests.length} requests`,
    ).toBe(true);

    // THE offline guarantee: zero external requests attempted (MT-175).
    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);

    // Visual evidence for the validator.
    await page.screenshot({
      path: testInfo.outputPath("rich-editor-offline-roundtrip.png"),
      fullPage: true,
    });
  });
});
