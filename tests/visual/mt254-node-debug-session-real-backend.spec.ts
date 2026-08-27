import { expect, test } from "./console_error_scan";

import { spawn, spawnSync, type ChildProcessWithoutNullStreams } from "node:child_process";
import path from "node:path";

import { buildDebugSessionHarness } from "./build_debug_session_harness";

// The frontend dap_client/api speak to the product backend at this fixed origin;
// the spec rewrites these requests onto the per-run fixture base url and blocks
// every other (external) request, proving zero external network access.
const apiBaseUrl = "http://127.0.0.1:37501";
const repoRoot = path.resolve(__dirname, "..", "..");
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT ?? path.resolve(repoRoot, "..", "Handshake_Artifacts");
const cargoTargetDir = path.join(artifactRoot, "handshake-cargo-target");

const PAGE_SHELL = (css: string) => `<!doctype html>
<html>
  <head><meta charset="utf-8"><style>${css}
    body { margin: 0; font-family: ui-sans-serif, system-ui, sans-serif; background: #f8fafc; }
    /* MT-254 gutter glyphs must be visible for the spec to assert decorations. */
    .hsk-debug-breakpoint-glyph { background: #ef4444; border-radius: 50%; }
    .hsk-debug-breakpoint-glyph-unverified { border: 1px solid #ef4444; border-radius: 50%; }
    .hsk-debug-current-stop-line { background: rgba(250, 204, 21, 0.25); }
    .hsk-debug-current-stop-glyph { background: #facc15; }
  </style></head>
  <body>
    <main data-testid="capture-root" style="padding:16px; width:1040px;">
      <div id="harness-root"></div>
    </main>
  </body>
</html>`;

type FixtureReady = {
  base_url: string;
  workspace_id: string;
  rich_document_id: string;
  script_path: string;
  script_url: string;
  breakpoint_line: number;
  node_available: boolean;
};

type FixtureProof = {
  breakpoint_lines: number[];
  receipt_event_ids: string[];
  receipt_event_types: string[];
};

type FixtureHandle =
  | { kind: "skip"; reason: string }
  | { kind: "ready"; child: ChildProcessWithoutNullStreams; ready: FixtureReady; stderr: () => string };

const SCRIPT_TEXT =
  'function add(a, b) {\n  const sum = a + b;\n  return sum;\n}\nconst result = add(2, 40);\nconsole.log("result=" + result);\n';

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
      "mt254_debug_session_fixture",
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
        () => reject(new Error(`MT-254 fixture did not become ready within 600s. stderr:\n${stderr}`)),
        (teardownError) =>
          reject(
            new Error(
              `MT-254 fixture did not become ready within 600s. Startup teardown failed: ${String(teardownError)}. stderr:\n${stderr}`,
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
        if (line.startsWith("MT254_FIXTURE_SKIP ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({ kind: "skip", reason: line.slice("MT254_FIXTURE_SKIP ".length) });
          return;
        }
        if (line.startsWith("MT254_FIXTURE_READY ")) {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          resolve({
            kind: "ready",
            child,
            ready: JSON.parse(line.slice("MT254_FIXTURE_READY ".length)) as FixtureReady,
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
          `MT-254 fixture exited before ready with code ${code} and signal ${signal}. stderr:\n${stderr}`,
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

async function terminateDebugSession(ready: FixtureReady, sessionId: string): Promise<void> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 7_000);
  try {
    const response = await fetch(
      `${ready.base_url}/debug/sessions/${encodeURIComponent(sessionId)}`,
      { method: "DELETE", signal: controller.signal },
    );
    const responseText = await response.text();
    if (!response.ok) {
      throw new Error(`DELETE returned ${response.status}: ${responseText}`);
    }
    const body = JSON.parse(responseText) as { terminated?: unknown; exit_code?: unknown };
    if (
      body.terminated !== true ||
      !(body.exit_code === null || typeof body.exit_code === "number")
    ) {
      throw new Error(`DELETE returned an invalid termination receipt: ${responseText}`);
    }
  } finally {
    clearTimeout(timeout);
  }
}

async function fixtureProof(ready: FixtureReady): Promise<FixtureProof> {
  const response = await fetch(`${ready.base_url}/mt254-fixture/proof`);
  if (!response.ok) {
    throw new Error(`fixture proof failed: ${response.status} ${await response.text()}`);
  }
  return (await response.json()) as FixtureProof;
}

test.describe("WP-KERNEL-009 MT-254 Node debug session real backend", () => {
  test("operator sets a breakpoint, launches a real node debuggee, hits it, inspects variables, evaluates, steps, continues to exit", async ({
    page,
  }, testInfo) => {
    test.setTimeout(900_000);
    const { js, css } = await buildDebugSessionHarness();
    let fixture: FixtureHandle | null = null;
    let debugSessionId: string | null = null;
    let launchResponsePromise: Promise<string> | null = null;
    const externalRequests: string[] = [];

    try {
      fixture = await startFixture();
      test.skip(fixture.kind === "skip", fixture.kind === "skip" ? fixture.reason : "");
      const ready = fixture.ready;
      test.skip(!ready.node_available, "node is not on PATH; debug session cannot launch");
      expect(ready.breakpoint_line).toBe(2);

      // Rewrite product API origin -> per-run fixture; block all external requests.
      await page.route("**/*", async (route) => {
        const request = route.request();
        const url = request.url();
        if (url.startsWith(apiBaseUrl)) {
          const parsed = new URL(url);
          const rewrittenUrl = `${ready.base_url}${parsed.pathname}${parsed.search}`;
          await route.continue({ url: rewrittenUrl });
          return;
        }
        if (!url.startsWith("about:") && !url.startsWith("data:") && !url.startsWith("blob:")) {
          externalRequests.push(url);
          await route.abort("connectionfailed");
          return;
        }
        await route.continue();
      });

      // Inject the fixture's real script config into the page, THEN mount the
      // harness (the harness IIFE reads window.__mt254DebugConfig at eval time).
      await page.setContent(PAGE_SHELL(css), { waitUntil: "domcontentloaded" });
      await page.evaluate(
        (config) => {
          (window as unknown as { __mt254DebugConfig: unknown }).__mt254DebugConfig = config;
        },
        { program: ready.script_path, sourceUrl: ready.script_url, sourceText: SCRIPT_TEXT },
      );
      await page.addScriptTag({ content: js });

      await expect(page.getByTestId("debug-side-panel")).toBeVisible();
      // Honesty gate: the adapter picker shows exactly one runnable adapter (Node).
      const adapterOptions = page.locator('[data-testid="debug-side-panel.adapter"] option');
      await expect(adapterOptions).toHaveCount(1);
      await expect(adapterOptions.first()).toHaveText(/Node\.js/);

      // The real Monaco editor with the breakpoint gutter mounts.
      await expect(page.getByTestId("debug-side-panel.editor").locator(".monaco-editor")).toBeVisible();

      // Click the glyph margin on line 2 to set a breakpoint (gutter toggle).
      // Monaco renders the glyph margin; clicking the line's margin toggles it.
      // We click the line-2 view-line's glyph margin via the editor mouse target;
      // the panel resolves the toggle and renders the breakpoint glyph.
      const line2 = page.locator(".monaco-editor .margin-view-overlays > div").nth(1);
      await line2.click({ position: { x: 3, y: 3 } });
      await expect(page.locator(`.${"hsk-debug-breakpoint-glyph-unverified"}`)).toHaveCount(1);

      // Launch the REAL node debuggee.
      launchResponsePromise = page
        .waitForResponse(
          (response) => {
            const url = new URL(response.url());
            return response.request().method() === "POST" && url.pathname === "/debug/sessions";
          },
          { timeout: 60_000 },
        )
        .then(async (response) => {
          const responseText = await response.text();
          if (!response.ok()) {
            throw new Error(`debug-session launch returned ${response.status()}: ${responseText}`);
          }
          const body = JSON.parse(responseText) as { session_id?: unknown };
          if (typeof body.session_id !== "string" || body.session_id.length === 0) {
            throw new Error(`debug-session launch omitted session_id: ${responseText}`);
          }
          debugSessionId = body.session_id;
          return body.session_id;
        });
      await page.getByTestId("debug-side-panel.launch").click();
      await launchResponsePromise;

      // It hits the breakpoint on line 2 and pauses.
      await expect(page.getByTestId("debug-side-panel.status")).toHaveAttribute(
        "data-status",
        "paused",
        { timeout: 60_000 },
      );
      // The bound breakpoint becomes VERIFIED (solid glyph) — real CDP binding.
      await expect(page.locator(`.${"hsk-debug-breakpoint-glyph"}`)).toHaveCount(1);
      // The current-stop line decoration is on line 2.
      await expect(page.locator(`.${"hsk-debug-current-stop-line"}`)).toHaveCount(1);

      // Real call stack: top frame is add() at line 2.
      await expect(page.getByTestId("debug-side-panel.frame.add")).toHaveAttribute("data-line", "2");

      // Real local variables: a == 2, b == 40.
      await expect(page.getByTestId("debug-side-panel.var.a")).toHaveAttribute("data-value", "2");
      await expect(page.getByTestId("debug-side-panel.var.b")).toHaveAttribute("data-value", "40");

      // Debug-console eval in the paused frame: a + b == 42 (real evaluateOnCallFrame).
      await page.getByTestId("debug-console.input").fill("a + b");
      await page.getByTestId("debug-console.eval").click();
      await expect(page.getByTestId("debug-console.entry.result")).toHaveText("42");

      // Step over advances to line 3.
      await page.getByTestId("debug-side-panel.step-over").click();
      await expect(page.getByTestId("debug-side-panel.frame.add")).toHaveAttribute("data-line", "3", {
        timeout: 30_000,
      });

      await page.getByTestId("capture-root").screenshot({
        path: testInfo.outputPath("mt254-node-debug-session-real-backend.png"),
      });

      // Continue to completion; the real node process exits 0.
      await page.getByTestId("debug-side-panel.continue").click();
      await expect(page.getByTestId("debug-side-panel.status")).toHaveAttribute(
        "data-status",
        "terminated",
        { timeout: 30_000 },
      );
      await expect(page.getByTestId("debug-side-panel.status")).toContainText("exit 0");

      // Durable breakpoint persistence: PUT the breakpoint set (through the
      // product API origin so it is rewritten onto the fixture by page.route —
      // proving the request goes through the same in-app transport) and read it
      // back from REAL SurrealDB + EventLedger via the fixture proof endpoint.
      const putResponse = await page.evaluate(
        async ({ apiBase, docId, workspaceId }) => {
          const res = await fetch(`${apiBase}/debug/documents/${encodeURIComponent(docId)}/breakpoints`, {
            method: "PUT",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              workspace_id: workspaceId,
              breakpoints: [{ source_url: "file:///x.js", line: 2, verified: true }],
            }),
          });
          return { ok: res.ok, body: (await res.json()) as { breakpoints: Array<{ line: number; event_ledger_event_id: string }> } };
        },
        { apiBase: apiBaseUrl, docId: ready.rich_document_id, workspaceId: ready.workspace_id },
      );
      expect(putResponse.ok).toBe(true);
      expect(putResponse.body.breakpoints).toHaveLength(1);
      expect(putResponse.body.breakpoints[0].line).toBe(2);
      expect(putResponse.body.breakpoints[0].event_ledger_event_id).toMatch(/^KE-/);

      const proof = await fixtureProof(ready);
      expect(proof.breakpoint_lines).toEqual([2]);
      expect(proof.receipt_event_ids.length).toBeGreaterThan(0);
      expect(proof.receipt_event_types).toContain("knowledge_debug_breakpoints_recorded");

      // Zero external network requests were made.
      expect(externalRequests).toEqual([]);
    } finally {
      const cleanupErrors: string[] = [];
      if (launchResponsePromise && !debugSessionId) {
        let idRecoveryTimeout: ReturnType<typeof setTimeout> | undefined;
        try {
          await Promise.race([
            launchResponsePromise,
            new Promise<never>((_, reject) => {
              idRecoveryTimeout = setTimeout(
                () => reject(new Error("timed out obtaining launched debug-session id")),
                5_000,
              );
            }),
          ]);
        } catch (error) {
          cleanupErrors.push(`debug-session id recovery failed: ${String(error)}`);
        } finally {
          clearTimeout(idRecoveryTimeout);
        }
      }
      if (fixture?.kind === "ready" && debugSessionId) {
        try {
          await terminateDebugSession(fixture.ready, debugSessionId);
        } catch (error) {
          cleanupErrors.push(`debug-session termination failed: ${String(error)}`);
        }
      }
      try {
        await stopFixture(fixture);
      } catch (error) {
        cleanupErrors.push(`fixture shutdown failed: ${String(error)}`);
      }
      if (cleanupErrors.length > 0) {
        throw new Error(cleanupErrors.join("; "));
      }
    }
  });
});
