import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { UserManualPanel } from "./UserManualPanel";
import {
  getUserManualPage,
  listUserManualAccessPoints,
  listUserManualPages,
  searchUserManual,
  type UserManualSearchResponse,
} from "../lib/api";

vi.mock("../lib/api", () => ({
  getUserManualPage: vi.fn(),
  listUserManualAccessPoints: vi.fn(),
  listUserManualPages: vi.fn(),
  searchUserManual: vi.fn(),
}));

const pagesResponse = {
  manual_version: "2.0.0",
  route_namespace: "/usermanual",
  count: 2,
  pages: [
    {
      slug: "manual-toc",
      title: "Manual TOC",
      page_kind: "guide",
      audience: "model",
      manual_version: "2.0.0",
      content_hash: "hash-manual-toc",
      status: "current",
      updated_at: "2026-06-12T00:00:00Z",
    },
    {
      slug: "state-recovery-guide",
      title: "State recovery guide",
      page_kind: "guide",
      audience: "model",
      manual_version: "2.0.0",
      content_hash: "hash-recovery",
      status: "current",
      updated_at: "2026-06-12T00:00:00Z",
    },
  ],
};

const accessPointsResponse = {
  count: 3,
  access_points: [
    {
      access_point_id: "ap.diagnostics.manual_tab",
      host_surface: "diagnostics",
      entry_kind: "panel",
      target_page_slug: "manual-toc",
      ui_wiring_route: "/usermanual/pages/manual-toc",
      stable_element_id: "hs-usermanual-diagnostics-tab",
      note: "Diagnostics manual tab",
      target_resolves: true,
    },
    {
      access_point_id: "ap.command_palette.open_manual",
      host_surface: "command_palette",
      entry_kind: "command",
      target_page_slug: "manual-toc",
      ui_wiring_route: "/usermanual/pages/manual-toc",
      stable_element_id: "hs-usermanual-palette-open",
      note: "Palette open",
      target_resolves: true,
    },
    {
      access_point_id: "ap.command_palette.search_manual",
      host_surface: "command_palette",
      entry_kind: "command",
      target_page_slug: "manual-toc",
      ui_wiring_route: "/usermanual/search",
      stable_element_id: "hs-usermanual-palette-search",
      note: "Palette search",
      target_resolves: true,
    },
  ],
};

const manualTocResponse = {
  page: {
    page_id: "page-manual-toc",
    slug: "manual-toc",
    title: "Manual TOC",
    page_kind: "guide",
    audience: "model",
    manual_version: "2.0.0",
    content_hash: "hash-manual-toc",
    status: "current",
    updated_at: "2026-06-12T00:00:00Z",
  },
  sections: [
    {
      section_id: "section-navigation",
      page_id: "page-manual-toc",
      position: 0,
      section_kind: "body",
      title: "Navigation",
      body_md: "Start with the page index, then open a task-sized guide.",
      body_json: null,
    },
  ],
  anchors: [
    {
      anchor_id: "anchor-pages",
      page_id: "page-manual-toc",
      anchor_kind: "http_route",
      anchor_value: "/usermanual/pages",
      http_method: "GET",
    },
  ],
  bootstrap_receipt_event_id: "evt-manual-opened",
  bootstrap_identity_used: true,
};

describe("UserManualPanel", () => {
  beforeEach(() => {
    vi.mocked(listUserManualPages).mockResolvedValue(pagesResponse);
    vi.mocked(listUserManualAccessPoints).mockResolvedValue(accessPointsResponse);
    vi.mocked(getUserManualPage).mockResolvedValue(manualTocResponse);
    vi.mocked(searchUserManual).mockResolvedValue({
      query: "recovery",
      count: 1,
      results: [
        {
          result_kind: "section",
          result_ref: "section-recovery",
          page_slug: "state-recovery-guide",
          title: "State recovery",
          excerpt: "Recover failed startup state.",
        },
      ],
    });
  });

  it("renders the diagnostics manual surface with stable selectors and fetched index/page data", async () => {
    render(<UserManualPanel />);

    const panel = await screen.findByTestId("usermanual-panel");
    expect(panel).toHaveAttribute("data-stable-id", "hs-usermanual-panel");
    expect(screen.getByTestId("usermanual-layout")).toHaveAttribute(
      "data-manual-layout",
      "index-page-search",
    );
    expect(await screen.findByText("Manual TOC")).toBeInTheDocument();
    expect(await screen.findByText(/Start with the page index/)).toBeInTheDocument();
    expect(screen.getByTestId("usermanual-page-list")).toBeInTheDocument();
    expect(screen.getByTestId("usermanual-search-input")).toBeInTheDocument();
    expect(screen.getByTestId("usermanual-access-points")).toHaveTextContent(
      "hs-usermanual-palette-open",
    );
    expect(
      screen.getByTestId("usermanual-access-point-hs-usermanual-diagnostics-tab"),
    ).toHaveAttribute("data-stable-id", "usermanual-access-point-ap.diagnostics.manual_tab");
    expect(listUserManualAccessPoints).toHaveBeenCalledTimes(1);
    expect(listUserManualPages).toHaveBeenCalledTimes(1);
    expect(getUserManualPage).toHaveBeenCalledWith("manual-toc");
  });

  it("searches the manual and opens a result page through the client layer", async () => {
    render(<UserManualPanel />);
    await screen.findByText("Manual TOC");

    fireEvent.change(screen.getByTestId("usermanual-search-input"), {
      target: { value: "recovery" },
    });
    fireEvent.click(screen.getByTestId("usermanual-search-submit"));

    const results = await screen.findByTestId("usermanual-search-results");
    expect(within(results).getByText("State recovery")).toBeInTheDocument();
    expect(searchUserManual).toHaveBeenCalledWith("recovery", 25);

    fireEvent.click(screen.getByTestId("usermanual-search-result-state-recovery-guide"));
    await waitFor(() => expect(getUserManualPage).toHaveBeenLastCalledWith("state-recovery-guide"));
  });

  it("renders backend tool search hits without treating result_ref as a page slug", async () => {
    const backendToolResult = {
      query: "recovery",
      count: 1,
      results: [
        {
          kind: "tool",
          result_ref: "tool.session_recovery",
          page_slug: null,
          title: "Session recovery tool",
          excerpt: "Run the backend recovery tool from its owning surface.",
        },
      ],
    } as unknown as UserManualSearchResponse;
    vi.mocked(searchUserManual).mockResolvedValueOnce(backendToolResult);

    render(<UserManualPanel />);
    await screen.findByText("Manual TOC");
    vi.mocked(getUserManualPage).mockClear();

    fireEvent.change(screen.getByTestId("usermanual-search-input"), {
      target: { value: "recovery" },
    });
    fireEvent.click(screen.getByTestId("usermanual-search-submit"));

    const result = await screen.findByTestId("usermanual-search-result-tool.session_recovery");
    expect(result.tagName).not.toBe("BUTTON");
    expect(result).toHaveAttribute("data-page-backed", "false");
    expect(result).toHaveTextContent("tool / tool.session_recovery");

    fireEvent.click(result);

    expect(getUserManualPage).not.toHaveBeenCalled();
    expect(screen.getByTestId("usermanual-panel")).toHaveAttribute("data-selected-slug", "manual-toc");
  });

  it("shows a recoverable page error when the backend reports a missing page", async () => {
    render(<UserManualPanel />);
    await screen.findByText("Manual TOC");
    vi.mocked(getUserManualPage).mockRejectedValueOnce(new Error("Request failed: 404 Not Found"));

    fireEvent.click(screen.getByTestId("usermanual-page-link-state-recovery-guide"));

    const error = await screen.findByTestId("usermanual-page-error");
    expect(error).toHaveTextContent("Request failed: 404 Not Found");
    expect(screen.getByTestId("usermanual-page-list")).toBeInTheDocument();
    expect(within(screen.getByTestId("usermanual-page")).queryByText(/Start with the page index/)).not.toBeInTheDocument();
  });

  it("shows a recoverable search error without blanking the manual page", async () => {
    render(<UserManualPanel />);
    await screen.findByText("Manual TOC");
    vi.mocked(searchUserManual).mockRejectedValueOnce(new Error("query parameter 'q' is required"));

    fireEvent.change(screen.getByTestId("usermanual-search-input"), {
      target: { value: "missing" },
    });
    fireEvent.click(screen.getByTestId("usermanual-search-submit"));

    expect(await screen.findByTestId("usermanual-search-error")).toHaveTextContent(
      "query parameter 'q' is required",
    );
    expect(screen.getByText(/Start with the page index/)).toBeInTheDocument();
  });
});
