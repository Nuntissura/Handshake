// WP-KERNEL-009 / MT-246 - real App split editor workbench proof.
//
// Starts the real Vite app shell on loopback, fulfills the backend authority
// routes needed by RichDocumentView, blocks non-loopback network, and drives the
// actual pane/tab/workbench UI through Chromium.

import { expect, test, type Page, type Route } from "@playwright/test";
import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { createServer, get as httpGet } from "node:http";
import { createRequire } from "node:module";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const appDir = path.join(repoRoot, "app");
const appRequire = createRequire(path.join(appDir, "package.json"));
const vitePkgPath = appRequire.resolve("vite/package.json");
const vitePkg = JSON.parse(readFileSync(vitePkgPath, "utf8")) as { bin?: string | Record<string, string> };
const viteBinRel = typeof vitePkg.bin === "string" ? vitePkg.bin : vitePkg.bin?.vite;
if (!viteBinRel) throw new Error(`vite package.json at ${vitePkgPath} declares no vite bin`);
const viteCli = path.join(path.dirname(vitePkgPath), viteBinRel);
const apiBaseUrl = "http://127.0.0.1:37501";

const workspaceId = "project-main";
const alphaId = "KRD-00000000000000000000000000000001";
const betaId = "KRD-00000000000000000000000000000002";
const alphaCrdtId = "KCRDT-000000000000000000000000000001";
const betaCrdtId = "KCRDT-000000000000000000000000000002";

type JsonContent = {
  type?: string;
  attrs?: Record<string, unknown>;
  content?: JsonContent[];
  text?: string;
  [key: string]: unknown;
};

type RichDocument = {
  rich_document_id: string;
  workspace_id: string;
  document_id: string | null;
  title: string;
  schema_version: string;
  doc_version: number;
  content_json: JsonContent;
  content_sha256: string;
  crdt_document_id: string | null;
  crdt_snapshot_id: string | null;
  promotion_receipt_event_id: string | null;
  projection_refs: unknown[];
  project_ref: string | null;
  folder_ref: string | null;
  authority_label: string;
  owner_actor_kind: string | null;
  owner_actor_id: string | null;
  created_at: string;
  updated_at: string;
};

type WorkbenchLayoutState = {
  schema_id: "hsk.workbench_layout_state@1";
  activePaneId: string;
  activeModule: string;
  splitWeights: { vertical: number; horizontal: number };
  drawers: { project: boolean; file: boolean; bottom: boolean };
  panes: Array<{
    id: string;
    module: string;
    activeTab: string;
    tabs: string[];
    locked: boolean;
    projectRef: string;
    activeDocumentId: string | null;
    activeCanvasId?: string | null;
    openDocuments: Array<{ documentId: string; pinned: boolean; dirty: boolean }>;
  }>;
};

const docs = new Map<string, RichDocument>();
let workbenchLayoutState: WorkbenchLayoutState | null = null;
let layoutSaveCount = 0;

function stableIdPart(value: string): string {
  const stable = value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stable || "item";
}

function tabTestId(paneId: string, documentId: string) {
  return `pane-${paneId}.document-tab.${stableIdPart(documentId)}`;
}

function docContent(text: string): JsonContent {
  return { type: "doc", content: [{ type: "paragraph", content: [{ type: "text", text }] }] };
}

function makeRichDocument(id: string, title: string, crdtDocumentId: string, text: string): RichDocument {
  return {
    rich_document_id: id,
    workspace_id: workspaceId,
    document_id: null,
    title,
    schema_version: "rich_document_v1",
    doc_version: 1,
    content_json: docContent(text),
    content_sha256: "0".repeat(64),
    crdt_document_id: crdtDocumentId,
    crdt_snapshot_id: null,
    promotion_receipt_event_id: null,
    projection_refs: [],
    project_ref: workspaceId,
    folder_ref: "mt246-workbench-proof",
    authority_label: "promoted",
    owner_actor_kind: "operator",
    owner_actor_id: "operator",
    created_at: "2026-06-15T00:00:00Z",
    updated_at: "2026-06-15T00:00:00Z",
  };
}

function resetFixtureState() {
  docs.clear();
  docs.set(
    alphaId,
    makeRichDocument(alphaId, "Alpha Rich Workbench Doc", alphaCrdtId, "alpha initial split workbench text"),
  );
  docs.set(
    betaId,
    makeRichDocument(betaId, "Beta Rich Workbench Doc", betaCrdtId, "beta initial split workbench text"),
  );
  workbenchLayoutState = null;
  layoutSaveCount = 0;
}

function jsonResponse(route: Route, body: unknown) {
  return route.fulfill({
    status: 200,
    contentType: "application/json",
    body: JSON.stringify(body),
  });
}

function documentSummaries() {
  return Array.from(docs.values()).map((doc) => ({
    id: doc.rich_document_id,
    workspace_id: workspaceId,
    title: doc.title,
    created_at: doc.created_at,
    updated_at: doc.updated_at,
  }));
}

function richDocLoad(documentId: string) {
  const doc = docs.get(documentId);
  if (!doc) throw new Error(`unknown rich document ${documentId}`);
  return {
    document: doc,
    tree: { schema_version: "rich_document_v1", schema_matches: true, block_ids: [], blocks: [] },
    code_nodes: [],
  };
}

function richDocHistory(documentId: string) {
  const doc = docs.get(documentId);
  if (!doc) throw new Error(`unknown rich document ${documentId}`);
  return {
    rich_document_id: doc.rich_document_id,
    current_version: doc.doc_version,
    authority_label: doc.authority_label,
    owner_actor_kind: doc.owner_actor_kind,
    owner_actor_id: doc.owner_actor_id,
    versions: [
      {
        rich_document_id: doc.rich_document_id,
        doc_version: doc.doc_version,
        schema_version: doc.schema_version,
        content_sha256: doc.content_sha256,
        promotion_receipt_event_id: doc.promotion_receipt_event_id,
        created_at: doc.updated_at,
      },
    ],
  };
}

async function fulfillApi(route: Route) {
  const request = route.request();
  const url = new URL(request.url());
  const method = request.method();

  if (url.pathname === "/workspaces") {
    return jsonResponse(route, [
      {
        id: workspaceId,
        name: "Project Main",
        created_at: "2026-06-15T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
      },
    ]);
  }
  if (url.pathname === `/workspaces/${workspaceId}/documents`) {
    return jsonResponse(route, documentSummaries());
  }
  if (url.pathname === `/workspaces/${workspaceId}/canvases`) {
    return jsonResponse(route, []);
  }
  if (url.pathname === `/workspaces/${workspaceId}/workbench/layout`) {
    if (method === "PUT") {
      const payload = request.postDataJSON() as { layout_state?: WorkbenchLayoutState };
      workbenchLayoutState = payload.layout_state ?? null;
      layoutSaveCount += 1;
    }
    return jsonResponse(route, {
      workspace_id: workspaceId,
      layout_state: workbenchLayoutState,
      updated_at: workbenchLayoutState ? "2026-06-15T00:00:00Z" : null,
      event_ledger_event_id: workbenchLayoutState ? `EVT-workbench-layout-${layoutSaveCount}` : null,
    });
  }

  const richDocMatch = /^\/knowledge\/documents\/([^/]+)(?:\/([^/]+)(?:\/([^/]+))?)?$/.exec(url.pathname);
  if (richDocMatch) {
    const documentId = decodeURIComponent(richDocMatch[1]);
    const segment = richDocMatch[2];
    const childSegment = richDocMatch[3];
    if (!segment && method === "GET") {
      return jsonResponse(route, richDocLoad(documentId));
    }
    if (segment === "save" && method === "PUT") {
      const doc = docs.get(documentId);
      if (!doc) return route.fulfill({ status: 404, body: "not found" });
      const payload = request.postDataJSON() as {
        expected_version?: number;
        content_json?: JsonContent;
        crdt_document_id?: string | null;
      };
      if (payload.expected_version !== doc.doc_version) {
        return route.fulfill({
          status: 409,
          contentType: "application/json",
          body: JSON.stringify({
            error: `HSK-409 version conflict: expected_version ${payload.expected_version ?? "missing"} got ${
              doc.doc_version
            }`,
          }),
        });
      }
      const nextDoc: RichDocument = {
        ...doc,
        doc_version: doc.doc_version + 1,
        content_json: payload.content_json ?? doc.content_json,
        crdt_document_id: payload.crdt_document_id ?? doc.crdt_document_id,
        updated_at: "2026-06-15T00:01:00Z",
      };
      docs.set(documentId, nextDoc);
      return jsonResponse(route, {
        document: nextDoc,
        save_receipt_event_id: `EVT-save-${nextDoc.doc_version}-${stableIdPart(documentId)}`,
        backlinks_persisted: 0,
        backlinks_skipped_reason: null,
      });
    }
    if (segment === "history") {
      return jsonResponse(route, richDocHistory(documentId));
    }
    if (segment === "embeds" && childSegment === "broken") {
      return jsonResponse(route, { rich_document_id: documentId, broken_embeds: [], available_actions: [] });
    }
    if (segment === "embeds") {
      return jsonResponse(route, { rich_document_id: documentId, embeds: [] });
    }
    if (segment === "backlinks") {
      return jsonResponse(route, { source_document_id: documentId, backlinks: [] });
    }
    if (segment === "blocks") {
      return jsonResponse(route, richDocLoad(documentId).tree);
    }
  }

  if (url.pathname === "/api/flight_recorder") {
    return jsonResponse(route, []);
  }
  if (url.pathname === "/logs/tail") {
    return jsonResponse(route, { lines: [] });
  }
  if (url.pathname === "/health") {
    return jsonResponse(route, { status: "ok", db_status: "ok", component: "mt246-workbench-fixture" });
  }
  if (url.pathname === "/api/usermanual/access-points") {
    return jsonResponse(route, { count: 0, access_points: [] });
  }
  if (url.pathname === "/api/usermanual/pages") {
    return jsonResponse(route, {
      manual_version: "2.0.0",
      route_namespace: "/usermanual",
      count: 0,
      pages: [],
    });
  }
  if (url.pathname === "/api/usermanual/search") {
    return jsonResponse(route, { query: url.searchParams.get("q") ?? "", count: 0, results: [] });
  }
  if (url.pathname === "/api/kernel/dcc_projection") {
    return jsonResponse(route, {
      schema_id: "hsk.kernel.dcc_mvp_runtime_surface@1",
      surface_id: "dcc-mt246-fixture",
      folded_stub_id: "WP-KERNEL-009",
      panels: [],
      work_items: [],
      worktrees: [],
      sessions: [],
      proposals: [],
      evidence: [],
      approval_previews: [],
      write_box_queue_rows: [],
      direct_edit_denials: [],
      promotion_previews: [],
      freshness_badges: [],
      stable_element_ids: [],
      catalog_action_refs: [],
      catalog_action_rows: [],
      direct_authority_mutation_allowed: false,
      ungoverned_tool_execution_allowed: false,
      destructive_git_ops_require_same_turn_approval: true,
      flight_recorder_event_types: [],
      product_authority_refs: [],
      folded_source_refs: [],
      spawn_tree_projection: null,
    });
  }
  if (url.pathname.startsWith("/atelier/")) {
    return jsonResponse(route, []);
  }

  return jsonResponse(route, {});
}

function freePort(): Promise<number> {
  const server = createServer();
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") {
          resolve(address.port);
        } else {
          reject(new Error("could not allocate a loopback port"));
        }
      });
    });
  });
}

async function waitForHttp(url: string, proc: ChildProcessWithoutNullStreams, output: string[]) {
  const deadline = Date.now() + 45_000;
  while (Date.now() < deadline) {
    if (proc.exitCode !== null) {
      throw new Error(`vite exited early with ${proc.exitCode}\n${output.join("")}`);
    }
    const ready = await new Promise<boolean>((resolve) => {
      const req = httpGet(url, (res) => {
        res.resume();
        resolve((res.statusCode ?? 500) < 500);
      });
      req.once("error", () => resolve(false));
      req.setTimeout(1_000, () => {
        req.destroy();
        resolve(false);
      });
    });
    if (ready) return;
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`vite did not become ready at ${url}\n${output.join("")}`);
}

async function paneTabOrder(page: Page, paneId: string): Promise<string[]> {
  return page
    .getByTestId(`pane-${paneId}.document-tabs`)
    .locator("[data-document-id]")
    .evaluateAll((nodes) => nodes.map((node) => node.getAttribute("data-document-id") ?? ""));
}

async function editorSurface(page: Page, paneId: string) {
  return page.getByTestId(`pane-${paneId}`).locator("[data-testid='rich-text-editor-surface'] .ProseMirror").first();
}

function fileDrawerDocument(page: Page, title: string) {
  return page.getByTestId("file-drawer").getByText(title, { exact: true });
}

test.describe("WP-KERNEL-009 MT-246 App split editor workbench proof", () => {
  let viteProcess: ChildProcessWithoutNullStreams;
  let baseUrl: string;
  const serverOutput: string[] = [];

  test.beforeAll(async () => {
    expect(existsSync(viteCli), `vite executable missing at ${viteCli}`).toBe(true);
    const port = await freePort();
    baseUrl = `http://127.0.0.1:${port}`;
    viteProcess = spawn(process.execPath, [viteCli, "--host", "127.0.0.1", "--port", String(port), "--strictPort"], {
      cwd: appDir,
      env: { ...process.env, BROWSER: "none" },
      stdio: ["ignore", "pipe", "pipe"],
    });
    viteProcess.stdout.on("data", (chunk) => serverOutput.push(String(chunk)));
    viteProcess.stderr.on("data", (chunk) => serverOutput.push(String(chunk)));
    await waitForHttp(baseUrl, viteProcess, serverOutput);
  });

  test.afterAll(async () => {
    if (viteProcess && viteProcess.exitCode === null) {
      viteProcess.kill();
    }
  });

  test("opens, pins, reorders, edits same-doc groups, drags between groups, and restores layout", async ({
    page,
  }, testInfo) => {
    const externalRequests: string[] = [];
    const browserDiagnostics: string[] = [];
    const dialogResponses: boolean[] = [];
    resetFixtureState();

    page.on("console", (message) => {
      browserDiagnostics.push(`[console:${message.type()}] ${message.text()}`);
    });
    page.on("pageerror", (error) => {
      browserDiagnostics.push(`[pageerror] ${error.message}\n${error.stack ?? ""}`);
    });
    page.on("dialog", async (dialog) => {
      browserDiagnostics.push(`[dialog:${dialog.type()}] ${dialog.message()}`);
      const shouldAccept = dialogResponses.shift() ?? true;
      if (shouldAccept) {
        await dialog.accept();
      } else {
        await dialog.dismiss();
      }
    });

    await page.route("**/*", async (route) => {
      const url = route.request().url();
      if (url.startsWith(apiBaseUrl)) {
        await fulfillApi(route);
        return;
      }
      if (url.startsWith(baseUrl)) {
        await route.continue();
        return;
      }
      externalRequests.push(url);
      await route.abort("connectionfailed");
    });

    await page.goto(baseUrl);
    await expect(page.getByTestId("main-window"))
      .toBeVisible({ timeout: 10_000 })
      .catch((error) => {
        throw new Error(`${error.message}\n\nBrowser diagnostics:\n${browserDiagnostics.join("\n")}`);
      });
    await expect(fileDrawerDocument(page, "Alpha Rich Workbench Doc")).toBeVisible();
    await expect(page.getByTestId("pane-grid")).toBeVisible();

    const verticalSplitter = page.getByTestId("main-window-splitter-vertical");
    const horizontalSplitter = page.getByTestId("main-window-splitter-horizontal");
    const initialSplitWeights = await page.getByTestId("main-window").getAttribute("data-split-weights");
    const isStackedPaneLayout = (page.viewportSize()?.width ?? 9999) <= 700;
    if (isStackedPaneLayout) {
      await expect(verticalSplitter).toBeHidden();
      await expect(horizontalSplitter).toBeHidden();
    } else {
      await expect(verticalSplitter).toBeVisible();
      await expect(horizontalSplitter).toBeVisible();
      await verticalSplitter.focus();
      await page.keyboard.press("ArrowRight");
      await horizontalSplitter.focus();
      await page.keyboard.press("ArrowDown");
      await expect
        .poll(() => page.getByTestId("main-window").getAttribute("data-split-weights"))
        .not.toBe(initialSplitWeights);
    }

    await fileDrawerDocument(page, "Alpha Rich Workbench Doc").click();
    await expect(page.getByTestId(tabTestId("pane-a", alphaId))).toHaveAttribute("data-active", "true");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-active-document-id", alphaId);
    await expect(page.getByTestId("pane-pane-a").getByTestId("rich-document-title")).toContainText(
      "Alpha Rich Workbench Doc",
    );

    await fileDrawerDocument(page, "Beta Rich Workbench Doc").click();
    await expect(page.getByTestId(tabTestId("pane-a", betaId))).toHaveAttribute("data-active", "true");
    await expect(page.getByTestId("pane-pane-a")).toHaveAttribute("data-pane-open-document-count", "2");
    await page.getByTestId(`${tabTestId("pane-a", betaId)}.pin`).click();
    await expect(page.getByTestId(tabTestId("pane-a", betaId))).toHaveAttribute("data-pinned", "true");
    await page.getByTestId(`${tabTestId("pane-a", betaId)}.move-left`).click();
    await expect.poll(() => paneTabOrder(page, "pane-a")).toEqual([betaId, alphaId]);
    await page.getByTestId(`${tabTestId("pane-a", alphaId)}.activate`).click();
    await expect(page.getByTestId(tabTestId("pane-a", alphaId))).toHaveAttribute("data-active", "true");

    await page.keyboard.press("Control+Alt+ArrowRight");
    await expect(page.getByTestId("main-window")).toHaveAttribute("data-active-pane-id", "pane-b");
    await fileDrawerDocument(page, "Alpha Rich Workbench Doc").click();
    await expect(page.getByTestId(tabTestId("pane-b", alphaId))).toHaveAttribute("data-active", "true");
    await expect(page.getByTestId("pane-pane-b").getByTestId("rich-document-title")).toContainText(
      "Alpha Rich Workbench Doc",
    );

    const editText = "MT246 same-doc browser edit";
    const paneBEditor = await editorSurface(page, "pane-b");
    await expect(paneBEditor).toBeVisible();
    await paneBEditor.click();
    await page.keyboard.type(` ${editText}`);
    await expect(page.getByTestId("pane-pane-b").getByTestId("rich-document-view")).toHaveAttribute(
      "data-dirty",
      "true",
    );
    await expect(page.getByTestId(tabTestId("pane-b", alphaId))).toHaveAttribute("data-dirty", "true");
    await expect(page.getByTestId("pane-pane-a").locator("[data-testid='rich-text-editor-surface']")).toContainText(
      editText,
    );

    dialogResponses.push(false);
    await page.getByTestId(`${tabTestId("pane-b", alphaId)}.close`).click();
    await expect(page.getByTestId(tabTestId("pane-b", alphaId))).toHaveAttribute("data-dirty", "true");
    await expect(page.getByTestId("pane-pane-b")).toHaveAttribute("data-pane-active-document-id", alphaId);

    await page.getByTestId("pane-pane-b").getByTestId("rich-document-save").click();
    await expect(page.getByTestId("pane-pane-b").getByTestId("rich-document-view")).toHaveAttribute(
      "data-dirty",
      "false",
    );
    await expect(page.getByTestId("pane-pane-a").getByTestId("rich-document-view")).toHaveAttribute(
      "data-dirty",
      "false",
    );
    await expect(page.getByTestId(tabTestId("pane-a", alphaId))).toHaveAttribute("data-dirty", "false");
    await page.screenshot({ path: testInfo.outputPath("mt246-workbench-same-doc-clean.png"), fullPage: true });

    if (isStackedPaneLayout) {
      await expect(page.getByTestId(tabTestId("pane-a", betaId))).toHaveAttribute("data-pinned", "true");
    } else {
      await page.getByTestId(tabTestId("pane-a", betaId)).dragTo(page.getByTestId("pane-pane-b.document-tabs"));
      await expect(page.getByTestId(tabTestId("pane-a", betaId))).toHaveCount(0);
      await expect(page.getByTestId(tabTestId("pane-b", betaId))).toHaveAttribute("data-active", "true");
      await expect(page.getByTestId(tabTestId("pane-b", betaId))).toHaveAttribute("data-pinned", "true");
      await page.getByTestId(`${tabTestId("pane-b", alphaId)}.close`).click();
      await expect(page.getByTestId(tabTestId("pane-b", alphaId))).toHaveCount(0);
    }

    await expect
      .poll(() => layoutSaveCount, { message: "workbench layout should save through the backend fixture" })
      .toBeGreaterThan(0);
    const savedBeforeReload = workbenchLayoutState;
    expect(savedBeforeReload?.panes.find((pane) => pane.id === "pane-a")?.openDocuments).toEqual(
      isStackedPaneLayout
        ? [
            { documentId: betaId, pinned: true, dirty: false },
            { documentId: alphaId, pinned: false, dirty: false },
          ]
        : [{ documentId: alphaId, pinned: false, dirty: false }],
    );
    expect(savedBeforeReload?.panes.find((pane) => pane.id === "pane-b")?.openDocuments).toEqual(
      isStackedPaneLayout
        ? [{ documentId: alphaId, pinned: false, dirty: false }]
        : [{ documentId: betaId, pinned: true, dirty: false }],
    );

    const restoredSplitWeights = await page.getByTestId("main-window").getAttribute("data-split-weights");
    await page.reload();
    await expect(page.getByTestId("main-window")).toBeVisible({ timeout: 10_000 });
    await expect(page.getByTestId("main-window")).toHaveAttribute("data-split-weights", restoredSplitWeights ?? "");
    if (isStackedPaneLayout) {
      await expect(page.getByTestId(tabTestId("pane-a", alphaId))).toHaveAttribute("data-active", "true");
      await expect(page.getByTestId(tabTestId("pane-a", betaId))).toHaveAttribute("data-pinned", "true");
      await expect(page.getByTestId(tabTestId("pane-b", alphaId))).toHaveAttribute("data-active", "true");
      await expect(page.getByTestId("pane-pane-b").getByTestId("rich-document-title")).toContainText(
        "Alpha Rich Workbench Doc",
      );
    } else {
      await expect(page.getByTestId(tabTestId("pane-a", alphaId))).toHaveAttribute("data-active", "true");
      await expect(page.getByTestId(tabTestId("pane-b", betaId))).toHaveAttribute("data-active", "true");
      await expect(page.getByTestId(tabTestId("pane-b", betaId))).toHaveAttribute("data-pinned", "true");
      await expect(page.getByTestId("pane-pane-b").getByTestId("rich-document-title")).toContainText(
        "Beta Rich Workbench Doc",
      );
    }
    await page.screenshot({ path: testInfo.outputPath("mt246-workbench-restored-layout.png"), fullPage: true });

    expect(externalRequests, `external requests attempted: ${externalRequests.join(", ")}`).toEqual([]);
  });
});
