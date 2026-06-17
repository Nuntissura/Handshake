import { expect, test } from "./console_error_scan";

import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";

import { buildLoomCanvasHarness } from "./build_loom_canvas_harness";

const apiBaseUrl = "http://127.0.0.1:37501";
const repoRoot = path.resolve(__dirname, "..", "..");
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT ?? path.resolve(repoRoot, "..", "Handshake_Artifacts");
const cargoTargetDir = path.join(artifactRoot, "handshake-cargo-target");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; background: #f8fafc; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:0; width:1040px; height:640px;">
      <div id="harness-root" style="height:640px;"></div>
    </main>
  </body>
</html>`;

type SeedBlock = { block_id: string; title: string };

type FixtureReady = {
  base_url: string;
  workspace_id: string;
  canvas_block_id: string;
  blocks: SeedBlock[];
};

type FixtureProof = {
  canvas_content_type: string;
  placed_blocks_present: string[];
  semantic_edge_count: number;
  visual_edge_count: number;
  board_has_event_receipt: boolean;
};

type FixtureHandle =
  | { kind: "skip"; reason: string }
  | { kind: "ready"; child: ChildProcessWithoutNullStreams; ready: FixtureReady; stderr: () => string };

function startFixture(): Promise<FixtureHandle> {
  const child = spawn(
    "cargo",
    [
      "run",
      "--manifest-path",
      path.join(repoRoot, "src", "backend", "handshake_core", "Cargo.toml"),
      "--features",
      "runtime-full,duckdb-flight-recorder",
      "--target-dir",
      cargoTargetDir,
      "--bin",
      "mt261_loom_canvas_fixture",
    ],
    { cwd: repoRoot, env: { ...process.env, RUST_BACKTRACE: "1" }, windowsHide: true },
  );
  let stdoutBuffer = "";
  let stderr = "";
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error(`MT-261 fixture did not become ready within 600s. stderr:\n${stderr}`));
    }, 600_000);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString();
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() ?? "";
      for (const line of lines) {
        if (line.startsWith("MT261_FIXTURE_SKIP ")) {
          clearTimeout(timeout);
          resolve({ kind: "skip", reason: line.slice("MT261_FIXTURE_SKIP ".length) });
          return;
        }
        if (line.startsWith("MT261_FIXTURE_READY ")) {
          clearTimeout(timeout);
          resolve({
            kind: "ready",
            child,
            ready: JSON.parse(line.slice("MT261_FIXTURE_READY ".length)) as FixtureReady,
            stderr: () => stderr,
          });
          return;
        }
      }
    });
    child.once("error", (error) => {
      clearTimeout(timeout);
      reject(error);
    });
    child.once("exit", (code) => {
      if (code !== null && code !== 0) {
        clearTimeout(timeout);
        reject(new Error(`MT-261 fixture exited before ready with code ${code}. stderr:\n${stderr}`));
      }
    });
  });
}

async function stopFixture(handle: FixtureHandle | null): Promise<void> {
  if (!handle || handle.kind !== "ready") return;
  if (handle.child.exitCode !== null) return;
  handle.child.kill();
  await new Promise<void>((resolve) => {
    const timeout = setTimeout(() => {
      handle.child.kill("SIGKILL");
      resolve();
    }, 5_000);
    handle.child.once("exit", () => {
      clearTimeout(timeout);
      resolve();
    });
  });
}

async function fixtureProof(ready: FixtureReady): Promise<FixtureProof> {
  const response = await fetch(
    `${ready.base_url}/mt261-fixture/proof?canvas_block_id=${encodeURIComponent(ready.canvas_block_id)}`,
  );
  if (!response.ok) {
    throw new Error(`fixture proof failed: ${response.status} ${await response.text()}`);
  }
  return (await response.json()) as FixtureProof;
}

test.describe("WP-KERNEL-009 MT-261 loom canvas board real backend", () => {
  test("places blocks by reference, draws semantic vs visual edges, persists viewport, deletes placement keeping source", async ({
    page,
  }, testInfo) => {
    test.setTimeout(900_000);
    const { js, css } = await buildLoomCanvasHarness();
    let fixture: FixtureHandle | null = null;
    const externalRequests: string[] = [];

    try {
      fixture = await startFixture();
      test.skip(fixture.kind === "skip", fixture.kind === "skip" ? fixture.reason : "");
      const ready = fixture.ready;
      expect(ready.blocks).toHaveLength(2);
      const [roadmap, risk] = ready.blocks;

      await page.route("**/*", async (route) => {
        const request = route.request();
        const url = request.url();
        if (url.startsWith(apiBaseUrl)) {
          const parsed = new URL(url);
          const rewrittenUrl = `${ready.base_url}${parsed.pathname}${parsed.search}`;
          await route.continue({ url: rewrittenUrl });
          return;
        }
        if (!url.startsWith("about:") && !url.startsWith("data:")) {
          externalRequests.push(url);
          await route.abort("connectionfailed");
          return;
        }
        await route.continue();
      });

      const configScript = `window.__hsLoomCanvasConfig = ${JSON.stringify({
        workspaceId: ready.workspace_id,
        canvasBlockId: ready.canvas_block_id,
      })};`;

      await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
      await page.addScriptTag({ content: configScript });
      await page.addScriptTag({ content: js });
      await expect(page.getByTestId("loom-canvas")).toBeVisible({ timeout: 15_000 });

      // -- place two blocks (drag/drop) -----------------------------------
      // Simulate the HTML5 drop directly (drag source lives elsewhere); the
      // payload mime + drop handler are what we are proving.
      const dropBlock = async (blockId: string, clientX: number, clientY: number) => {
        await page.evaluate(
          async ({ blockId, clientX, clientY }) => {
            const surface = document.querySelector<HTMLElement>('[data-testid="loom-canvas.surface"]');
            if (!surface) throw new Error("no surface");
            const dt = new DataTransfer();
            dt.setData(
              "application/x-handshake-loom-block",
              JSON.stringify({ blockId, title: "drag" }),
            );
            surface.dispatchEvent(
              new DragEvent("drop", { bubbles: true, cancelable: true, clientX, clientY, dataTransfer: dt }),
            );
          },
          { blockId, clientX, clientY },
        );
      };

      await dropBlock(roadmap.block_id, 200, 200);
      await dropBlock(risk.block_id, 600, 200);

      // Two placements rendered as LIVE previews of the referenced blocks.
      await expect(page.locator('[data-testid^="loom-canvas.placement."][data-placed-block-id]')).toHaveCount(2);
      await expect(
        page.locator(`[data-placed-block-id="${roadmap.block_id}"] [data-testid$=".title"]`),
      ).toHaveText("Roadmap note");
      await expect(
        page.locator(`[data-placed-block-id="${risk.block_id}"] [data-testid$=".title"]`),
      ).toHaveText("Risk note");

      const placementIds = await page
        .locator('[data-testid^="loom-canvas.placement."][data-placed-block-id]')
        .evaluateAll((els) =>
          els.map((el) => el.getAttribute("data-testid")!.replace("loom-canvas.placement.", "")),
        );
      expect(placementIds).toHaveLength(2);
      const [pA, pB] = placementIds;

      // -- semantic edge (real loom edge) ---------------------------------
      await page.getByTestId(`loom-canvas.placement.${pA}`).click();
      await page.getByTestId("loom-canvas.start-edge").click();
      await page.getByTestId(`loom-canvas.placement.${pB}`).click();
      await expect(page.getByTestId("loom-canvas.status")).toContainText("real loom edge");

      // -- visual-only edge (board-local, NOT graph authority) ------------
      await page.getByTestId("loom-canvas.edge-mode").selectOption("visual");
      await page.getByTestId(`loom-canvas.placement.${pA}`).click();
      await page.getByTestId("loom-canvas.start-edge").click();
      await page.getByTestId(`loom-canvas.placement.${pB}`).click();
      await expect(page.getByTestId("loom-canvas.status")).toContainText("NOT graph authority");
      await expect(page.locator('[data-testid^="loom-canvas.visual-edge."]')).toHaveCount(1);

      // -- viewport persists + survives reload ----------------------------
      await page.getByTestId("loom-canvas.zoom-in").click();
      await expect(page.getByTestId("loom-canvas.zoom-value")).toHaveText("1.25x");

      // Reload (rebuild) and confirm the persisted viewport + placements come
      // back from the real backend.
      await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
      await page.addScriptTag({ content: configScript });
      await page.addScriptTag({ content: js });
      await expect(page.getByTestId("loom-canvas")).toBeVisible({ timeout: 15_000 });
      await expect(page.getByTestId("loom-canvas.zoom-value")).toHaveText("1.25x");
      await expect(page.locator('[data-testid^="loom-canvas.placement."][data-placed-block-id]')).toHaveCount(2);
      await expect(page.locator('[data-testid^="loom-canvas.visual-edge."]')).toHaveCount(1);

      // Backend proof BEFORE delete: canvas is a real canvas block, both placed
      // blocks present, exactly one semantic edge + one visual edge, board has
      // an EventLedger receipt.
      const proofBefore = await fixtureProof(ready);
      expect(proofBefore.canvas_content_type).toBe("canvas");
      expect(proofBefore.placed_blocks_present.sort()).toEqual([roadmap.block_id, risk.block_id].sort());
      expect(proofBefore.semantic_edge_count).toBe(1);
      expect(proofBefore.visual_edge_count).toBe(1);
      expect(proofBefore.board_has_event_receipt).toBe(true);

      // -- delete a placement; the source block must survive --------------
      const firstPlaced = await page
        .locator('[data-testid^="loom-canvas.placement."][data-placed-block-id]')
        .first()
        .getAttribute("data-placed-block-id");
      const firstPlacementId = await page
        .locator('[data-testid^="loom-canvas.placement."][data-placed-block-id]')
        .first()
        .evaluate((el) => el.getAttribute("data-testid")!.replace("loom-canvas.placement.", ""));
      await page.getByTestId(`loom-canvas.placement.${firstPlacementId}.remove`).click();
      await expect(page.locator('[data-testid^="loom-canvas.placement."][data-placed-block-id]')).toHaveCount(1);

      const proofAfter = await fixtureProof(ready);
      // The placement is gone (one fewer placed block on the canvas) but the
      // SOURCE block itself still exists in loom_blocks (reference-not-copy).
      expect(proofAfter.placed_blocks_present).toHaveLength(1);
      expect(proofAfter.placed_blocks_present).not.toContain(firstPlaced);
      const blockStillExists = await fetch(
        `${ready.base_url}/workspaces/${ready.workspace_id}/loom/blocks/${firstPlaced}`,
      );
      expect(blockStillExists.ok).toBe(true);

      await page.getByTestId("capture-root").screenshot({
        path: testInfo.outputPath("mt261-loom-canvas-real-backend.png"),
      });
      expect(externalRequests).toEqual([]);
    } finally {
      await stopFixture(fixture);
    }
  });
});
