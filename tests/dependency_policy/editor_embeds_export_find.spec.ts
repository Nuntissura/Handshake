// WP-KERNEL-009 / MT-244 — embeds + save-to-format + find/replace offline proofs.
//
// Serves the BUILT harness (app/dist-harness/harness/rich-editor-embeds.html)
// from a loopback-only server that ALSO implements the REAL Handshake asset
// route shapes (/workspaces/:ws/assets/:id and …/content) with REAL bytes:
//   - a pngjs-encoded PNG (decodes in the browser: naturalWidth proof),
//   - a WebM RECORDED BY THE BROWSER ITSELF (canvas captureStream +
//     MediaRecorder) so <video> playback is proven against a genuine media
//     file — no binary fixtures committed, no mock data.
//
// Proves with the network kill-switch active (zero external requests):
//   1. [[HS_images:…]] renders a real <img> with decoded pixels,
//      [[HS_slideshow:a,b]] renders the sequenced viewer and navigates,
//      [[video:…]] renders a real <video> that PLAYS (currentTime advances),
//      an unresolvable embed renders the TYPED visible error state
//      (role=alert + data-error-kind) — never blank,
//   2. document-wide find/replace counts matches across prose AND the
//      embedded Monaco code block and replace-all rewrites both,
//   3. save-to-format: the self-contained HTML export downloads through the
//      real UI, contains base64 data: URLs (image inlined, video reference-
//      linked by the documented guard), and OPENS STANDALONE from file://
//      with zero network requests (img decodes from the data: URL).

import { expect, test, type Page } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { createReadStream, existsSync, statSync, readFileSync } from "node:fs";
import type { AddressInfo } from "node:net";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { PNG } from "pngjs";

// CJS context (Playwright transpiles spec files to CJS; import.meta unavailable).
const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");

const WORKSPACE = "ws-embed-proof";

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

interface ServedAsset {
  metadata: Record<string, unknown>;
  bytes: Buffer;
}

/** Real decodable 8x8 red/blue checker PNG, encoded by pngjs (no fixtures). */
function buildRealPng(): Buffer {
  const png = new PNG({ width: 8, height: 8 });
  for (let y = 0; y < png.height; y++) {
    for (let x = 0; x < png.width; x++) {
      const idx = (png.width * y + x) << 2;
      const red = (x + y) % 2 === 0;
      png.data[idx] = red ? 220 : 30;
      png.data[idx + 1] = 40;
      png.data[idx + 2] = red ? 60 : 200;
      png.data[idx + 3] = 255;
    }
  }
  return PNG.sync.write(png);
}

function assetMetadata(assetId: string, mime: string, bytes: Buffer): Record<string, unknown> {
  return {
    asset_id: assetId,
    workspace_id: WORKSPACE,
    kind: mime.startsWith("video/") ? "video" : "image",
    mime,
    original_filename: `${assetId}.${mime.split("/")[1] ?? "bin"}`,
    content_hash: `hash-${assetId}`,
    size_bytes: bytes.byteLength,
    width: 8,
    height: 8,
    created_at: "2026-06-12T00:00:00Z",
    classification: "proof",
    exportable: true,
    is_proxy_of: null,
    proxy_asset_id: null,
  };
}

/**
 * Loopback server: serves the built harness statically AND the real
 * Handshake asset route shapes for the proof workspace.
 */
function serveHarnessWithAssets(assets: Map<string, ServedAsset>): Promise<Server> {
  const server = createServer((req, res) => {
    const urlPath = decodeURIComponent((req.url ?? "/").split("?")[0]);

    const assetMatch = new RegExp(`^/workspaces/${WORKSPACE}/assets/([^/]+)(/content)?$`).exec(urlPath);
    if (assetMatch) {
      const asset = assets.get(assetMatch[1]);
      if (!asset) {
        res.writeHead(404, { "content-type": "application/json" });
        res.end(JSON.stringify({ error: "asset_not_found" }));
        return;
      }
      if (assetMatch[2]) {
        res.writeHead(200, { "content-type": String(asset.metadata.mime) });
        res.end(asset.bytes);
      } else {
        res.writeHead(200, { "content-type": "application/json" });
        res.end(JSON.stringify(asset.metadata));
      }
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

/**
 * Records a REAL WebM in the live Chromium (animated canvas captureStream →
 * MediaRecorder) and returns its bytes. The proof video is produced by the
 * same browser that must later play it back — a genuine end-to-end media
 * artifact with no committed binary and no mock.
 */
async function recordRealWebm(page: Page): Promise<Buffer> {
  // Retry the in-browser recording: a freshly restarted worker occasionally
  // yields an empty first MediaRecorder pass before the encoder warms up.
  for (let attempt = 0; attempt < 3; attempt++) {
    const base64 = await page.evaluate(async () => {
      const canvas = document.createElement("canvas");
      canvas.width = 64;
      canvas.height = 64;
      document.body.appendChild(canvas);
      const ctx = canvas.getContext("2d")!;
      // Paint before capture so the stream has a real first frame.
      ctx.fillStyle = "#cc2244";
      ctx.fillRect(0, 0, 64, 64);
      const stream = canvas.captureStream(30);
      const recorder = new MediaRecorder(stream, { mimeType: "video/webm" });
      const chunks: Blob[] = [];
      recorder.ondataavailable = (event) => {
        if (event.data.size > 0) chunks.push(event.data);
      };
      const stopped = new Promise<void>((resolve) => {
        recorder.onstop = () => resolve();
      });
      let frame = 0;
      const animate = window.setInterval(() => {
        frame += 1;
        ctx.fillStyle = frame % 2 === 0 ? "#cc2244" : "#2244cc";
        ctx.fillRect(0, 0, 64, 64);
        ctx.fillStyle = "#ffffff";
        ctx.fillRect((frame * 4) % 64, 24, 12, 12);
      }, 33);
      recorder.start(100);
      await new Promise((resolve) => setTimeout(resolve, 900));
      recorder.requestData();
      await new Promise((resolve) => setTimeout(resolve, 120));
      window.clearInterval(animate);
      recorder.stop();
      await stopped;
      canvas.remove();
      const blob = new Blob(chunks, { type: "video/webm" });
      const buffer = await blob.arrayBuffer();
      let binary = "";
      const view = new Uint8Array(buffer);
      for (let i = 0; i < view.length; i++) binary += String.fromCharCode(view[i]);
      return btoa(binary);
    });
    const bytes = Buffer.from(base64, "base64");
    if (bytes.byteLength > 500) return bytes;
  }
  throw new Error("MediaRecorder produced no usable WebM after 3 attempts");
}

function installKillSwitch(page: Page, baseUrl: string, externalRequests: string[]): Promise<void> {
  return page.route("**/*", async (route) => {
    const url = route.request().url();
    if (url.startsWith(baseUrl)) {
      await route.continue();
      return;
    }
    externalRequests.push(url);
    await route.abort("connectionfailed");
  });
}

test.describe("WP-KERNEL-009 MT-244 embeds + export + find/replace (offline, network blocked)", () => {
  let server: Server;
  let baseUrl: string;
  const assets = new Map<string, ServedAsset>();

  test.beforeAll(async ({ browser }) => {
    expect(
      existsSync(path.join(distHarness, "harness", "rich-editor-embeds.html")),
      "dist-harness missing rich-editor-embeds.html — global setup builds it (pnpm run build:harness)",
    ).toBe(true);

    const pngBytes = buildRealPng();
    assets.set("img-ok", { metadata: assetMetadata("img-ok", "image/png", pngBytes), bytes: pngBytes });
    assets.set("s1", { metadata: assetMetadata("s1", "image/png", pngBytes), bytes: pngBytes });
    assets.set("s2", { metadata: assetMetadata("s2", "image/png", pngBytes), bytes: pngBytes });

    // Record the REAL WebM before the harness runs (its own blank page).
    const recordingPage = await browser.newPage();
    const webmBytes = await recordRealWebm(recordingPage);
    await recordingPage.close();
    expect(webmBytes.byteLength, "MediaRecorder produced an empty WebM").toBeGreaterThan(500);
    assets.set("vid-ok", { metadata: assetMetadata("vid-ok", "video/webm", webmBytes), bytes: webmBytes });

    server = await serveHarnessWithAssets(assets);
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    if (server) await new Promise((resolve) => server.close(resolve));
  });

  test("embed NodeViews render real assets (img/video/slideshow) and fail closed on a broken ref", async ({ page }, testInfo) => {
    const externalRequests: string[] = [];
    await installKillSwitch(page, baseUrl, externalRequests);
    await page.goto(`${baseUrl}/harness/rich-editor-embeds.html`);

    await expect(page.getByTestId("rich-editor-embeds-harness-root")).toBeVisible();
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();

    // 1) Picture embed: a real <img> with DECODED pixels from the asset bytes.
    const img = page.getByTestId("hs-embed-image");
    await expect(img).toBeVisible();
    await expect(img).toHaveAttribute("data-asset-id", "img-ok");
    await expect
      .poll(async () => img.evaluate((el) => (el as HTMLImageElement).naturalWidth))
      .toBe(8);

    // 2) Video embed: a real <video> that actually PLAYS the recorded WebM.
    const video = page.getByTestId("hs-embed-video");
    await expect(video).toBeVisible();
    await expect(video).toHaveAttribute("data-asset-mime", "video/webm");
    const playback = await video.evaluate(async (el) => {
      const media = el as HTMLVideoElement;
      media.muted = true;
      await media.play();
      await new Promise((resolve) => setTimeout(resolve, 400));
      return {
        currentTime: media.currentTime,
        videoWidth: media.videoWidth,
        readyState: media.readyState,
        error: media.error?.code ?? null,
      };
    });
    expect(playback.error, `video element error ${JSON.stringify(playback)}`).toBeNull();
    expect(playback.currentTime, "playback did not advance").toBeGreaterThan(0);
    expect(playback.videoWidth).toBe(64);

    // 3) Slideshow: sequenced viewer over two real images with navigation.
    const sequence = page.getByTestId("hs-embed-sequence");
    await expect(sequence).toBeVisible();
    await expect(sequence).toHaveAttribute("data-sequence-length", "2");
    await expect(sequence).toHaveAttribute("data-embed-kind", "slideshow");
    await expect(page.getByTestId("hs-embed-sequence-position")).toHaveText("1/2");
    await page.getByTestId("hs-embed-sequence-next").click();
    await expect(page.getByTestId("hs-embed-sequence-position")).toHaveText("2/2");
    await expect(page.getByTestId("hs-embed-sequence-image")).toHaveAttribute("data-asset-id", "s2");

    // 4) Unresolvable embed: TYPED visible error state — never blank.
    const error = page.getByTestId("hs-embed-error");
    await expect(error).toBeVisible();
    await expect(error).toHaveAttribute("role", "alert");
    await expect(error).toHaveAttribute("data-error-kind", "not_found");
    await expect(error).toContainText("missing-asset");

    // 5) Non-media chip unchanged.
    await expect(page.locator("[data-testid='hs-link'][data-ref-kind='wp']")).toBeVisible();

    // Zero external requests (MT-175 lane guarantee).
    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);

    await page.screenshot({ path: testInfo.outputPath("mt244-embeds-rendered.png"), fullPage: true });
  });

  test("document-wide find/replace spans prose + Monaco code block in the real UI", async ({ page }, testInfo) => {
    const externalRequests: string[] = [];
    await installKillSwitch(page, baseUrl, externalRequests);
    await page.goto(`${baseUrl}/harness/rich-editor-embeds.html`);
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();

    await page.getByTestId("editor-open-find").click();
    await expect(page.getByTestId("find-panel")).toBeVisible();
    await page.getByTestId("find-input").fill("alpha");
    // prose "alpha beta alpha" → 2, code block "const alpha = 'alpha';" → 2.
    await expect(page.getByTestId("find-panel")).toHaveAttribute("data-match-count", "4");

    // Highlights are visible in the document.
    await expect.poll(async () => page.locator(".hs-find-match").count()).toBeGreaterThanOrEqual(2);
    await page.getByTestId("find-next").click();
    await expect
      .poll(async () =>
        page.evaluate(
          () =>
            document.querySelectorAll(".hs-find-match--current, .hs-find-match-block--current").length,
        ),
      )
      .toBeGreaterThanOrEqual(1);

    // Switch to replace mode via Ctrl+H inside the editor surface.
    await page.locator("[data-testid='rich-text-editor-surface'] [contenteditable]").first().click();
    await page.keyboard.press("ControlOrMeta+h");
    await expect(page.getByTestId("replace-input")).toBeVisible();
    await page.getByTestId("find-input").fill("alpha");
    await page.getByTestId("replace-input").fill("omega");
    await page.getByTestId("replace-all").click();

    await expect(page.getByTestId("find-outcome")).toContainText("Replaced 4 matches");
    await expect(page.getByTestId("find-outcome")).toContainText("2 in code blocks");
    // The prose now shows omega…
    await expect(page.getByTestId("rich-text-editor-surface")).toContainText("omega beta omega");
    // …and the Monaco code block model carries the replaced code.
    await expect
      .poll(async () =>
        page.evaluate(() => {
          const debug = (window as unknown as Record<string, unknown>).__HS_EDITOR_DEBUG__ as
            | { codeBlocks: Array<{ codeLength: number }> }
            | undefined;
          return debug?.codeBlocks?.[0]?.codeLength ?? -1;
        }),
      )
      .toBe("const omega = 'omega';".length);

    expect(externalRequests).toEqual([]);
    await page.screenshot({ path: testInfo.outputPath("mt244-find-replace.png"), fullPage: true });
  });

  test("self-contained HTML export downloads, inlines the image, and opens standalone from file:// with zero network", async ({ page, context }, testInfo) => {
    const externalRequests: string[] = [];
    await installKillSwitch(page, baseUrl, externalRequests);
    await page.goto(`${baseUrl}/harness/rich-editor-embeds.html`);
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();
    // Wait for embeds to resolve so the export sees resolvable assets.
    await expect(page.getByTestId("hs-embed-image")).toBeVisible();

    // Export through the REAL UI: toolbar menu → self-contained → download.
    await page.getByTestId("editor-open-export").click();
    await expect(page.getByTestId("editor-export-menu")).toBeVisible();
    const downloadPromise = page.waitForEvent("download");
    await page.getByTestId("export-format-html_self_contained").click();
    const download = await downloadPromise;
    const suggested = download.suggestedFilename();
    expect(suggested).toMatch(/^embed-proof-document-self-contained-.*\.html$/);
    expect(suggested).not.toMatch(/\s/);

    const exportPath = testInfo.outputPath(suggested);
    await download.saveAs(exportPath);
    const exported = readFileSync(exportPath, "utf8");

    // Typed export semantics + the documented media guards.
    expect(exported).toContain('data-hs-export="rich_document"');
    expect(exported).toContain('data-hs-export-mode="self_contained"');
    expect(exported).toContain("data:image/png;base64,");
    // Video stays reference-linked (documented size guard) + visible notice.
    expect(exported).toContain('data-hs-inline-skipped="video_size_guard"');
    // The unresolvable embed exported fail-closed with its typed marker.
    expect(exported).toContain('data-hs-export-error="not_found"');
    // Inlined image must NOT reference the asset server.
    const imgSrcs = [...exported.matchAll(/<img[^>]+src="([^"]+)"/g)].map((m) => m[1]);
    const inlined = imgSrcs.filter((src) => src.startsWith("data:image/png;base64,"));
    expect(inlined.length).toBeGreaterThanOrEqual(3); // img-ok + s1 + s2
    const exportStatus = page.getByTestId("export-status");
    await expect(exportStatus).toBeVisible();
    expect(externalRequests).toEqual([]);

    // STANDALONE-OPEN PROOF: a fresh page, file:// URL, count every network
    // request — the self-contained file must need NONE.
    const standalone = await context.newPage();
    const networkRequests: string[] = [];
    standalone.on("request", (request) => {
      if (!request.url().startsWith("file://")) networkRequests.push(request.url());
    });
    await standalone.goto(pathToFileURL(exportPath).href);
    const standaloneImg = standalone.locator("span[data-hs-ref-kind='images'] img").first();
    await expect(standaloneImg).toBeVisible();
    await expect
      .poll(async () => standaloneImg.evaluate((el) => (el as HTMLImageElement).naturalWidth))
      .toBe(8);
    // Typed semantics survive into the standalone file.
    expect(await standalone.locator("span[data-hs-ref-kind='wp']").count()).toBe(1);
    expect(await standalone.locator("pre[data-hs-node='monacoCodeBlock']").count()).toBe(1);
    expect(networkRequests, `standalone file made network requests: ${networkRequests.join(", ")}`).toEqual([]);

    await standalone.screenshot({ path: testInfo.outputPath("mt244-export-standalone.png"), fullPage: true });
    await standalone.close();
  });

  test("HTML export round-trips through importHtml to the identical document (in-harness)", async ({ page }) => {
    const externalRequests: string[] = [];
    await installKillSwitch(page, baseUrl, externalRequests);
    await page.goto(`${baseUrl}/harness/rich-editor-embeds.html`);
    await expect(page.getByTestId("rich-text-editor")).toBeVisible();
    await expect(page.getByTestId("hs-embed-image")).toBeVisible();

    // Export reference-linked via the palette (second UI path), then compare
    // the published export content's structural semantics.
    await page.getByTestId("editor-open-palette").click();
    await page.getByTestId("editor-command-palette-input").fill("export");
    const downloadPromise = page.waitForEvent("download");
    await page.getByTestId("palette-cmd-export.html_reference_linked").click();
    await downloadPromise;

    const published = await page.evaluate(() => {
      const result = (window as unknown as Record<string, unknown>).__HS_EDITOR_LAST_EXPORT__ as
        | { content: string; formatId: string }
        | undefined;
      return result ?? null;
    });
    expect(published?.formatId).toBe("html_reference_linked");
    const content = published?.content ?? "";
    expect(content).toContain('data-hs-node="hsLink"');
    expect(content).toContain('data-hs-ref-kind="slideshow"');
    expect(content).toContain('data-hs-node="monacoCodeBlock"');
    expect(content).toContain('data-hs-language="typescript"');
    // Reference-linked: image points at the workspace asset content route.
    expect(content).toContain(`/workspaces/${WORKSPACE}/assets/img-ok/content`);
    expect(externalRequests).toEqual([]);
  });
});
