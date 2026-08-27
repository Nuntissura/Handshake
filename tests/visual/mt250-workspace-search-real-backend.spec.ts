import { expect, test } from "./console_error_scan";

import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";

import { buildWorkspaceSearchHarness } from "./build_workspace_search_harness";

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
    <main data-testid="capture-root" style="padding:16px; width:1040px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

type FixtureDocument = {
  rich_document_id: string;
  title: string;
  initial_text: string;
};

type FixtureReady = {
  base_url: string;
  workspace_id: string;
  documents: FixtureDocument[];
};

type FixtureProof = {
  documents: Array<{
    rich_document_id: string;
    title: string;
    doc_version: number;
    text: string;
    event_count: number;
    save_events: Array<{ event_id: string; event_type: string; payload: { event?: string; doc_version?: number } }>;
  }>;
};

type FixtureHandle =
  | { kind: "skip"; reason: string }
  | {
      kind: "ready";
      child: ChildProcessWithoutNullStreams;
      ready: FixtureReady;
      stderr: () => string;
    };

function stablePart(value: string): string {
  const stable = value
    .trim()
    .replace(/[^a-zA-Z0-9_-]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stable || "item";
}

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
      "mt250_workspace_search_fixture",
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
        () => reject(new Error(`MT-250 fixture did not become ready within 600s. stderr:\n${stderr}`)),
        (teardownError) =>
          reject(
            new Error(
              `MT-250 fixture did not become ready within 600s. Startup teardown failed: ${String(teardownError)}. stderr:\n${stderr}`,
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
        if (line.startsWith("MT250_FIXTURE_SKIP ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({ kind: "skip", reason: line.slice("MT250_FIXTURE_SKIP ".length) });
          return;
        }
        if (line.startsWith("MT250_FIXTURE_READY ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({
            kind: "ready",
            child,
            ready: JSON.parse(line.slice("MT250_FIXTURE_READY ".length)) as FixtureReady,
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
          `MT-250 fixture exited before ready with code ${code} and signal ${signal}. stderr:\n${stderr}`,
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

async function fixtureProof(ready: FixtureReady): Promise<FixtureProof> {
  const docIds = ready.documents.map((doc) => doc.rich_document_id).join(",");
  const response = await fetch(`${ready.base_url}/mt250-fixture/proof?doc_ids=${encodeURIComponent(docIds)}`);
  if (!response.ok) {
    throw new Error(`fixture proof failed: ${response.status} ${await response.text()}`);
  }
  return (await response.json()) as FixtureProof;
}

test.describe("WP-KERNEL-009 MT-250 workspace search real backend", () => {
  test("replace-in-files previews, cancels, and applies across real SurrealDB documents with receipts", async ({
    page,
  }, testInfo) => {
    test.setTimeout(900_000);
    const { js, css } = await buildWorkspaceSearchHarness();
    let fixture: FixtureHandle | null = null;
    const externalRequests: string[] = [];
    const apiRequests: Array<{ originalUrl: string; rewrittenUrl: string; method: string }> = [];

    try {
      fixture = await startFixture();
      test.skip(fixture.kind === "skip", fixture.kind === "skip" ? fixture.reason : "");
      const ready = fixture.ready;
      expect(ready.documents).toHaveLength(2);
      const [openedDoc, unopenedDoc] = ready.documents;

      await page.route("**/*", async (route) => {
        const request = route.request();
        const url = request.url();
        if (url.startsWith(apiBaseUrl)) {
          const parsed = new URL(url);
          let pathname = parsed.pathname;
          if (pathname === "/workspaces/w1/loom/graph-search") {
            pathname = `/workspaces/${ready.workspace_id}/loom/graph-search`;
          }
          const rewrittenUrl = `${ready.base_url}${pathname}${parsed.search}`;
          apiRequests.push({ originalUrl: url, rewrittenUrl, method: request.method() });
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

      await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
      await page.addScriptTag({ content: js });
      await expect(page.getByTestId("workspace-search")).toBeVisible();

      await page.getByTestId("workspace-search.query").fill("ReplaceAlpha");
      await page.getByTestId("workspace-search.replace").fill("OmegaProof");
      await page.getByTestId("workspace-search.kind-filter").selectOption("document");
      await page.getByTestId("workspace-search.search").click();

      for (const doc of ready.documents) {
        await expect(page.getByTestId(`workspace-search.result.document.${stablePart(doc.rich_document_id)}`)).toContainText(
          doc.title,
        );
      }
      expect(
        apiRequests.some(
          (request) =>
            request.originalUrl.includes("/workspaces/w1/loom/graph-search") &&
            request.rewrittenUrl.includes(`/workspaces/${ready.workspace_id}/loom/graph-search`) &&
            request.rewrittenUrl.includes("source_kinds=document"),
        ),
      ).toBe(true);

      await page.getByTestId(`workspace-search.result.document.${stablePart(openedDoc.rich_document_id)}`).click();
      await expect
        .poll(async () => page.evaluate(() => window.__workspaceSearchOpenLog?.join("|") ?? ""))
        .toContain(`document:${openedDoc.rich_document_id}:ReplaceAlpha:false:false:false`);
      await expect
        .poll(async () => page.evaluate(() => window.__workspaceSearchOpenLog?.join("|") ?? ""))
        .not.toContain(unopenedDoc.rich_document_id);

      await page.getByTestId("workspace-search.preview-replace").click();
      await expect(page.getByTestId(`workspace-search.preview.${openedDoc.rich_document_id}`)).toContainText(
        "2 matches",
      );
      await expect(page.getByTestId(`workspace-search.preview.${unopenedDoc.rich_document_id}`)).toContainText(
        "1 matches",
      );
      await expect(page.getByTestId(`workspace-search.preview.${openedDoc.rich_document_id}`)).toContainText(
        "OmegaProof beta OmegaProof",
      );

      const cancelProofBefore = await fixtureProof(ready);
      expect(cancelProofBefore.documents.map((doc) => [doc.rich_document_id, doc.doc_version, doc.event_count])).toEqual(
        ready.documents.map((doc) => [doc.rich_document_id, 1, 0]),
      );

      await page.getByTestId("workspace-search.cancel-replace").click();
      await expect(page.getByTestId(`workspace-search.preview.${openedDoc.rich_document_id}`)).toHaveCount(0);
      const cancelProofAfter = await fixtureProof(ready);
      expect(cancelProofAfter).toEqual(cancelProofBefore);

      await page.getByTestId("workspace-search.preview-replace").click();
      await expect(page.getByTestId(`workspace-search.preview.${unopenedDoc.rich_document_id}`)).toBeVisible();
      await page.getByTestId("workspace-search.apply-replace").click();
      await expect(page.getByTestId("workspace-search.replace-status")).toContainText(
        "Applied 2 document replacement plan(s); receipts:",
      );

      const applyProof = await fixtureProof(ready);
      for (const proofDoc of applyProof.documents) {
        expect(proofDoc.doc_version).toBe(2);
        expect(proofDoc.text).toContain("OmegaProof");
        expect(proofDoc.text).not.toContain("ReplaceAlpha");
        expect(proofDoc.save_events).toHaveLength(1);
        expect(proofDoc.save_events[0].event_type).toBe("KNOWLEDGE_RICH_DOCUMENT_SAVED");
        expect(proofDoc.save_events[0].payload.event).toBe("saved");
        expect(proofDoc.save_events[0].payload.doc_version).toBe(2);
        await expect(page.getByTestId("workspace-search.replace-status")).toContainText(proofDoc.save_events[0].event_id);
      }

      await page.getByTestId("capture-root").screenshot({
        path: testInfo.outputPath("mt250-workspace-search-real-backend.png"),
      });
      expect(externalRequests).toEqual([]);
    } finally {
      await stopFixture(fixture);
    }
  });
});
