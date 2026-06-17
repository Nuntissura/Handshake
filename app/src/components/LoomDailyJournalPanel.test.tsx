import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { openDailyJournal, type LoomBlock } from "../lib/api";
import { LoomDailyJournalPanel } from "./LoomDailyJournalPanel";

vi.mock("../lib/api", () => ({
  openDailyJournal: vi.fn(),
}));

function journalBlock(date: string): LoomBlock {
  return {
    block_id: `journal-${date}`,
    workspace_id: "w1",
    content_type: "journal",
    document_id: null,
    asset_id: null,
    title: `Daily Note ${date}`,
    original_filename: null,
    content_hash: null,
    pinned: false,
    favorite: false,
    pin_order: null,
    journal_date: date,
    created_at: `${date}T00:00:00Z`,
    updated_at: `${date}T00:00:00Z`,
    imported_at: null,
    derived: {
      full_text_index: `# Daily Note ${date}\n\n`,
      backlink_count: 0,
      mention_count: 0,
      tag_count: 0,
      preview_status: "none",
    },
  };
}

describe("LoomDailyJournalPanel", () => {
  beforeEach(() => {
    vi.mocked(openDailyJournal).mockReset();
    vi.mocked(openDailyJournal).mockImplementation(async (_workspaceId, date) => journalBlock(date));
  });

  it("opens today's journal on mount and navigates previous, next, and today by journal date", async () => {
    render(<LoomDailyJournalPanel workspaceId="w1" initialDate="2026-06-16" todayDate="2026-06-16" />);

    expect(await screen.findByTestId("loom-daily-journal-panel.title")).toHaveTextContent(
      "Daily Note 2026-06-16",
    );
    expect(openDailyJournal).toHaveBeenLastCalledWith("w1", "2026-06-16");

    fireEvent.click(screen.getByTestId("loom-daily-journal-panel.prev"));
    await waitFor(() => expect(openDailyJournal).toHaveBeenLastCalledWith("w1", "2026-06-15"));
    expect(await screen.findByTestId("loom-daily-journal-panel.date")).toHaveTextContent("2026-06-15");

    fireEvent.click(screen.getByTestId("loom-daily-journal-panel.next"));
    await waitFor(() => expect(openDailyJournal).toHaveBeenLastCalledWith("w1", "2026-06-16"));
    expect(await screen.findByTestId("loom-daily-journal-panel.date")).toHaveTextContent("2026-06-16");

    fireEvent.click(screen.getByTestId("loom-daily-journal-panel.prev"));
    await waitFor(() => expect(openDailyJournal).toHaveBeenLastCalledWith("w1", "2026-06-15"));
    fireEvent.click(screen.getByTestId("loom-daily-journal-panel.today"));
    await waitFor(() => expect(openDailyJournal).toHaveBeenLastCalledWith("w1", "2026-06-16"));
  });
});
