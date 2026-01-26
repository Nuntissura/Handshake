import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { act } from "react";
import { vi } from "vitest";
import { DocumentView } from "./DocumentView";
import type { JSONContent } from "@tiptap/core";
import type { BlockInput } from "../lib/api";

vi.mock("../state/debugEvents", () => ({
  logEvent: vi.fn(),
}));

vi.mock("../state/aiJobs", () => ({
  addJob: vi.fn(),
}));

vi.mock("./TiptapEditor", () => ({
  TiptapEditor: ({
    initialContent,
    onChange,
  }: {
    initialContent: JSONContent | null;
    onChange: (doc: JSONContent | null) => void;
  }) => (
    <div>
      <textarea
        data-testid="tiptap-editor"
        defaultValue={initialContent?.content?.[0]?.text ?? "Hello"}
        onChange={(e) =>
          onChange({
            type: "doc",
            content: [{ type: "text", text: (e.target as HTMLTextAreaElement).value }],
          })
        }
      />
    </div>
  ),
}));

vi.mock("../lib/api", () => {
  const deleteDocument = vi.fn(async () => undefined);
  const updateDocumentBlocks = vi.fn(async (_id: string, blocks: BlockInput[]) => blocks);
  const createJob = vi.fn(async () => ({ job_id: "job-1" }));
  const createDiagnostic = vi.fn(async () => ({ id: "diag-1" }));
  const getDocument = vi.fn(async () => ({
    id: "doc-1",
    workspace_id: "w1",
    title: "Doc 1",
    created_at: "2025-01-01T00:00:00Z",
    updated_at: "2025-01-01T00:00:00Z",
    blocks: [
      {
        id: "b1",
        kind: "paragraph",
        sequence: 0,
        raw_content: "Hello",
        display_content: "Hello",
        derived_content: null,
      },
    ],
  }));
  return { getDocument, updateDocumentBlocks, deleteDocument, createJob, createDiagnostic };
});

describe("DocumentView", () => {
  it("saves edited text via updateDocumentBlocks", async () => {
    await act(async () => {
      render(<DocumentView documentId="doc-1" onDeleted={() => {}} />);
    });

    // Wait for the editor to render the initial text.
    const editor = await screen.findByTestId("tiptap-editor");
    await act(async () => {
      fireEvent.change(editor, { target: { value: "Hello world" } });
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /save/i }));
    });

    const api = await import("../lib/api");
    const updateDocumentBlocks = vi.mocked(api.updateDocumentBlocks);

    await waitFor(() => {
      expect(updateDocumentBlocks).toHaveBeenCalledTimes(1);
      const [docId, blocks] = updateDocumentBlocks.mock.calls[0] as [string, BlockInput[]];
      expect(docId).toBe("doc-1");
      expect(blocks[0]?.raw_content).toBe("Hello world");
    });
  });

  it("creates a doc_summarize job via the command palette with DocsAiJobProfile inputs", async () => {
    await act(async () => {
      render(<DocumentView documentId="doc-1" onDeleted={() => {}} />);
    });

    await screen.findByTestId("tiptap-editor");

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /ai actions/i }));
    });

    const instructionsInput = await screen.findByLabelText(/instructions/i);
    await act(async () => {
      fireEvent.change(instructionsInput, { target: { value: "Focus on action items." } });
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("option", { name: /summarize document/i }));
    });

    const api = await import("../lib/api");
    const createJob = vi.mocked(api.createJob);

    await waitFor(() => {
      expect(createJob).toHaveBeenCalledTimes(1);
      expect(createJob).toHaveBeenCalledWith("doc_summarize", "doc-proto-001", "doc-1", {
        doc_id: "doc-1",
        selection: null,
        layer_scope: "Document",
        instructions: "Focus on action items.",
      });
    });

    const store = await import("../state/aiJobs");
    const addJob = vi.mocked(store.addJob);

    await waitFor(() => {
      expect(addJob).toHaveBeenCalledTimes(1);
      expect(addJob).toHaveBeenCalledWith(expect.objectContaining({ jobId: "job-1", docId: "doc-1" }));
    });
  });
});
