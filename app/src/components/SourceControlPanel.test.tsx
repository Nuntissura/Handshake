import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SourceControlPanel } from "./SourceControlPanel";
import {
  commitSourceControl,
  createSourceControlBranch,
  discardSourceControlPaths,
  getSourceControlBlame,
  getSourceControlDiff,
  getSourceControlLog,
  getSourceControlStatus,
  listSourceControlBranches,
  loadRichDocumentHistory,
  stageSourceControlPaths,
  switchSourceControlBranch,
  unstageSourceControlPaths,
  type RichDocHistory,
  type SourceControlBlame,
  type SourceControlBranch,
  type SourceControlLog,
  type SourceControlStatus,
} from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual = await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    commitSourceControl: vi.fn(),
    createSourceControlBranch: vi.fn(),
    discardSourceControlPaths: vi.fn(),
    getSourceControlBlame: vi.fn(),
    getSourceControlDiff: vi.fn(),
    getSourceControlLog: vi.fn(),
    getSourceControlStatus: vi.fn(),
    listSourceControlBranches: vi.fn(),
    loadRichDocumentHistory: vi.fn(),
    stageSourceControlPaths: vi.fn(),
    switchSourceControlBranch: vi.fn(),
    unstageSourceControlPaths: vi.fn(),
  };
});

const repoPath = "D:\\Projects\\Handshake Repo";

function statusFixture(): SourceControlStatus {
  return {
    repo_root: repoPath,
    branch: "main",
    entries: [
      { path: "src/main.rs", index: null, worktree: "modified" },
      { path: "scratch/notes.md", index: null, worktree: "untracked" },
    ],
  };
}

function branchFixture(): SourceControlBranch[] {
  return [
    { name: "main", current: true, commit_id: "a".repeat(40) },
    { name: "feature/source-control", current: false, commit_id: "b".repeat(40) },
  ];
}

function logFixture(): SourceControlLog {
  return {
    entries: [
      { id: "a".repeat(40), author: "Ilja", timestamp: 1_700_000_000, message: "source control base" },
    ],
  };
}

function blameFixture(): SourceControlBlame {
  return {
    path: "src/main.rs",
    lines: [
      { line_number: 1, commit_id: "a".repeat(40), author: "Ilja", content: "fn main() {}" },
    ],
  };
}

function historyFixture(): RichDocHistory {
  return {
    rich_document_id: "KRD-00000000000000000000000000000001",
    current_version: 2,
    authority_label: "PostgreSQL/EventLedger",
    owner_actor_kind: "operator",
    owner_actor_id: "ilja",
    versions: [
      {
        rich_document_id: "KRD-00000000000000000000000000000001",
        doc_version: 2,
        schema_version: "rich_document_v1",
        content_sha256: "c".repeat(64),
        crdt_snapshot_id: "CRDT-SNAP-2",
        promotion_receipt_event_id: "EVT-RICH-DOC-2",
        created_at: "2026-06-12T00:00:00Z",
      },
    ],
  };
}

describe("SourceControlPanel", () => {
  beforeEach(() => {
    vi.mocked(commitSourceControl).mockReset();
    vi.mocked(createSourceControlBranch).mockReset();
    vi.mocked(discardSourceControlPaths).mockReset();
    vi.mocked(getSourceControlBlame).mockReset();
    vi.mocked(getSourceControlDiff).mockReset();
    vi.mocked(getSourceControlLog).mockReset();
    vi.mocked(getSourceControlStatus).mockReset();
    vi.mocked(listSourceControlBranches).mockReset();
    vi.mocked(loadRichDocumentHistory).mockReset();
    vi.mocked(stageSourceControlPaths).mockReset();
    vi.mocked(switchSourceControlBranch).mockReset();
    vi.mocked(unstageSourceControlPaths).mockReset();

    vi.mocked(getSourceControlStatus).mockResolvedValue(statusFixture());
    vi.mocked(getSourceControlDiff).mockResolvedValue({
      path: "src/main.rs",
      scope: "worktree",
      patch: "@@ -1 +1\n+changed\n",
    });
    vi.mocked(listSourceControlBranches).mockResolvedValue(branchFixture());
    vi.mocked(getSourceControlLog).mockResolvedValue(logFixture());
    vi.mocked(getSourceControlBlame).mockResolvedValue(blameFixture());
    vi.mocked(loadRichDocumentHistory).mockResolvedValue(historyFixture());
  });

  it("loads repository status and previews the selected worktree diff", async () => {
    render(<SourceControlPanel initialRepoPath={repoPath} />);

    fireEvent.click(screen.getByTestId("source-control.load"));

    await waitFor(() => expect(getSourceControlStatus).toHaveBeenCalledWith(repoPath));
    expect(await screen.findByTestId("source-control.branch")).toHaveTextContent("main");
    expect(screen.getByTestId("source-control.status.src-main-rs")).toHaveTextContent("modified");
    expect(screen.getByTestId("source-control.status.scratch-notes-md")).toHaveTextContent("untracked");
    expect(screen.getByText("feature/source-control")).toBeInTheDocument();
    expect(screen.getByTestId("source-control.log")).toHaveTextContent("source control base");
    await waitFor(() =>
      expect(getSourceControlDiff).toHaveBeenCalledWith(repoPath, "src/main.rs", "worktree"),
    );
    expect(screen.getByTestId("source-control.diff")).toHaveTextContent("+changed");
  });

  it("stages, discards with explicit confirmation, and commits the selected path", async () => {
    vi.mocked(stageSourceControlPaths).mockResolvedValueOnce({
      operation: "stage",
      paths: ["src/main.rs"],
      event_ledger_event_id: "KE-stage-source-control",
    });
    vi.mocked(discardSourceControlPaths).mockResolvedValueOnce({
      operation: "discard",
      paths: ["src/main.rs"],
      event_ledger_event_id: "KE-discard-source-control",
    });
    vi.mocked(commitSourceControl).mockResolvedValueOnce({
      id: "a".repeat(40),
      message: "local commit",
      event_ledger_event_id: "KE-commit-source-control",
    });

    render(<SourceControlPanel initialRepoPath={repoPath} />);

    fireEvent.click(screen.getByTestId("source-control.load"));
    await screen.findByTestId("source-control.status.src-main-rs");

    fireEvent.click(screen.getByTestId("source-control.stage"));
    await waitFor(() =>
      expect(stageSourceControlPaths).toHaveBeenCalledWith(repoPath, ["src/main.rs"]),
    );
    expect(screen.getByTestId("source-control.action-status")).toHaveTextContent(
      "KE-stage-so",
    );

    fireEvent.click(screen.getByTestId("source-control.discard-confirm"));
    fireEvent.click(screen.getByTestId("source-control.discard"));
    await waitFor(() =>
      expect(discardSourceControlPaths).toHaveBeenCalledWith(repoPath, ["src/main.rs"], true),
    );
    expect(screen.getByTestId("source-control.action-status")).toHaveTextContent(
      "KE-discard-",
    );

    expect(screen.getByTestId("source-control.commit")).toBeDisabled();
    fireEvent.change(screen.getByTestId("source-control.commit-message"), {
      target: { value: "local commit" },
    });
    expect(screen.getByTestId("source-control.commit")).toBeEnabled();
    fireEvent.click(screen.getByTestId("source-control.commit"));
    await waitFor(() => expect(commitSourceControl).toHaveBeenCalledWith(repoPath, "local commit"));
    expect(screen.getByTestId("source-control.action-status")).toHaveTextContent(
      "KE-commit-s",
    );
  });

  it("unstages, manages branches, loads blame, and shows Handshake document history", async () => {
    vi.mocked(unstageSourceControlPaths).mockResolvedValueOnce({
      operation: "unstage",
      paths: ["src/main.rs"],
      event_ledger_event_id: "KE-unstage-source-control",
    });
    vi.mocked(switchSourceControlBranch).mockResolvedValueOnce({
      operation: "switch_branch",
      paths: ["feature/source-control"],
      event_ledger_event_id: "KE-switch-source-control",
    });
    vi.mocked(createSourceControlBranch).mockResolvedValueOnce({
      operation: "create_branch",
      paths: ["feature/new-source-control"],
      event_ledger_event_id: "KE-create-source-control",
    });

    render(<SourceControlPanel initialRepoPath={repoPath} />);

    fireEvent.click(screen.getByTestId("source-control.load"));
    await screen.findByTestId("source-control.status.src-main-rs");

    fireEvent.click(screen.getByTestId("source-control.unstage"));
    await waitFor(() =>
      expect(unstageSourceControlPaths).toHaveBeenCalledWith(repoPath, ["src/main.rs"]),
    );
    expect(screen.getByTestId("source-control.action-status")).toHaveTextContent(
      "KE-unstage",
    );

    fireEvent.click(screen.getByTestId("source-control.branch.feature-source-control"));
    await waitFor(() =>
      expect(switchSourceControlBranch).toHaveBeenCalledWith(repoPath, "feature/source-control"),
    );

    fireEvent.change(screen.getByTestId("source-control.new-branch"), {
      target: { value: "feature/new-source-control" },
    });
    fireEvent.click(screen.getByTestId("source-control.create-branch"));
    await waitFor(() =>
      expect(createSourceControlBranch).toHaveBeenCalledWith(repoPath, "feature/new-source-control"),
    );

    fireEvent.click(screen.getByTestId("source-control.load-blame"));
    await waitFor(() => expect(getSourceControlBlame).toHaveBeenCalledWith(repoPath, "src/main.rs"));
    expect(screen.getByTestId("source-control.blame")).toHaveTextContent("fn main() {}");

    fireEvent.change(screen.getByTestId("source-control.history-document-id"), {
      target: { value: "KRD-00000000000000000000000000000001" },
    });
    fireEvent.click(screen.getByTestId("source-control.load-history"));
    await waitFor(() =>
      expect(loadRichDocumentHistory).toHaveBeenCalledWith("KRD-00000000000000000000000000000001"),
    );
    expect(screen.getByTestId("source-control.handshake-history")).toHaveTextContent(
      "EVT-RICH-DOC-2",
    );
  });
});
