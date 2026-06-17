import { FormEvent, useEffect, useRef, useState } from "react";
import { createConfiguredDiffEditor, monaco } from "../lib/monaco/setup";
import { parseUnifiedPatchSides } from "../lib/source_control/unified_patch";
import "monaco-editor/min/vs/editor/editor.main.css";
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
  type SourceControlDiff,
  type SourceControlDiffScope,
  type SourceControlLog,
  type SourceControlReceipt,
  type SourceControlStatus,
  type SourceControlStatusEntry,
} from "../lib/api";

type Props = {
  initialRepoPath?: string;
  initialHistoryDocumentId?: string;
};

function errorMessage(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

function pathTestIdPart(path: string): string {
  return path.replace(/[^a-zA-Z0-9]+/g, "-").replace(/^-+|-+$/g, "") || "root";
}

function shortId(id: string | null | undefined): string {
  return id ? id.slice(0, 12) : "none";
}

function statusLabel(entry: SourceControlStatusEntry): string {
  const index = entry.index ? `index ${entry.index}` : null;
  const worktree = entry.worktree ? `worktree ${entry.worktree}` : null;
  return [index, worktree].filter(Boolean).join(" / ") || "clean";
}

function receiptLabel(receipt: SourceControlReceipt): string {
  const target = receipt.paths.length > 0 ? receipt.paths.join(", ") : "repo";
  return `${receipt.operation}: ${target} (receipt ${shortId(receipt.event_ledger_event_id)})`;
}

function selectedEntry(status: SourceControlStatus | null, path: string | null): SourceControlStatusEntry | null {
  if (!status || !path) return null;
  return status.entries.find((entry) => entry.path === path) ?? null;
}

const MONACO_LANGUAGE_BY_EXTENSION: Record<string, string> = {
  ts: "typescript",
  tsx: "typescript",
  js: "javascript",
  jsx: "javascript",
  rs: "rust",
  json: "json",
  css: "css",
  html: "html",
  md: "markdown",
  py: "python",
  toml: "ini",
  yml: "yaml",
  yaml: "yaml",
  sql: "sql",
  sh: "shell",
};

function monacoLanguageForPath(path: string | null): string {
  if (!path) return "plaintext";
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return MONACO_LANGUAGE_BY_EXTENSION[ext] ?? "plaintext";
}

type MonacoDiffViewProps = {
  patch: string;
  language: string;
};

/**
 * Renders a per-file git patch in a REAL Monaco diff editor (the MT-247 diff
 * seam, `createConfiguredDiffEditor`). The unified patch is reconstructed into
 * original/modified sides; the editor is read-only. An accessible `<pre>`
 * fallback always carries the raw patch text alongside (see the panel body).
 */
function MonacoDiffView({ patch, language }: MonacoDiffViewProps) {
  const host = useRef<HTMLDivElement>(null);
  const [degraded, setDegraded] = useState(false);

  useEffect(() => {
    if (!host.current) return;
    const sides = parseUnifiedPatchSides(patch);
    let original: monaco.editor.ITextModel | null = null;
    let modified: monaco.editor.ITextModel | null = null;
    let editor: monaco.editor.IStandaloneDiffEditor | null = null;
    try {
      original = monaco.editor.createModel(sides.original, language);
      modified = monaco.editor.createModel(sides.modified, language);
      editor = createConfiguredDiffEditor({
        container: host.current,
        renderSideBySide: true,
        originalEditable: false,
        readOnly: true,
        minimap: { enabled: false },
        fontSize: 12,
      });
      editor.setModel({ original, modified });
    } catch {
      // createConfiguredDiffEditor already reported a typed dependency failure
      // (and headless/no-canvas environments cannot mount Monaco). Degrade to
      // the always-present raw-patch <pre> fallback rather than crashing. The
      // flip is deferred off the effect body so it never cascades a render
      // synchronously (react-hooks/set-state-in-effect).
      editor?.dispose();
      original?.dispose();
      modified?.dispose();
      queueMicrotask(() => setDegraded(true));
      return;
    }
    return () => {
      editor?.dispose();
      original?.dispose();
      modified?.dispose();
    };
    // Re-create on patch/language change so the diff always reflects selection.
  }, [patch, language]);

  return (
    <div
      ref={host}
      className="source-control-panel__diff-monaco"
      data-testid="source-control.diff-monaco"
      data-degraded={degraded ? "true" : "false"}
      style={{ height: 320 }}
    />
  );
}

export function SourceControlPanel({ initialRepoPath = "", initialHistoryDocumentId = "" }: Props) {
  const [repoPath, setRepoPath] = useState(initialRepoPath);
  const [status, setStatus] = useState<SourceControlStatus | null>(null);
  const [branches, setBranches] = useState<SourceControlBranch[]>([]);
  const [log, setLog] = useState<SourceControlLog | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [diffScope, setDiffScope] = useState<SourceControlDiffScope>("worktree");
  const [diff, setDiff] = useState<SourceControlDiff | null>(null);
  const [blame, setBlame] = useState<SourceControlBlame | null>(null);
  const [statusLoading, setStatusLoading] = useState(false);
  const [diffLoading, setDiffLoading] = useState(false);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [actionBusy, setActionBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [actionStatus, setActionStatus] = useState<string | null>(null);
  const [commitMessage, setCommitMessage] = useState("");
  const [branchName, setBranchName] = useState("");
  const [discardConfirmed, setDiscardConfirmed] = useState(false);
  const [historyDocumentId, setHistoryDocumentId] = useState(initialHistoryDocumentId);
  const [documentHistory, setDocumentHistory] = useState<RichDocHistory | null>(null);

  const loadDiffFor = async (path: string, scope: SourceControlDiffScope, repo = repoPath.trim()) => {
    if (!repo) return;
    setDiffLoading(true);
    setError(null);
    try {
      setDiff(await getSourceControlDiff(repo, path, scope));
    } catch (err) {
      setDiff(null);
      setError(errorMessage(err, "Failed to load source-control diff"));
    } finally {
      setDiffLoading(false);
    }
  };

  const loadStatus = async (event?: FormEvent) => {
    event?.preventDefault();
    const nextRepoPath = repoPath.trim();
    if (!nextRepoPath) {
      setError("Enter a repository path.");
      return;
    }

    setStatusLoading(true);
    setError(null);
    setActionStatus(null);
    try {
      const [nextStatus, nextBranches, nextLog] = await Promise.all([
        getSourceControlStatus(nextRepoPath),
        listSourceControlBranches(nextRepoPath),
        getSourceControlLog(nextRepoPath, 10),
      ]);
      const nextSelectedPath = nextStatus.entries[0]?.path ?? null;
      setStatus(nextStatus);
      setBranches(nextBranches);
      setLog(nextLog);
      setSelectedPath(nextSelectedPath);
      setDiff(null);
      setBlame(null);
      if (nextSelectedPath) {
        await loadDiffFor(nextSelectedPath, diffScope, nextRepoPath);
      }
    } catch (err) {
      setStatus(null);
      setBranches([]);
      setLog(null);
      setSelectedPath(null);
      setDiff(null);
      setBlame(null);
      setError(errorMessage(err, "Failed to load source-control status"));
    } finally {
      setStatusLoading(false);
    }
  };

  const refreshRepoSummary = async (repo: string) => {
    const [nextStatus, nextBranches, nextLog] = await Promise.all([
      getSourceControlStatus(repo),
      listSourceControlBranches(repo),
      getSourceControlLog(repo, 10),
    ]);
    setStatus(nextStatus);
    setBranches(nextBranches);
    setLog(nextLog);
    if (!nextStatus.entries.some((entry) => entry.path === selectedPath)) {
      setSelectedPath(nextStatus.entries[0]?.path ?? null);
      setDiff(null);
      setBlame(null);
    }
  };

  const selectPath = (path: string) => {
    setSelectedPath(path);
    setBlame(null);
    void loadDiffFor(path, diffScope);
  };

  const changeDiffScope = (scope: SourceControlDiffScope) => {
    setDiffScope(scope);
    if (selectedPath) {
      void loadDiffFor(selectedPath, scope);
    }
  };

  const runStage = async () => {
    if (!selectedPath) return;
    setActionBusy(true);
    setActionStatus(`stage pending: ${selectedPath}`);
    setError(null);
    try {
      const receipt = await stageSourceControlPaths(repoPath.trim(), [selectedPath]);
      setActionStatus(receiptLabel(receipt));
    } catch (err) {
      setError(errorMessage(err, "Failed to stage selected path"));
    } finally {
      setActionBusy(false);
    }
  };

  const runUnstage = async () => {
    if (!selectedPath) return;
    setActionBusy(true);
    setActionStatus(`unstage pending: ${selectedPath}`);
    setError(null);
    try {
      const receipt = await unstageSourceControlPaths(repoPath.trim(), [selectedPath]);
      setActionStatus(receiptLabel(receipt));
    } catch (err) {
      setError(errorMessage(err, "Failed to unstage selected path"));
    } finally {
      setActionBusy(false);
    }
  };

  const runDiscard = async () => {
    if (!selectedPath || !discardConfirmed) return;
    setActionBusy(true);
    setActionStatus(`discard pending: ${selectedPath}`);
    setError(null);
    try {
      const receipt = await discardSourceControlPaths(repoPath.trim(), [selectedPath], true);
      setActionStatus(receiptLabel(receipt));
      setDiscardConfirmed(false);
    } catch (err) {
      setError(errorMessage(err, "Failed to discard selected path"));
    } finally {
      setActionBusy(false);
    }
  };

  const runCommit = async (event: FormEvent) => {
    event.preventDefault();
    const repo = repoPath.trim();
    const message = commitMessage.trim();
    if (!message) {
      setError("Enter a commit message.");
      return;
    }
    setActionBusy(true);
    setActionStatus("commit pending");
    setError(null);
    try {
      const commit = await commitSourceControl(repo, message);
      setActionStatus(
        `commit ${commit.id.slice(0, 12)}: ${commit.message} (receipt ${shortId(commit.event_ledger_event_id)})`,
      );
      setCommitMessage("");
      setLog(await getSourceControlLog(repo, 10));
    } catch (err) {
      setError(errorMessage(err, "Failed to commit staged changes"));
    } finally {
      setActionBusy(false);
    }
  };

  const runSwitchBranch = async (name: string) => {
    const repo = repoPath.trim();
    if (!repo) return;
    setActionBusy(true);
    setActionStatus(`switch pending: ${name}`);
    setError(null);
    try {
      const receipt = await switchSourceControlBranch(repo, name);
      setActionStatus(receiptLabel(receipt));
      await refreshRepoSummary(repo);
    } catch (err) {
      setError(errorMessage(err, "Failed to switch branch"));
    } finally {
      setActionBusy(false);
    }
  };

  const runCreateBranch = async (event: FormEvent) => {
    event.preventDefault();
    const repo = repoPath.trim();
    const name = branchName.trim();
    if (!repo || !name) return;
    setActionBusy(true);
    setActionStatus(`create branch pending: ${name}`);
    setError(null);
    try {
      const receipt = await createSourceControlBranch(repo, name);
      setActionStatus(receiptLabel(receipt));
      setBranchName("");
      setBranches(await listSourceControlBranches(repo));
    } catch (err) {
      setError(errorMessage(err, "Failed to create branch"));
    } finally {
      setActionBusy(false);
    }
  };

  const runBlame = async () => {
    const repo = repoPath.trim();
    if (!repo || !selectedPath) return;
    setActionBusy(true);
    setActionStatus(`blame pending: ${selectedPath}`);
    setError(null);
    try {
      const nextBlame = await getSourceControlBlame(repo, selectedPath);
      setBlame(nextBlame);
      setActionStatus(`blame loaded: ${nextBlame.path}`);
    } catch (err) {
      setBlame(null);
      setError(errorMessage(err, "Failed to load line blame"));
    } finally {
      setActionBusy(false);
    }
  };

  const runHistory = async (event: FormEvent) => {
    event.preventDefault();
    const documentId = historyDocumentId.trim();
    if (!documentId) {
      setError("Enter a rich document id.");
      return;
    }
    setHistoryLoading(true);
    setError(null);
    try {
      setDocumentHistory(await loadRichDocumentHistory(documentId));
    } catch (err) {
      setDocumentHistory(null);
      setError(errorMessage(err, "Failed to load Handshake document history"));
    } finally {
      setHistoryLoading(false);
    }
  };

  const activeEntry = selectedEntry(status, selectedPath);
  const canUseRepo = repoPath.trim().length > 0;
  const canAct = Boolean(selectedPath && canUseRepo && !actionBusy);
  const canCommit = canUseRepo && commitMessage.trim().length > 0 && !actionBusy;
  const canCreateBranch = canUseRepo && branchName.trim().length > 0 && !actionBusy;

  return (
    <section
      className="content-card source-control-panel"
      data-testid="source-control-panel"
      data-stable-id="source-control-panel"
    >
      <header className="source-control-panel__header">
        <div>
          <p className="app-eyebrow">Source Control</p>
          <h2>Source Control</h2>
          <p className="muted">Local git status, diffs, branches, log, blame, and Handshake history.</p>
        </div>
        <span className="kernel-dcc__badge" data-testid="source-control.branch">
          {status?.branch ?? "No branch loaded"}
        </span>
      </header>

      <form className="source-control-panel__repo" onSubmit={(event) => void loadStatus(event)}>
        <label htmlFor="source-control-repo-path">Repository path</label>
        <div className="filter-actions">
          <input
            id="source-control-repo-path"
            value={repoPath}
            onChange={(event) => setRepoPath(event.target.value)}
            placeholder="Path to local git repository"
            data-testid="source-control.repo-path"
          />
          <button type="submit" disabled={statusLoading} data-testid="source-control.load">
            {statusLoading ? "Loading..." : "Load status"}
          </button>
        </div>
      </form>

      {error ? (
        <p className="source-control-panel__error" data-testid="source-control.error">
          {error}
        </p>
      ) : null}
      {actionStatus ? (
        <p className="source-control-panel__status" data-testid="source-control.action-status">
          {actionStatus}
        </p>
      ) : null}

      <div className="source-control-panel__layout">
        <section className="source-control-panel__changes" aria-label="Source-control changes">
          <div className="source-control-panel__section-header">
            <h3>Changes</h3>
            <span className="muted">{status ? `${status.entries.length} paths` : "Not loaded"}</span>
          </div>
          {!status ? <p className="muted">Load a repository to inspect changes.</p> : null}
          {status?.entries.length === 0 ? <p className="muted">No local changes.</p> : null}
          {status?.entries.length ? (
            <ul className="source-control-panel__change-list">
              {status.entries.map((entry) => {
                const active = entry.path === selectedPath;
                return (
                  <li key={entry.path}>
                    <button
                      type="button"
                      className={
                        active
                          ? "source-control-panel__change source-control-panel__change--active"
                          : "source-control-panel__change"
                      }
                      onClick={() => selectPath(entry.path)}
                      data-testid={`source-control.status.${pathTestIdPart(entry.path)}`}
                      data-selected={active ? "true" : "false"}
                    >
                      <span>{entry.path}</span>
                      <span className="muted">{statusLabel(entry)}</span>
                    </button>
                  </li>
                );
              })}
            </ul>
          ) : null}
        </section>

        <section className="source-control-panel__detail" aria-label="Selected source-control diff">
          <div className="source-control-panel__section-header">
            <div>
              <h3>{selectedPath ?? "No path selected"}</h3>
              <p className="muted">{activeEntry ? statusLabel(activeEntry) : "Select a path to inspect its diff."}</p>
            </div>
            <label className="source-control-panel__scope">
              <span>Diff</span>
              <select
                value={diffScope}
                onChange={(event) => changeDiffScope(event.target.value as SourceControlDiffScope)}
                data-testid="source-control.diff-scope"
              >
                <option value="worktree">Worktree</option>
                <option value="staged">Staged</option>
              </select>
            </label>
          </div>

          <div className="source-control-panel__actions">
            <button type="button" disabled={!canAct} onClick={() => void runStage()} data-testid="source-control.stage">
              Stage
            </button>
            <button
              type="button"
              disabled={!canAct}
              onClick={() => void runUnstage()}
              data-testid="source-control.unstage"
            >
              Unstage
            </button>
            <button
              type="button"
              disabled={!canAct}
              onClick={() => void runBlame()}
              data-testid="source-control.load-blame"
            >
              Load blame
            </button>
            <label className="source-control-panel__discard-confirm">
              <input
                type="checkbox"
                checked={discardConfirmed}
                onChange={(event) => setDiscardConfirmed(event.target.checked)}
                data-testid="source-control.discard-confirm"
              />
              Confirm discard
            </label>
            <button
              type="button"
              disabled={!canAct || !discardConfirmed}
              onClick={() => void runDiscard()}
              data-testid="source-control.discard"
            >
              Discard
            </button>
          </div>

          <form className="source-control-panel__commit" onSubmit={(event) => void runCommit(event)}>
            <label htmlFor="source-control-commit-message">Commit message</label>
            <div className="filter-actions">
              <input
                id="source-control-commit-message"
                value={commitMessage}
                onChange={(event) => setCommitMessage(event.target.value)}
                placeholder="Commit staged changes"
                data-testid="source-control.commit-message"
              />
              <button type="submit" disabled={!canCommit} data-testid="source-control.commit">
                Commit
              </button>
            </div>
          </form>

          <div className="source-control-panel__diff-region" data-testid="source-control.diff-region">
            {diffLoading ? (
              <p className="muted" data-testid="source-control.diff-loading">
                Loading diff...
              </p>
            ) : diff?.patch ? (
              <MonacoDiffView patch={diff.patch} language={monacoLanguageForPath(selectedPath)} />
            ) : (
              <p className="muted">No diff loaded.</p>
            )}
            <details className="source-control-panel__diff-raw">
              <summary>Raw patch</summary>
              <pre className="source-control-panel__diff" data-testid="source-control.diff">
                {diffLoading ? "Loading diff..." : diff?.patch || "No diff loaded."}
              </pre>
            </details>
          </div>

          {blame ? (
            <section className="source-control-panel__blame" aria-label="Line blame" data-testid="source-control.blame">
              <div className="source-control-panel__section-header">
                <h3>Line blame</h3>
                <span className="muted">{blame.lines.length} lines</span>
              </div>
              <ol className="source-control-panel__plain-list">
                {blame.lines.map((line) => (
                  <li key={`${line.line_number}-${line.commit_id}`}>
                    <span className="source-control-panel__mono">{line.line_number}</span>
                    <span>{line.content}</span>
                    <span className="muted">
                      {shortId(line.commit_id)} {line.author}
                    </span>
                  </li>
                ))}
              </ol>
            </section>
          ) : null}
        </section>

        <aside className="source-control-panel__repo-history" aria-label="Repository and Handshake history">
          <section className="source-control-panel__side-section" aria-label="Git branches">
            <div className="source-control-panel__section-header">
              <h3>Branches</h3>
              <span className="muted">{branches.length ? `${branches.length} branches` : "Not loaded"}</span>
            </div>
            {branches.length ? (
              <ul className="source-control-panel__plain-list">
                {branches.map((branch) => (
                  <li key={branch.name}>
                    <button
                      type="button"
                      disabled={branch.current || !canUseRepo || actionBusy}
                      onClick={() => void runSwitchBranch(branch.name)}
                      data-testid={`source-control.branch.${pathTestIdPart(branch.name)}`}
                    >
                      {branch.current ? "Current" : "Switch"}
                    </button>
                    <span>{branch.name}</span>
                    <span className="muted">{shortId(branch.commit_id)}</span>
                  </li>
                ))}
              </ul>
            ) : (
              <p className="muted">Load a repository to list local branches.</p>
            )}
            <form className="source-control-panel__branch-create" onSubmit={(event) => void runCreateBranch(event)}>
              <label htmlFor="source-control-new-branch">New branch</label>
              <div className="filter-actions">
                <input
                  id="source-control-new-branch"
                  value={branchName}
                  onChange={(event) => setBranchName(event.target.value)}
                  placeholder="feature/name"
                  data-testid="source-control.new-branch"
                />
                <button type="submit" disabled={!canCreateBranch} data-testid="source-control.create-branch">
                  Create
                </button>
              </div>
            </form>
          </section>

          <section className="source-control-panel__side-section" aria-label="Git log">
            <div className="source-control-panel__section-header">
              <h3>Git log</h3>
              <span className="muted">{log ? `${log.entries.length} commits` : "Not loaded"}</span>
            </div>
            <ol className="source-control-panel__plain-list" data-testid="source-control.log">
              {log?.entries.length ? (
                log.entries.map((entry) => (
                  <li key={entry.id}>
                    <span className="source-control-panel__mono">{shortId(entry.id)}</span>
                    <span>{entry.message}</span>
                    <span className="muted">{entry.author}</span>
                  </li>
                ))
              ) : (
                <li className="muted">No commits loaded.</li>
              )}
            </ol>
          </section>

          <section className="source-control-panel__side-section" aria-label="Handshake document history">
            <div className="source-control-panel__section-header">
              <h3>Handshake history</h3>
              <span className="muted">
                {documentHistory ? `v${documentHistory.current_version}` : "Not loaded"}
              </span>
            </div>
            <form className="source-control-panel__handshake-history-form" onSubmit={(event) => void runHistory(event)}>
              <label htmlFor="source-control-history-document-id">Rich document id</label>
              <div className="filter-actions">
                <input
                  id="source-control-history-document-id"
                  value={historyDocumentId}
                  onChange={(event) => setHistoryDocumentId(event.target.value)}
                  placeholder="KRD-..."
                  data-testid="source-control.history-document-id"
                />
                <button type="submit" disabled={historyLoading} data-testid="source-control.load-history">
                  {historyLoading ? "Loading..." : "Load"}
                </button>
              </div>
            </form>
            <div data-testid="source-control.handshake-history">
              {documentHistory ? (
                <>
                  <p className="muted">
                    {documentHistory.authority_label} owner {documentHistory.owner_actor_id ?? "unknown"}
                  </p>
                  <ol className="source-control-panel__plain-list">
                    {documentHistory.versions.map((version) => (
                      <li key={`${version.rich_document_id}-${version.doc_version}`}>
                        <span className="source-control-panel__mono">v{version.doc_version}</span>
                        <span>{shortId(version.content_sha256)}</span>
                        <span className="muted">{version.promotion_receipt_event_id ?? "no receipt"}</span>
                      </li>
                    ))}
                  </ol>
                </>
              ) : (
                <p className="muted">Load a RichDocument id to compare EventLedger history beside git history.</p>
              )}
            </div>
          </section>
        </aside>
      </div>
    </section>
  );
}
