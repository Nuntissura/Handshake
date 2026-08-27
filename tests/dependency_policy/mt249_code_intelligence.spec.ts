// WP-KERNEL-009 / MT-249 — Monaco code-intelligence runtime proof.
//
// Starts a Rust fixture server that seeds a real CodeIndexEngine fixture into
// isolated embedded SurrealDB, then drives the built Monaco harness against the real
// /knowledge/code/* and /api/diagnostics routes.

import { expect, test } from "@playwright/test";
import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from "node:child_process";
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const distHarness = path.join(appDir, "dist-harness");
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT ??
  path.resolve(repoRoot, "..", "Handshake_Artifacts");
const cargoTargetDir = path.join(artifactRoot, "handshake-cargo-target");

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

type FixtureReady = {
  base_url: string;
  workspace_id: string;
  symbol_entity_id: string;
  content_hash: string;
  parser_version: string;
};

type FixtureHandle =
  | { kind: "skip"; reason: string }
  | { kind: "ready"; child: ChildProcessWithoutNullStreams; ready: FixtureReady; stderr: () => string };

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
      "mt249_code_intelligence_fixture",
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
        () => reject(new Error(`MT-249 fixture did not become ready within 300s. stderr:\n${stderr}`)),
        (teardownError) =>
          reject(
            new Error(
              `MT-249 fixture did not become ready within 300s. Startup teardown failed: ${String(teardownError)}. stderr:\n${stderr}`,
            ),
          ),
      );
    }, 300_000);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString();
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() ?? "";
      for (const line of lines) {
        if (line.startsWith("MT249_FIXTURE_SKIP ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({ kind: "skip", reason: line.slice("MT249_FIXTURE_SKIP ".length) });
          return;
        }
        if (line.startsWith("MT249_FIXTURE_READY ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({
            kind: "ready",
            child,
            ready: JSON.parse(line.slice("MT249_FIXTURE_READY ".length)) as FixtureReady,
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
          `MT-249 fixture exited before ready with code ${code} and signal ${signal}. stderr:\n${stderr}`,
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

async function closeServer(server: Server | null): Promise<void> {
  if (!server) return;
  await new Promise((resolve) => server.close(resolve));
}

test.describe("WP-KERNEL-009 MT-249 Monaco code intelligence", () => {
  test("providers consume real backend code routes and surface stale diagnostics", async ({ page }, testInfo) => {
    test.setTimeout(420_000);
    expect(
      existsSync(path.join(distHarness, "harness", "mt249-code-intelligence.html")),
      "dist-harness missing mt249-code-intelligence.html; global setup should have built it",
    ).toBe(true);

    let server: Server | null = null;
    let fixture: FixtureHandle | null = null;
    const consoleErrors: string[] = [];
    page.on("console", (message) => {
      if (message.type() === "error") consoleErrors.push(message.text());
    });

    try {
      fixture = await startFixture();
      test.skip(fixture.kind === "skip", fixture.kind === "skip" ? fixture.reason : "");
      const ready = fixture.ready;
      server = await serveDistHarness();
      const { port } = server.address() as AddressInfo;
      const baseUrl = `http://127.0.0.1:${port}`;

      await page.goto(
        `${baseUrl}/harness/mt249-code-intelligence.html?backend=${encodeURIComponent(
          ready.base_url,
        )}&workspace_id=${encodeURIComponent(ready.workspace_id)}&symbol_entity_id=${encodeURIComponent(
          ready.symbol_entity_id,
        )}`,
      );
      await expect(page.getByTestId("mt249-code-intelligence-root")).toBeVisible();
      await expect(page.locator("[data-testid='mt249-monaco-host'] .monaco-editor")).toBeVisible();
      await page.waitForFunction(() => window.__MT249_STATE__?.ready === true, undefined, {
        timeout: 60_000,
      });
      const harnessErrors = await page.evaluate(() => window.__MT249_STATE__?.errors ?? []);
      expect(harnessErrors).toEqual([]);

      const proof = await page.evaluate(async () => {
        const state = window.__MT249_STATE__;
        if (!state?.runFullProof) throw new Error("MT-249 harness proof function missing");
        return state.runFullProof();
      });

      expect(proof.completionWidgetText).toContain("add");
      expect(proof.hoverText).toContain("Adds two numbers.");
      expect(proof.hoverText).toContain("marked_stale");
      expect(proof.definitionRoute.originalUrl).toContain("/knowledge/code/symbols?");
      expect(proof.definitionRoute.originalUrl).toContain("name=add");
      expect(proof.referenceRoute.originalUrl).toContain(`/knowledge/code/symbols/${ready.symbol_entity_id}/references`);
      expect(proof.formatRemovedTrailingWhitespace).toBe(true);
      expect(proof.markers.some((marker) => marker.message.includes("marked_stale"))).toBe(true);
      expect(proof.problem.sample.code).toBe("HSK-CODE-INTEL-STALE");
      expect(proof.problem.sample.wsid).toBe(ready.workspace_id);
      expect(proof.problem.sample.locations?.[0]?.entity_id).toBe(ready.symbol_entity_id);
      expect(proof.symbolDetail.symbol.symbol_entity_id).toBe(ready.symbol_entity_id);

      const routeUrls = proof.routeHits.map((request) => request.originalUrl);
      expect(routeUrls.some((url) => url.includes("/knowledge/code/symbols?") && url.includes("prefix=ad"))).toBe(true);
      expect(routeUrls.some((url) => url.includes(`/knowledge/code/symbols/${ready.symbol_entity_id}`))).toBe(true);
      expect(routeUrls.some((url) => url.includes(`/knowledge/code/symbols/${ready.symbol_entity_id}/references`))).toBe(true);
      expect(routeUrls.some((url) => url.includes("/knowledge/code/files/src%2Flib.rs/lens"))).toBe(true);
      expect(routeUrls.some((url) => url.includes("/api/diagnostics"))).toBe(true);
      expect(routeUrls.some((url) => url.includes("/api/diagnostics/problems"))).toBe(true);
      expect(consoleErrors).toEqual([]);

      await page.screenshot({
        path: testInfo.outputPath("mt249-code-intelligence.png"),
        fullPage: true,
      });
    } finally {
      await closeServer(server);
      await stopFixture(fixture);
    }
  });
});

declare global {
  interface Window {
    __MT249_STATE__?: {
      ready: boolean;
      errors: string[];
      runFullProof?: () => Promise<{
        completionWidgetText: string;
        hoverText: string;
        definitionRoute: { originalUrl: string; rewrittenUrl: string; method: string };
        referenceRoute: { originalUrl: string; rewrittenUrl: string; method: string };
        formatRemovedTrailingWhitespace: boolean;
        markers: Array<{ message: string; startLineNumber: number; startColumn: number }>;
        problem: {
          sample: {
            code?: string | null;
            wsid?: string | null;
            locations?: Array<{ entity_id?: string }>;
          };
        };
        symbolDetail: { symbol: { symbol_entity_id: string } };
        routeHits: Array<{ originalUrl: string; rewrittenUrl: string; method: string }>;
      }>;
    };
  }
}
