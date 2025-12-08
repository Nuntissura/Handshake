import { render, screen, waitFor, fireEvent } from "@testing-library/react";
import { vi } from "vitest";
import { DocumentView } from "./DocumentView";

vi.mock("../state/debugEvents", () => ({
  logEvent: vi.fn(),
}));

vi.mock("./TiptapEditor", () => ({
  TiptapEditor: ({ initialContent, onChange }: { initialContent: any; onChange: (doc: any) => void }) => (
    <div>
      <div
        contentEditable
        data-testid="tiptap-editor"
        onInput={(e) =>
          onChange({
            type: "doc",
            content: [{ type: "text", text: (e.target as HTMLElement).textContent ?? "" }],
          })
        }
      >
        {initialContent?.content?.[0]?.text ?? "Hello"}
      </div>
    </div>
  ),
}));

vi.mock("../lib/api", () => {
  const updateDocumentBlocks = vi.fn(async (_id: string, blocks: any[]) => blocks);
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
  return { getDocument, updateDocumentBlocks };
});

describe("DocumentView", () => {
  it("saves edited text via updateDocumentBlocks", async () => {
    render(<DocumentView documentId="doc-1" />);

    // Wait for the editor to render the initial text.
    const editor = await screen.findByTestId("tiptap-editor");
    fireEvent.input(editor, { target: { textContent: "Hello world" } });

    fireEvent.click(screen.getByRole("button", { name: /save/i }));

    const api = await import("../lib/api");
    const updateDocumentBlocks = (api as { updateDocumentBlocks: ReturnType<typeof vi.fn> }).updateDocumentBlocks;

    await waitFor(() => {
      expect(updateDocumentBlocks).toHaveBeenCalledTimes(1);
      const [_docId, blocks] = updateDocumentBlocks.mock.calls[0] as [string, Array<{ raw_content: string }>];
      expect(blocks[0]?.raw_content).toBe("Hello world");
    });
  });
});
