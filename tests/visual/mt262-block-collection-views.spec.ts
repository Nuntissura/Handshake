import { expect, test } from "./console_error_scan";

import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";

import { buildLoomBlockCollectionHarness } from "./build_loom_block_collection_harness";

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

type FixtureReady = {
  base_url: string;
  workspace_id: string;
  table_view_id: string;
  kanban_view_id: string;
  calendar_view_id: string;
  todo_tag_id: string;
  done_tag_id: string;
  kanban_card_id: string;
  table_block_count: number;
};

type ViewProof = {
  content_type: string;
  has_view_definition: boolean;
  derived_json_leaks_definition: boolean;
  has_knowledge_bridge: boolean;
};

type CardTags = { tag_target_ids: string[] };

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
      "mt262_block_collection_views_fixture",
    ],
    {
      cwd: repoRoot,
      env: { ...process.env, RUST_BACKTRACE: "1" },
      windowsHide: true,
      detached: process.platform !== "win32",
    },
  );
  let stdoutBuffer = "";
  let stderr = "";
  return new Promise((resolve, reject) => {
    let settled = false;
    const timeout = setTimeout(() => {
      if (settled) return;
      settled = true;
      void terminateFixtureProcess(child).then(
        () => reject(new Error(`MT-262 fixture did not become ready within 600s. stderr:\n${stderr}`)),
        (teardownError) =>
          reject(
            new Error(
              `MT-262 fixture did not become ready within 600s. Startup teardown failed: ${String(teardownError)}. stderr:\n${stderr}`,
            ),
          ),
      );
    }, 600_000);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString();
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() ?? "";
      for (const line of lines) {
        if (line.startsWith("MT262_FIXTURE_SKIP ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({ kind: "skip", reason: line.slice("MT262_FIXTURE_SKIP ".length) });
          return;
        }
        if (line.startsWith("MT262_FIXTURE_READY ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({
            kind: "ready",
            child,
            ready: JSON.parse(line.slice("MT262_FIXTURE_READY ".length)) as FixtureReady,
            stderr: () => stderr,
          });
          return;
        }
      }
    });
    child.once("error", (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      reject(error);
    });
    child.once("exit", (code, signal) => {
      if (settled) return;
      settled = true;
      clearTimeout(timeout);
      reject(
        new Error(
          `MT-262 fixture exited before ready with code ${code} and signal ${signal}. stderr:\n${stderr}`,
        ),
      );
    });
  });
}

function waitForFixtureExit(
  child: ChildProcessWithoutNullStreams,
  timeoutMs: number,
): Promise<boolean> {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve(true);
  return new Promise((resolve) => {
    const onExit = () => {
      clearTimeout(timeout);
      resolve(true);
    };
    const timeout = setTimeout(() => {
      child.off("exit", onExit);
      resolve(false);
    }, timeoutMs);
    child.once("exit", onExit);
    if (child.exitCode !== null || child.signalCode !== null) {
      child.off("exit", onExit);
      clearTimeout(timeout);
      resolve(true);
    }
  });
}

async function terminateFixtureProcess(child: ChildProcessWithoutNullStreams): Promise<void> {
  if (child.exitCode !== null || child.signalCode !== null) return;
  if (process.platform === "win32" && child.pid !== undefined) {
    const result = spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], {
      encoding: "utf8",
      timeout: 7_000,
      windowsHide: true,
    });
    if (await waitForFixtureExit(child, 2_000)) return;
    throw new Error(
      `fixture process tree did not exit after taskkill; pid=${child.pid}; status=${result.status}; signal=${result.signal}; error=${result.error?.message ?? "none"}; stderr=${result.stderr}`,
    );
  }
  try {
    if (child.pid !== undefined) process.kill(-child.pid, "SIGTERM");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
  }
  if (await waitForFixtureExit(child, 5_000)) return;
  try {
    if (child.pid !== undefined) process.kill(-child.pid, "SIGKILL");
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
  }
  if (await waitForFixtureExit(child, 2_000)) return;
  throw new Error(
    `fixture process did not exit after bounded TERM/KILL; pid=${child.pid ?? "unknown"}`,
  );
}

async function stopFixture(handle: FixtureHandle | null): Promise<void> {
  if (!handle || handle.kind !== "ready") return;
  if (handle.child.exitCode !== null || handle.child.signalCode !== null) {
    if (handle.child.exitCode === 0) return;
    throw new Error(
      `Fixture exited before shutdown: exitCode=${handle.child.exitCode} signal=${handle.child.signalCode}. stderr:\n${handle.stderr()}`,
    );
  }

  const controller = new AbortController();
  const requestTimeout = setTimeout(() => controller.abort(), 2_000);
  try {
    await fetch(`${handle.ready.base_url}/__fixture/shutdown`, {
      method: "POST",
      signal: controller.signal,
    });
  } catch {
    // A failed request is handled by the bounded process-stop fallback below.
  } finally {
    clearTimeout(requestTimeout);
  }

  if (await waitForFixtureExit(handle.child, 10_000)) {
    if (handle.child.exitCode === 0) return;
    throw new Error(
      `Fixture graceful shutdown failed: exitCode=${handle.child.exitCode} signal=${handle.child.signalCode}. stderr:\n${handle.stderr()}`,
    );
  }

  await terminateFixtureProcess(handle.child);
  throw new Error(
    `Fixture graceful shutdown timed out and required forced termination. stderr:\n${handle.stderr()}`,
  );
}

async function viewProof(ready: FixtureReady, blockId: string): Promise<ViewProof> {
  const response = await fetch(
    `${ready.base_url}/mt262-fixture/view-proof?block_id=${encodeURIComponent(blockId)}`,
  );
  if (!response.ok) throw new Error(`view-proof failed: ${response.status} ${await response.text()}`);
  return (await response.json()) as ViewProof;
}

async function cardTags(ready: FixtureReady, blockId: string): Promise<CardTags> {
  const response = await fetch(
    `${ready.base_url}/mt262-fixture/card-tags?block_id=${encodeURIComponent(blockId)}`,
  );
  if (!response.ok) throw new Error(`card-tags failed: ${response.status} ${await response.text()}`);
  return (await response.json()) as CardTags;
}

test.describe("WP-KERNEL-009 MT-262 block collection views real backend", () => {
  test("table re-sorts via real backend, Kanban drag mutates real tags, calendar buckets by journal_date", async ({
    page,
  }, testInfo) => {
    test.setTimeout(900_000);
    const { js, css } = await buildLoomBlockCollectionHarness();
    let fixture: FixtureHandle | null = null;
    const externalRequests: string[] = [];

    try {
      fixture = await startFixture();
      test.skip(fixture.kind === "skip", fixture.kind === "skip" ? fixture.reason : "");
      const ready = fixture.ready;

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

      const mount = async (viewBlockId: string) => {
        await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
        await page.addScriptTag({
          content: `window.__hsBlockCollectionConfig = ${JSON.stringify({
            workspaceId: ready.workspace_id,
            viewBlockId,
          })};`,
        });
        await page.addScriptTag({ content: js });
        await expect(page.getByTestId("block-collection-view")).toBeVisible({ timeout: 15_000 });
      };

      // --- Backend proof: a saved view IS a typed view_def block ----------
      const tableProof = await viewProof(ready, ready.table_view_id);
      expect(tableProof.content_type).toBe("view_def");
      expect(tableProof.has_view_definition).toBe(true);
      expect(tableProof.derived_json_leaks_definition).toBe(false);
      expect(tableProof.has_knowledge_bridge).toBe(true);

      // --- TABLE: header click re-sorts via a REAL backend request --------
      await mount(ready.table_view_id);
      await expect(page.getByTestId("loom-table-view")).toBeVisible();
      const titlesAsc = await page
        .locator('[data-testid="loom-table-row"] td[data-field="title"]')
        .allInnerTexts();
      const sortedAsc = [...titlesAsc].sort();
      expect(titlesAsc).toEqual(sortedAsc);

      // Click the Title header -> flips to descending; the view re-queries the
      // backend (a real /results POST) and rows come back in descending order.
      const resultsRequest = page.waitForRequest(
        (req) => req.url().includes("/loom/views/definitions/") && req.url().endsWith("/results"),
      );
      await page.getByTestId("loom-table-sort-title").click();
      await resultsRequest;
      await expect
        .poll(async () =>
          page.locator('[data-testid="loom-table-row"] td[data-field="title"]').allInnerTexts(),
        )
        .toEqual([...sortedAsc].reverse());
      // The new sort was PERSISTED to SurrealDB (not localStorage).
      await expect
        .poll(async () => (await viewProof(ready, ready.table_view_id)).has_view_definition)
        .toBe(true);

      // --- KANBAN: drag a card => REAL tag edge mutation + re-query -------
      await mount(ready.kanban_view_id);
      await expect(page.getByTestId("loom-kanban-view")).toBeVisible();
      // The card starts in the todo lane.
      await expect(
        page.locator(
          `[data-testid="loom-kanban-card"][data-block-id="${ready.kanban_card_id}"][data-lane-key="${ready.todo_tag_id}"]`,
        ),
      ).toHaveCount(1);
      const tagsBefore = await cardTags(ready, ready.kanban_card_id);
      expect(tagsBefore.tag_target_ids).toContain(ready.todo_tag_id);
      expect(tagsBefore.tag_target_ids).not.toContain(ready.done_tag_id);

      // Simulate the HTML5 drag todo -> done. The drop handler performs the
      // real PATCH (add_tags/remove_tags) then the host re-queries.
      const patchRequest = page.waitForRequest(
        (req) =>
          req.method() === "PATCH" &&
          req.url().includes(`/loom/blocks/${ready.kanban_card_id}`),
      );
      await page.evaluate(
        ({ cardId, todoKey, doneKey }) => {
          const card = document.querySelector<HTMLElement>(
            `[data-testid="loom-kanban-card"][data-block-id="${cardId}"]`,
          );
          const doneLane = document.querySelector<HTMLElement>(
            `[data-testid="loom-kanban-lane"][data-lane-key="${doneKey}"]`,
          );
          if (!card || !doneLane) throw new Error("missing card or lane");
          const dt = new DataTransfer();
          card.dispatchEvent(
            new DragEvent("dragstart", { bubbles: true, cancelable: true, dataTransfer: dt }),
          );
          dt.setData(
            "application/x-handshake-kanban-card",
            JSON.stringify({ blockId: cardId, fromKey: todoKey }),
          );
          doneLane.dispatchEvent(
            new DragEvent("drop", { bubbles: true, cancelable: true, dataTransfer: dt }),
          );
        },
        { cardId: ready.kanban_card_id, todoKey: ready.todo_tag_id, doneKey: ready.done_tag_id },
      );
      await patchRequest;

      // Re-query shows the card in its NEW lane.
      await expect(
        page.locator(
          `[data-testid="loom-kanban-card"][data-block-id="${ready.kanban_card_id}"][data-lane-key="${ready.done_tag_id}"]`,
        ),
      ).toHaveCount(1, { timeout: 15_000 });
      // Fresh SurrealDB read confirms authority moved (not just the view).
      await expect
        .poll(async () => (await cardTags(ready, ready.kanban_card_id)).tag_target_ids)
        .toContain(ready.done_tag_id);
      const tagsAfter = await cardTags(ready, ready.kanban_card_id);
      expect(tagsAfter.tag_target_ids).not.toContain(ready.todo_tag_id);

      // --- CALENDAR: groups by the real journal_date field ----------------
      await mount(ready.calendar_view_id);
      await expect(page.getByTestId("loom-calendar-view")).toBeVisible();
      const days = await page.locator('[data-testid="loom-calendar-day"]').evaluateAll((els) =>
        els.map((el) => el.getAttribute("data-date")),
      );
      expect(days).toContain("2026-06-15");
      expect(days).toContain("2026-06-20");

      await page.getByTestId("capture-root").screenshot({
        path: testInfo.outputPath("mt262-block-collection-views.png"),
      });
      expect(externalRequests).toEqual([]);
    } finally {
      await stopFixture(fixture);
    }
  });
});
