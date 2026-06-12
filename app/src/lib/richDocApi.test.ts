// WP-KERNEL-009 RichDocumentCore (MT-145..MT-160) — rich-document API client
// tests. These assert the client targets the REAL backend authority surface
// (/knowledge/documents) with the backend-navigation identity headers, and that
// the save round-trip sends the optimistic-concurrency expected_version.

import { afterEach, describe, expect, it, vi } from "vitest";
import {
  DEFAULT_RICH_DOC_CONTEXT,
  createRichDocument,
  exportRichDocumentProjection,
  loadRichDocument,
  loadRichDocumentHistory,
  listRichDocumentBacklinks,
  saveRichDocument,
} from "./api";

afterEach(() => {
  vi.unstubAllGlobals();
});

function jsonResponse(body: unknown): Response {
  return {
    ok: true,
    status: 200,
    statusText: "OK",
    headers: new Headers({ "content-type": "application/json" }),
    text: async () => JSON.stringify(body),
  } as Response;
}

const SAMPLE_DOC = {
  rich_document_id: "KRD-00000000000000000000000000000001",
  workspace_id: "ws-1",
  document_id: null,
  title: "Runbook",
  schema_version: "rich_document_v1",
  doc_version: 1,
  content_json: { type: "doc", content: [] },
  content_sha256: "0".repeat(64),
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
};

describe("RichDocument API client (MT-145..MT-160)", () => {
  it("loads a document from the real authority surface with identity headers", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          document: SAMPLE_DOC,
          tree: { schema_version: "rich_document_v1", schema_matches: true, block_ids: [], blocks: [] },
          code_nodes: [],
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const loaded = await loadRichDocument("KRD-00000000000000000000000000000001");
    expect(loaded.document.rich_document_id).toBe("KRD-00000000000000000000000000000001");

    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/documents/KRD-00000000000000000000000000000001",
      expect.objectContaining({
        method: "GET",
        headers: expect.objectContaining({
          "x-hsk-actor-id": DEFAULT_RICH_DOC_CONTEXT.actor_id,
          "x-hsk-kernel-task-run-id": DEFAULT_RICH_DOC_CONTEXT.kernel_task_run_id,
          "x-hsk-session-run-id": DEFAULT_RICH_DOC_CONTEXT.session_run_id,
          "x-hsk-actor-kind": "operator",
        }),
      }),
    );
  });

  it("saves with the optimistic-concurrency expected_version (MT-149)", async () => {
    const saved = { ...SAMPLE_DOC, doc_version: 2 };
    const fetchMock = vi.fn().mockResolvedValueOnce(
      jsonResponse({
        document: saved,
        save_receipt_event_id: "EVT-1",
        backlinks_persisted: 3,
        backlinks_skipped_reason: null,
      }),
    );
    vi.stubGlobal("fetch", fetchMock);

    const result = await saveRichDocument(
      "KRD-00000000000000000000000000000001",
      1,
      { type: "doc", content: [{ type: "paragraph" }] },
    );
    expect(result.document.doc_version).toBe(2);
    expect(result.backlinks_persisted).toBe(3);

    const [, init] = fetchMock.mock.calls[0];
    expect(init.method).toBe("PUT");
    expect(JSON.parse(init.body)).toEqual({
      expected_version: 1,
      content_json: { type: "doc", content: [{ type: "paragraph" }] },
    });
  });

  it("creates a document against the authority POST route (MT-145)", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(jsonResponse({ document: SAMPLE_DOC, save_receipt_event_id: "EVT-1" }));
    vi.stubGlobal("fetch", fetchMock);

    await createRichDocument("ws-1", "Runbook");
    expect(fetchMock).toHaveBeenCalledWith(
      "http://127.0.0.1:37501/knowledge/documents",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("loads history, backlinks, and projections from authority sub-routes", async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValueOnce(
        jsonResponse({
          rich_document_id: SAMPLE_DOC.rich_document_id,
          current_version: 2,
          authority_label: "promoted",
          owner_actor_kind: "operator",
          owner_actor_id: "operator",
          versions: [],
        }),
      )
      .mockResolvedValueOnce(
        jsonResponse({ source_document_id: SAMPLE_DOC.rich_document_id, backlinks: [] }),
      )
      .mockResolvedValueOnce(
        jsonResponse({
          rich_document_id: SAMPLE_DOC.rich_document_id,
          projection: { format: "markdown", content: "# Runbook" },
        }),
      );
    vi.stubGlobal("fetch", fetchMock);

    const history = await loadRichDocumentHistory(SAMPLE_DOC.rich_document_id);
    expect(history.current_version).toBe(2);
    const backlinks = await listRichDocumentBacklinks(SAMPLE_DOC.rich_document_id);
    expect(backlinks.backlinks).toEqual([]);
    const projection = await exportRichDocumentProjection(SAMPLE_DOC.rich_document_id, "markdown");
    expect(projection.projection.content).toBe("# Runbook");

    expect(fetchMock.mock.calls[0][0]).toContain("/history");
    expect(fetchMock.mock.calls[1][0]).toContain("/backlinks");
    expect(fetchMock.mock.calls[2][0]).toContain("/projection?format=markdown");
  });
});
