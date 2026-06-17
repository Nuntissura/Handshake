// WP-KERNEL-009 / MT-255 — backend-backed rich editor draft recovery proof.
//
// Drives the built RichDocumentView harness in a real browser. The only mocked
// surface is the loopback backend API; the editor UI, debounce, recovery banner,
// restore/discard actions, and save path are product code.

import { expect, test, type BrowserContext, type Page, type Route } from "@playwright/test";
import { createServer, type Server } from "node:http";
import { createReadStream, existsSync, statSync } from "node:fs";
import type { AddressInfo } from "node:net";
import path from "node:path";

const repoRoot = path.resolve(__dirname, "..", "..");
const distHarness = path.join(repoRoot, "app", "dist-harness");
const apiOrigin = "http://127.0.0.1:37501";
const documentId = "KRD-00000000000000000000000000000255";

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

type DocJson = {
  type: "doc";
  content: Array<{ type: "paragraph"; content: Array<{ type: "text"; text: string }> }>;
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

function docJson(text: string): DocJson {
  return {
    type: "doc",
    content: [{ type: "paragraph", content: [{ type: "text", text }] }],
  };
}

function collectText(content: DocJson): string {
  return content.content.map((block) => block.content.map((node) => node.text).join("")).join("\n");
}

function documentResponse(content: DocJson, version: number, hash: string) {
  const text = collectText(content);
  return {
    document: {
      rich_document_id: documentId,
      workspace_id: "ws-mt255",
      document_id: null,
      title: "MT-255 Draft Recovery",
      schema_version: "rich_document_v1",
      doc_version: version,
      content_json: content,
      content_sha256: hash,
      crdt_document_id: null,
      crdt_snapshot_id: null,
      promotion_receipt_event_id: null,
      projection_refs: [],
      project_ref: null,
      folder_ref: null,
      authority_label: "promoted",
      owner_actor_kind: "operator",
      owner_actor_id: "operator",
      created_at: "2026-06-12T00:00:00Z",
      updated_at: "2026-06-12T00:00:00Z",
    },
    tree: {
      schema_version: "rich_document_v1",
      schema_matches: true,
      block_ids: ["KBL-MT255-1"],
      blocks: [
        {
          block_id: "KBL-MT255-1",
          kind: "paragraph",
          heading_level: null,
          sequence: 0,
          content: {
            raw: content.content[0],
            derived: { plain_text: text, word_count: text.split(/\s+/).filter(Boolean).length, preview: text },
            display: {},
          },
        },
      ],
    },
    code_nodes: [],
  };
}

async function installBackendMock(context: BrowserContext, harnessBaseUrl: string) {
  let savedContent = docJson("saved head");
  let savedVersion = 1;
  let savedHash = "0".repeat(64);
  let draftContent: DocJson | null = null;
  let draftWrites = 0;
  let saves = 0;
  let clears = 0;
  const externalRequests: string[] = [];

  await context.route("**/*", async (route: Route) => {
    const requestUrl = route.request().url();
    if (requestUrl.startsWith(harnessBaseUrl)) {
      await route.continue();
      return;
    }
    if (!requestUrl.startsWith(apiOrigin)) {
      externalRequests.push(requestUrl);
      await route.abort("connectionfailed");
      return;
    }

    const url = new URL(requestUrl);
    const method = route.request().method();
    const pathName = url.pathname;
    const json = async (body: unknown, status = 200) =>
      route.fulfill({
        status,
        contentType: "application/json",
        body: JSON.stringify(body),
      });

    if (method === "GET" && pathName === `/knowledge/documents/${documentId}`) {
      await json(documentResponse(savedContent, savedVersion, savedHash));
      return;
    }
    if (method === "GET" && pathName === `/knowledge/documents/${documentId}/draft`) {
      await json({
        rich_document_id: documentId,
        current_doc_version: savedVersion,
        current_content_sha256: savedHash,
        draft: draftContent
          ? {
              rich_document_id: documentId,
              workspace_id: "ws-mt255",
              base_doc_version: savedVersion,
              base_content_sha256: savedHash,
              draft_content_json: draftContent,
              draft_content_sha256: "d".repeat(64),
              actor_kind: "operator",
              actor_id: "operator",
              kernel_task_run_id: "KTR-EDITOR-UI",
              session_run_id: "SR-EDITOR-UI",
              created_at: "2026-06-12T00:00:00Z",
              updated_at: "2026-06-12T00:01:00Z",
            }
          : null,
      });
      return;
    }
    if (method === "PUT" && pathName === `/knowledge/documents/${documentId}/draft`) {
      const body = route.request().postDataJSON() as { content_json: DocJson };
      draftContent = body.content_json;
      draftWrites += 1;
      await json({
        rich_document_id: documentId,
        draft: {
          rich_document_id: documentId,
          workspace_id: "ws-mt255",
          base_doc_version: savedVersion,
          base_content_sha256: savedHash,
          draft_content_json: draftContent,
          draft_content_sha256: "d".repeat(64),
          actor_kind: "operator",
          actor_id: "operator",
          kernel_task_run_id: "KTR-EDITOR-UI",
          session_run_id: "SR-EDITOR-UI",
          created_at: "2026-06-12T00:00:00Z",
          updated_at: "2026-06-12T00:01:00Z",
        },
        cleared: false,
        draft_receipt_event_id: "EVT-DRAFT",
        receipt_error: null,
      });
      return;
    }
    if (method === "DELETE" && pathName === `/knowledge/documents/${documentId}/draft`) {
      draftContent = null;
      clears += 1;
      await json({
        rich_document_id: documentId,
        cleared: true,
        clear_receipt_event_id: "EVT-DRAFT-CLEAR",
        receipt_error: null,
      });
      return;
    }
    if (method === "PUT" && pathName === `/knowledge/documents/${documentId}/save`) {
      const body = route.request().postDataJSON() as { content_json: DocJson };
      savedContent = body.content_json;
      savedVersion += 1;
      savedHash = String(savedVersion).repeat(64).slice(0, 64);
      draftContent = null;
      saves += 1;
      await json({
        document: documentResponse(savedContent, savedVersion, savedHash).document,
        save_receipt_event_id: "EVT-SAVE",
        backlinks_persisted: 0,
        backlinks_skipped_reason: null,
      });
      return;
    }
    if (method === "GET" && pathName === `/knowledge/documents/${documentId}/history`) {
      await json({
        rich_document_id: documentId,
        current_version: savedVersion,
        authority_label: "promoted",
        owner_actor_kind: "operator",
        owner_actor_id: "operator",
        versions: [
          {
            rich_document_id: documentId,
            doc_version: savedVersion,
            schema_version: "rich_document_v1",
            content_sha256: savedHash,
            crdt_snapshot_id: null,
            promotion_receipt_event_id: null,
            created_at: "2026-06-12T00:00:00Z",
          },
        ],
      });
      return;
    }
    if (method === "GET" && pathName === `/knowledge/documents/${documentId}/embeds`) {
      await json({ rich_document_id: documentId, embeds: [] });
      return;
    }
    if (method === "GET" && pathName === `/knowledge/documents/${documentId}/embeds/broken`) {
      await json({ rich_document_id: documentId, broken_embeds: [], available_actions: [] });
      return;
    }
    if (method === "GET" && pathName === `/knowledge/documents/${documentId}/backlinks`) {
      await json({ source_document_id: documentId, backlinks: [] });
      return;
    }

    await json({ error: "unhandled", method, pathName }, 404);
  });

  return {
    stats: () => ({
      draftWrites,
      saves,
      clears,
      savedText: collectText(savedContent),
      draftText: draftContent ? collectText(draftContent) : null,
      externalRequests: [...externalRequests],
    }),
  };
}

async function boot(page: Page, baseUrl: string): Promise<void> {
  await page.goto(`${baseUrl}/harness/editor-draft-recovery.html`);
  await expect(page.getByTestId("editor-draft-recovery-harness-root")).toBeVisible();
  await expect(page.getByTestId("rich-document-view")).toBeVisible();
  await expect(page.getByTestId("rich-text-editor")).toBeVisible();
}

async function replaceEditorText(page: Page, text: string): Promise<void> {
  const editor = page.locator("[data-testid='rich-text-editor-surface'] .ProseMirror").first();
  await editor.click();
  await page.keyboard.press(process.platform === "darwin" ? "Meta+A" : "Control+A");
  await page.keyboard.type(text);
}

test.describe("WP-KERNEL-009 MT-255 editor draft recovery (built harness)", () => {
  let server: Server;
  let baseUrl: string;

  test.beforeAll(async () => {
    expect(
      existsSync(path.join(distHarness, "harness", "editor-draft-recovery.html")),
      "dist-harness missing editor-draft-recovery.html; global setup should build it",
    ).toBe(true);
    server = await serveDistHarness();
    const { port } = server.address() as AddressInfo;
    baseUrl = `http://127.0.0.1:${port}`;
  });

  test.afterAll(async () => {
    if (server) await new Promise((resolve) => server.close(resolve));
  });

  test("crash/reopen offers backend draft, restore is exact, save clears, discard leaves saved head untouched", async ({
    page,
    context,
  }) => {
    const backend = await installBackendMock(context, baseUrl);
    const firstDraft = "MT-255 byte-exact crash draft";
    const secondDraft = "MT-255 discard-only draft";

    await boot(page, baseUrl);
    await replaceEditorText(page, firstDraft);
    await expect.poll(() => backend.stats().draftWrites).toBeGreaterThan(0);
    expect(backend.stats().draftText).toBe(firstDraft);

    await page.close({ runBeforeUnload: false });
    const reopened = await context.newPage();
    await boot(reopened, baseUrl);
    const recovery = reopened.getByTestId("rich-document-local-snapshot");
    await expect(recovery).toBeVisible();
    await expect.poll(async () => recovery.getAttribute("data-snapshot-reason")).toBe("draft_recovery");
    await expect(reopened.getByTestId("rich-document-merge-panel")).toBeVisible();

    await reopened.getByTestId("snapshot-restore").click();
    await expect
      .poll(async () =>
        reopened.locator("[data-testid='rich-text-editor-surface'] .ProseMirror").first().textContent(),
      )
      .toBe(firstDraft);
    await reopened.getByTestId("rich-document-save").click();
    await expect.poll(() => backend.stats().saves).toBe(1);
    await expect(reopened.getByTestId("rich-document-local-snapshot")).toHaveCount(0);
    expect(backend.stats().draftText).toBeNull();

    await replaceEditorText(reopened, secondDraft);
    await expect.poll(() => backend.stats().draftText).toBe(secondDraft);
    await reopened.close({ runBeforeUnload: false });

    const discardPage = await context.newPage();
    await boot(discardPage, baseUrl);
    await expect(discardPage.getByTestId("rich-document-local-snapshot")).toBeVisible();
    await discardPage.getByTestId("snapshot-discard").click();
    await expect.poll(() => backend.stats().clears).toBe(1);
    await expect(discardPage.getByTestId("rich-document-local-snapshot")).toHaveCount(0);
    expect(backend.stats().saves).toBe(1);
    expect(backend.stats().savedText).toBe(firstDraft);
    await expect
      .poll(async () =>
        discardPage.locator("[data-testid='rich-text-editor-surface'] .ProseMirror").first().textContent(),
      )
      .toBe(firstDraft);
    expect(backend.stats().externalRequests).toEqual([]);
  });
});
