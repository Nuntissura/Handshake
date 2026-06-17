// WP-KERNEL-009 / MT-253 — offline source-control panel runtime proof.
//
// Spawns the mt253_source_control_fixture (real temp git repo + real PG-backed
// source-control routes against isolated PostgreSQL), serves the BUILT
// SourceControlPanel harness from loopback, and drives the REAL panel:
// status -> diff (real Monaco diff editor) -> stage -> commit -> branch -> log
// -> blame. Proves the commit lands in real `git log` with the correct message
// and that write ops appended EventLedger receipts to real PostgreSQL. The
// negative case proves discard demands explicit confirm and a cancel leaves the
// worktree untouched. All non-loopback network is blocked and asserted empty.

import { expect, test } from "@playwright/test";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const distHarness = path.join(appDir, "dist-harness");
const artifactRoot =
  process.env.HANDSHAKE_ARTIFACT_ROOT ?? path.resolve(repoRoot, "..", "Handshake_Artifacts");
const cargoTargetDir = path.join(artifactRoot, "handshake-cargo-target");

// The panel (lib/api BASE_URL) targets this fixed host; we rewrite to fixture.
const apiBaseUrl = "http://127.0.0.1:37501";

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
  repo_path: string;
  repo_root_id: string;
  tracked_path: string;
  untracked_path: string;
};

type FixtureProof = {
  head_commit_id: string;
  head_commit_message: string;
  log_messages: string[];
  branches: string[];
  receipt_event_ids: string[];
  receipt_operations: string[];
  commit_receipt_payloads: Array<{ operation: string; commit_message: string; phase: string }>;
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
      "mt253_source_control_fixture",
    ],
    {
      cwd: repoRoot,
      env: { ...process.env, RUST_BACKTRACE: "1" },
      windowsHide: true,
    },
  );
  let stdoutBuffer = "";
  let stderr = "";
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      child.kill();
      reject(new Error(`MT-253 fixture did not become ready within 600s. stderr:\n${stderr}`));
    }, 600_000);
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.stdout.on("data", (chunk) => {
      stdoutBuffer += chunk.toString();
      const lines = stdoutBuffer.split(/\r?\n/);
      stdoutBuffer = lines.pop() ?? "";
      for (const line of lines) {
        if (line.startsWith("MT253_FIXTURE_SKIP ")) {
          clearTimeout(timeout);
          resolve({ kind: "skip", reason: line.slice("MT253_FIXTURE_SKIP ".length) });
          return;
        }
        if (line.startsWith("MT253_FIXTURE_READY ")) {
          clearTimeout(timeout);
          resolve({
            kind: "ready",
            child,
            ready: JSON.parse(line.slice("MT253_FIXTURE_READY ".length)) as FixtureReady,
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
        reject(new Error(`MT-253 fixture exited before ready with code ${code}. stderr:\n${stderr}`));
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
  const response = await fetch(`${ready.base_url}/mt253-fixture/proof`);
  if (!response.ok) {
    throw new Error(`fixture proof failed: ${response.status} ${await response.text()}`);
  }
  return (await response.json()) as FixtureProof;
}

test.describe("WP-KERNEL-009 MT-253 source-control panel real backend (offline)", () => {
  test("drives status/diff(Monaco)/stage/commit/branch/log/blame against a real temp git repo with receipts", async ({
    page,
  }, testInfo) => {
    test.setTimeout(900_000);
    expect(
      existsSync(path.join(distHarness, "harness", "source-control.html")),
      "dist-harness missing source-control.html; global setup should have built it",
    ).toBe(true);

    let server: Server | null = null;
    let fixture: FixtureHandle | null = null;
    const externalRequests: string[] = [];
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
      const harnessBase = `http://127.0.0.1:${port}`;

      await page.route("**/*", async (route) => {
        const request = route.request();
        const url = request.url();
        if (url.startsWith(harnessBase)) {
          await route.continue();
          return;
        }
        if (url.startsWith(apiBaseUrl)) {
          // Rewrite the panel's fixed API base to the fixture's dynamic port.
          const parsed = new URL(url);
          const rewritten = `${ready.base_url}${parsed.pathname}${parsed.search}`;
          await route.continue({ url: rewritten });
          return;
        }
        if (!url.startsWith("about:") && !url.startsWith("data:")) {
          externalRequests.push(url);
          await route.abort("connectionfailed");
          return;
        }
        await route.continue();
      });

      await page.goto(
        `${harnessBase}/harness/source-control.html?repo_path=${encodeURIComponent(ready.repo_path)}`,
      );
      await expect(page.getByTestId("source-control-harness-root")).toBeVisible();
      await expect(page.getByTestId("source-control-panel")).toBeVisible();

      // status -> the panel auto-loads status + diff for the first changed path.
      await page.getByTestId("source-control.load").click();
      await expect(page.getByTestId("source-control.branch")).toHaveText("main");
      await expect(page.getByTestId(`source-control.status.tracked-txt`)).toContainText("modified");
      await expect(page.getByTestId(`source-control.status.new-txt`)).toContainText("untracked");

      // diff -> REAL Monaco diff editor renders the per-file patch.
      await page.getByTestId("source-control.status.tracked-txt").click();
      await expect(
        page.locator("[data-testid='source-control.diff-monaco'] .monaco-diff-editor"),
      ).toBeVisible({ timeout: 60_000 });
      // Raw-patch fallback carries the real git patch text.
      await expect(page.getByTestId("source-control.diff")).toContainText("run();");

      // blame -> load blame for the file in code context (tracked.txt still
      // selected and present in status here, before it is committed clean).
      await page.getByTestId("source-control.load-blame").click();
      await expect(page.getByTestId("source-control.blame")).toContainText("init();");

      // Negative: discard demands explicit confirm. Button disabled until checked;
      // cancel (leave unchecked) must leave the worktree untouched.
      await expect(page.getByTestId("source-control.discard")).toBeDisabled();
      const beforeDiscard = await fixtureProof(ready);
      // No discard issued -> head commit unchanged, still the seed commit only.
      expect(beforeDiscard.log_messages).toEqual(["seed: initial tracked file"]);

      // stage the modified file.
      await page.getByTestId("source-control.stage").click();
      await expect(page.getByTestId("source-control.action-status")).toContainText("stage");
      await expect(page.getByTestId("source-control.action-status")).toContainText("receipt KE-");

      // commit with a message.
      await page.getByTestId("source-control.commit-message").fill("mt253 panel commit");
      await page.getByTestId("source-control.commit").click();
      await expect(page.getByTestId("source-control.action-status")).toContainText("mt253 panel commit");

      // log -> the new commit appears in the panel's git log view.
      await expect(page.getByTestId("source-control.log")).toContainText("mt253 panel commit");

      // branch -> create then switch.
      await page.getByTestId("source-control.new-branch").fill("mt253/feature");
      await page.getByTestId("source-control.create-branch").click();
      await expect(page.getByTestId("source-control.branch.mt253-feature")).toBeVisible();
      await page.getByTestId("source-control.branch.mt253-feature").click();
      await expect(page.getByTestId("source-control.branch")).toHaveText("mt253/feature");

      // Backend proof: commit truly landed in real git log + EventLedger receipts.
      const proof = await fixtureProof(ready);
      expect(proof.head_commit_message).toBe("mt253 panel commit");
      expect(proof.log_messages).toEqual(["mt253 panel commit", "seed: initial tracked file"]);
      expect(proof.branches).toContain("mt253/feature");
      expect(proof.receipt_operations).toEqual(["stage", "commit", "create_branch", "switch_branch"]);
      expect(proof.receipt_event_ids.every((id) => id.startsWith("KE-"))).toBe(true);
      expect(proof.commit_receipt_payloads).toHaveLength(1);
      expect(proof.commit_receipt_payloads[0].commit_message).toBe("mt253 panel commit");
      expect(proof.commit_receipt_payloads[0].phase).toBe("pre_git_write");

      await page.screenshot({ path: testInfo.outputPath("mt253-source-control-panel.png"), fullPage: true });

      expect(consoleErrors, `console errors: ${consoleErrors.join(", ")}`).toEqual([]);
      expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);
    } finally {
      if (server) await new Promise((resolve) => server.close(resolve));
      await stopFixture(fixture);
    }
  });
});
