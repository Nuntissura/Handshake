import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { AiJob, WorkflowRun, createJob, getJob } from "../lib/api";
import { addJob } from "../state/aiJobs";
import {
  MdSessionRecordV0,
  mdOutputRootDirGet,
  mdOutputRootDirSet,
  mdSessionCreate,
  mdSessionExportCookies,
  mdSessionOpen,
  mdSessionsList,
} from "../lib/mediaDownloader";

type MdSourceKind = "youtube" | "instagram" | "forumcrawler" | "videodownloader";
type MdAuthMode = "none" | "stage_session" | "cookie_jar";

type MdCookieImportResultV0 = {
  schema_version: "hsk.media_downloader.cookie_import.result@v0";
  cookie_jar_artifact_ref: unknown;
  stage_session_id?: string | null;
  updated_session: boolean;
};

type MdJobOutputV0 = {
  schema_version: "hsk.media_downloader.result@v0";
  plan: { stable_item_total: number; items: { item_id: string; url_canonical: string; source_kind: MdSourceKind }[] };
  progress: {
    state: string;
    item_done: number;
    item_total: number;
    bytes_downloaded?: number | null;
    bytes_total?: number | null;
    concurrency: number;
  };
  items: {
    item_id: string;
    status: string;
    artifact_handles?: unknown[];
    materialized_paths?: string[];
    error_code?: string | null;
    error_message?: string | null;
  }[];
};

function parseLines(input: string): string[] {
  return input
    .split(/\r?\n/g)
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

function asCookieImportResult(value: unknown): MdCookieImportResultV0 | null {
  if (!value || typeof value !== "object") return null;
  const obj = value as Record<string, unknown>;
  if (obj.schema_version !== "hsk.media_downloader.cookie_import.result@v0") return null;
  if (!("cookie_jar_artifact_ref" in obj)) return null;
  return obj as MdCookieImportResultV0;
}

function asMdJobOutput(value: unknown): MdJobOutputV0 | null {
  if (!value || typeof value !== "object") return null;
  const obj = value as Record<string, unknown>;
  if (obj.schema_version !== "hsk.media_downloader.result@v0") return null;
  return obj as MdJobOutputV0;
}

function sessionHasCookieJar(session: MdSessionRecordV0): boolean {
  return session.cookie_jar_artifact_ref !== null && session.cookie_jar_artifact_ref !== undefined;
}

function defaultStageStartUrl(kind: MdSourceKind, sources: string[]): string {
  if (kind === "instagram") return "https://www.instagram.com/accounts/login/";
  if (kind === "youtube") return "https://www.youtube.com/";
  const first = sources[0];
  if (first) return first;
  return "https://www.example.com/";
}

async function waitForJobDone(jobId: string, timeoutMs = 60_000): Promise<AiJob> {
  const started = Date.now();
  // Poll quickly at first, then settle to 1s.
  let delayMs = 500;
  for (;;) {
    const job = await getJob(jobId);
    if (job.state !== "queued" && job.state !== "running") return job;
    if (Date.now() - started > timeoutMs) {
      throw new Error("Timed out waiting for job completion.");
    }
    await new Promise((r) => window.setTimeout(r, delayMs));
    delayMs = 1000;
  }
}

export function MediaDownloaderView() {
  const [outputRootDir, setOutputRootDir] = useState<string | null>(null);
  const [sessions, setSessions] = useState<MdSessionRecordV0[]>([]);
  const [sessionsLoading, setSessionsLoading] = useState(false);
  const [sessionsError, setSessionsError] = useState<string | null>(null);

  const [sourceKind, setSourceKind] = useState<MdSourceKind>("youtube");
  const [sourcesText, setSourcesText] = useState<string>("");

  const [authMode, setAuthMode] = useState<MdAuthMode>("none");
  const [stageSessionId, setStageSessionId] = useState<string>("");
  const [cookieJarHandle, setCookieJarHandle] = useState<unknown | null>(null);
  const [cookieImportBusy, setCookieImportBusy] = useState(false);

  const [concurrency, setConcurrency] = useState<number>(4);
  const [forumMaxPages, setForumMaxPages] = useState<number>(1500);
  const [allowlistDomainsText, setAllowlistDomainsText] = useState<string>("");

  const [activeJobId, setActiveJobId] = useState<string | null>(null);
  const [activeJob, setActiveJob] = useState<AiJob | null>(null);
  const [activeJobError, setActiveJobError] = useState<string | null>(null);

  const sources = useMemo(() => parseLines(sourcesText), [sourcesText]);
  const allowlistDomains = useMemo(() => parseLines(allowlistDomainsText), [allowlistDomainsText]);

  const refreshOutputRootDir = useCallback(async () => {
    const dir = await mdOutputRootDirGet();
    setOutputRootDir(dir);
  }, []);

  const refreshSessions = useCallback(async () => {
    setSessionsLoading(true);
    setSessionsError(null);
    try {
      const next = await mdSessionsList();
      setSessions(next);
    } catch (err) {
      setSessionsError(err instanceof Error ? err.message : String(err));
    } finally {
      setSessionsLoading(false);
    }
  }, []);

  useEffect(() => {
    refreshOutputRootDir().catch(() => {
      // ignore, shown when trying to change it
    });
    refreshSessions().catch(() => {
      // ignore
    });
  }, [refreshOutputRootDir, refreshSessions]);

  useEffect(() => {
    if (!activeJobId) return;
    let cancelled = false;
    const tick = async () => {
      try {
        const job = await getJob(activeJobId);
        if (!cancelled) setActiveJob(job);
      } catch (err) {
        if (!cancelled) setActiveJobError(err instanceof Error ? err.message : String(err));
      }
    };
    void tick();
    const timer = window.setInterval(() => void tick(), 2000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [activeJobId]);

  const mdOutput = useMemo(() => asMdJobOutput(activeJob?.job_outputs), [activeJob?.job_outputs]);

  const handlePickOutputRoot = useCallback(async () => {
    setActiveJobError(null);
    const picked = await open({ directory: true, multiple: false });
    if (!picked) return;
    const dir = Array.isArray(picked) ? picked[0] : picked;
    if (!dir) return;
    await mdOutputRootDirSet(dir);
    await refreshOutputRootDir();
  }, [refreshOutputRootDir]);

  const handleCreateSession = useCallback(async () => {
    const label = window.prompt("Session label (shown in UI):", "");
    if (label === null) return;
    await mdSessionCreate(label);
    await refreshSessions();
  }, [refreshSessions]);

  const handleOpenSession = useCallback(
    async (sessionId: string) => {
      const url = defaultStageStartUrl(sourceKind, sources);
      await mdSessionOpen(sessionId, url);
    },
    [sourceKind, sources],
  );

  const runCookieImportFromPath = useCallback(
    async (sourcePath: string, stageSessionIdOrNull: string | null, cleanupSource: boolean) => {
      const jobInputs = {
        schema_version: "hsk.media_downloader.cookie_import@v0",
        source_path: sourcePath,
        stage_session_id: stageSessionIdOrNull ?? undefined,
        cleanup_source: cleanupSource,
      };
      const run = await createJob("media_downloader", "hsk.media_downloader.cookie_import.v0", undefined, jobInputs);
      addJob({
        jobId: run.job_id,
        jobKind: "media_downloader",
        protocolId: "hsk.media_downloader.cookie_import.v0",
        docId: "",
        createdAt: Date.now(),
      });
      const job = await waitForJobDone(run.job_id, 120_000);
      if (job.state !== "completed") {
        throw new Error(job.error_message ?? "Cookie import failed.");
      }
      const result = asCookieImportResult(job.job_outputs);
      if (!result) throw new Error("Cookie import output schema mismatch.");
      return result;
    },
    [],
  );

  const handleImportCookieFile = useCallback(async () => {
    setCookieImportBusy(true);
    setActiveJobError(null);
    try {
      const picked = await open({
        multiple: false,
        filters: [{ name: "Cookies", extensions: ["json", "txt"] }],
      });
      if (!picked) return;
      const path = Array.isArray(picked) ? picked[0] : picked;
      if (!path) return;

      const result = await runCookieImportFromPath(path, null, false);
      setCookieJarHandle(result.cookie_jar_artifact_ref);
      setAuthMode("cookie_jar");
    } catch (err) {
      setActiveJobError(err instanceof Error ? err.message : String(err));
    } finally {
      setCookieImportBusy(false);
    }
  }, [runCookieImportFromPath]);

  const handleExportSessionCookies = useCallback(
    async (sessionId: string) => {
      setCookieImportBusy(true);
      setActiveJobError(null);
      try {
        const tempCookiePath = await mdSessionExportCookies(sessionId);
        await runCookieImportFromPath(tempCookiePath, sessionId, true);
        await refreshSessions();
        setAuthMode("stage_session");
        setStageSessionId(sessionId);
      } catch (err) {
        setActiveJobError(err instanceof Error ? err.message : String(err));
      } finally {
        setCookieImportBusy(false);
      }
    },
    [refreshSessions, runCookieImportFromPath],
  );

  const startBatch = useCallback(async () => {
    setActiveJobError(null);

    if (sources.length === 0) {
      setActiveJobError("Add at least one URL.");
      return;
    }

    const controls: Record<string, unknown> = { concurrency };
    if (sourceKind === "forumcrawler") {
      controls.max_pages = forumMaxPages;
      controls.allowlist_domains = allowlistDomains;
    }

    let auth: Record<string, unknown> | undefined;
    if (authMode === "stage_session") {
      if (!stageSessionId) {
        setActiveJobError("Select a Stage Session.");
        return;
      }
      const sess = sessions.find((s) => s.session_id === stageSessionId) ?? null;
      if (!sess) {
        setActiveJobError("Selected Stage Session not found.");
        return;
      }
      if (!sessionHasCookieJar(sess)) {
        setActiveJobError("Selected Stage Session has no exported cookie jar. Use Export Cookies first.");
        return;
      }
      auth = { mode: "stage_session", stage_session_id: stageSessionId };
    } else if (authMode === "cookie_jar") {
      if (!cookieJarHandle) {
        setActiveJobError("Import a cookie file first.");
        return;
      }
      auth = { mode: "cookie_jar", cookie_jar_artifact_ref: cookieJarHandle };
    }

    const jobInputs: Record<string, unknown> = {
      schema_version: "hsk.media_downloader.batch@v0",
      source_kind: sourceKind,
      sources,
      controls,
    };
    if (auth) jobInputs.auth = auth;

    const run: WorkflowRun = await createJob("media_downloader", "hsk.media_downloader.batch.v0", undefined, jobInputs);
    addJob({
      jobId: run.job_id,
      jobKind: "media_downloader",
      protocolId: "hsk.media_downloader.batch.v0",
      docId: "",
      createdAt: Date.now(),
    });
    setActiveJobId(run.job_id);
    setActiveJob(null);
  }, [
    allowlistDomains,
    authMode,
    concurrency,
    cookieJarHandle,
    forumMaxPages,
    sessions,
    sourceKind,
    sources,
    stageSessionId,
  ]);

  const sendControl = useCallback(
    async (action: string, itemId?: string) => {
      if (!activeJobId) return;
      const jobInputs: Record<string, unknown> = {
        schema_version: "hsk.media_downloader.control@v0",
        target_job_id: activeJobId,
        action,
      };
      if (itemId) jobInputs.item_id = itemId;
      await createJob("media_downloader", "hsk.media_downloader.control.v0", undefined, jobInputs);
    },
    [activeJobId],
  );

  const itemsMerged = useMemo(() => {
    const planItems = mdOutput?.plan.items ?? [];
    const results = mdOutput?.items ?? [];
    const resultsById = new Map(results.map((r) => [r.item_id, r]));
    return planItems.map((p) => ({ plan: p, result: resultsById.get(p.item_id) ?? null }));
  }, [mdOutput?.items, mdOutput?.plan.items]);

  const progressText = useMemo(() => {
    if (!mdOutput) return null;
    const done = mdOutput.progress.item_done;
    const total = mdOutput.progress.item_total;
    const state = mdOutput.progress.state;
    return `${state} • ${done}/${total} • concurrency=${mdOutput.progress.concurrency}`;
  }, [mdOutput]);

  return (
    <div className="content-card">
      <div style={{ display: "flex", justifyContent: "space-between", gap: 12, alignItems: "center" }}>
        <div>
          <h2>Media Downloader</h2>
          <p className="muted">
            Downloads are stored as workspace artifacts and materialized under{" "}
            <code>{outputRootDir ? `${outputRootDir}\\media_downloader\\...` : "{OutputRootDir}\\media_downloader\\..."}</code>.
          </p>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button type="button" className="secondary" onClick={() => void refreshSessions()} disabled={sessionsLoading}>
            {sessionsLoading ? "Refreshing..." : "Refresh Sessions"}
          </button>
          <button type="button" className="secondary" onClick={() => void handlePickOutputRoot()}>
            Set OutputRootDir
          </button>
        </div>
      </div>

      {sessionsError && (
        <div style={{ marginTop: 12 }}>
          <p style={{ color: "#b91c1c" }}>{sessionsError}</p>
        </div>
      )}

      {activeJobError && (
        <div style={{ marginTop: 12 }}>
          <p style={{ color: "#b91c1c" }}>{activeJobError}</p>
        </div>
      )}

      <div style={{ marginTop: 12, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 12 }}>
        <div style={{ border: "1px solid #e5e7eb", borderRadius: 12, padding: 12 }}>
          <h3>Job</h3>
          <div style={{ display: "grid", gap: 8 }}>
            <label>
              <span className="muted small">Mode</span>
              <select value={sourceKind} onChange={(e) => setSourceKind(e.target.value as MdSourceKind)}>
                <option value="youtube">YouTube batch archive</option>
                <option value="instagram">Instagram batch archive</option>
                <option value="forumcrawler">Forum/blog topic image crawl</option>
                <option value="videodownloader">Generic video downloader</option>
              </select>
            </label>

            <label>
              <span className="muted small">URLs (one per line)</span>
              <textarea
                value={sourcesText}
                onChange={(e) => setSourcesText(e.target.value)}
                rows={8}
                placeholder="https://..."
              />
            </label>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <label>
                <span className="muted small">Concurrency (1..16)</span>
                <input
                  type="number"
                  min={1}
                  max={16}
                  value={concurrency}
                  onChange={(e) => setConcurrency(Number(e.target.value))}
                />
              </label>
              {sourceKind === "forumcrawler" && (
                <label>
                  <span className="muted small">Max pages (default 1500, cap 5000)</span>
                  <input
                    type="number"
                    min={1}
                    max={5000}
                    value={forumMaxPages}
                    onChange={(e) => setForumMaxPages(Number(e.target.value))}
                  />
                </label>
              )}
            </div>

            {sourceKind === "forumcrawler" && (
              <label>
                <span className="muted small">Allowlist domains (optional; one per line)</span>
                <textarea
                  value={allowlistDomainsText}
                  onChange={(e) => setAllowlistDomainsText(e.target.value)}
                  rows={4}
                  placeholder="cdn.example.com"
                />
              </label>
            )}

            <button type="button" onClick={() => void startBatch()}>
              Start download job
            </button>
          </div>
        </div>

        <div style={{ border: "1px solid #e5e7eb", borderRadius: 12, padding: 12 }}>
          <h3>Auth (optional)</h3>

          <div style={{ display: "grid", gap: 8 }}>
            <label>
              <span className="muted small">Mode</span>
              <select value={authMode} onChange={(e) => setAuthMode(e.target.value as MdAuthMode)}>
                <option value="none">No account</option>
                <option value="stage_session">Stage Session</option>
                <option value="cookie_jar">Import cookies file</option>
              </select>
            </label>

            {authMode === "stage_session" && (
              <label>
                <span className="muted small">Stage Session</span>
                <select value={stageSessionId} onChange={(e) => setStageSessionId(e.target.value)}>
                  <option value="">Select…</option>
                  {sessions
                    .filter((s) => s.kind === "stage_session")
                    .map((s) => (
                      <option key={s.session_id} value={s.session_id}>
                        {s.label} {sessionHasCookieJar(s) ? "(cookies ready)" : "(no cookies)"}
                      </option>
                    ))}
                </select>
              </label>
            )}

            {authMode === "cookie_jar" && (
              <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                <button type="button" className="secondary" onClick={() => void handleImportCookieFile()} disabled={cookieImportBusy}>
                  {cookieImportBusy ? "Importing..." : "Import cookies"}
                </button>
                <span className="muted small">{cookieJarHandle ? "Cookie jar imported." : "No cookie jar imported yet."}</span>
              </div>
            )}
          </div>

          <div style={{ marginTop: 12 }}>
            <h4>Session Manager</h4>
            <p className="muted small">
              Create a Stage Session, log in on the site-owned page, then export cookies to use with downloads.
            </p>

            <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
              <button type="button" className="secondary" onClick={() => void handleCreateSession()}>
                New Stage Session
              </button>
            </div>

            {sessions.filter((s) => s.kind === "stage_session").length === 0 ? (
              <p className="muted">No Stage Sessions yet.</p>
            ) : (
              <ul style={{ listStyle: "none", padding: 0, margin: 0, display: "grid", gap: 8 }}>
                {sessions
                  .filter((s) => s.kind === "stage_session")
                  .map((s) => (
                    <li key={s.session_id} style={{ border: "1px solid #e5e7eb", borderRadius: 10, padding: 10 }}>
                      <div style={{ display: "flex", justifyContent: "space-between", gap: 8, alignItems: "center" }}>
                        <div style={{ minWidth: 0 }}>
                          <strong>{s.label}</strong>
                          <div className="muted small" style={{ overflow: "hidden", textOverflow: "ellipsis" }}>
                            {s.session_id}
                          </div>
                          <div className="muted small">
                            {sessionHasCookieJar(s) ? "Cookies: ready" : "Cookies: not exported"}
                            {s.last_used_at ? ` • last used ${new Date(s.last_used_at).toLocaleString()}` : ""}
                          </div>
                        </div>
                        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", justifyContent: "flex-end" }}>
                          <button type="button" className="secondary" onClick={() => void handleOpenSession(s.session_id)}>
                            Open
                          </button>
                          <button
                            type="button"
                            className="secondary"
                            onClick={() => void handleExportSessionCookies(s.session_id)}
                            disabled={cookieImportBusy}
                          >
                            {cookieImportBusy ? "Working..." : "Export cookies"}
                          </button>
                        </div>
                      </div>
                    </li>
                  ))}
              </ul>
            )}
          </div>
        </div>
      </div>

      <div style={{ marginTop: 12, border: "1px solid #e5e7eb", borderRadius: 12, padding: 12 }}>
        <h3>Progress</h3>

        {!activeJobId ? (
          <p className="muted">No active Media Downloader job selected yet.</p>
        ) : !activeJob ? (
          <p className="muted">Loading job {activeJobId}…</p>
        ) : (
          <>
            <p className="muted small">
              Job: <code>{activeJob.job_id}</code> • state: <code>{activeJob.state}</code>
            </p>
            {progressText && <p>{progressText}</p>}

            <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginTop: 8 }}>
              <button type="button" className="secondary" onClick={() => void sendControl("pause")}>
                Pause
              </button>
              <button type="button" className="secondary" onClick={() => void sendControl("resume")}>
                Resume
              </button>
              <button type="button" className="secondary" onClick={() => void sendControl("retry_failed")}>
                Retry failed
              </button>
              <button type="button" className="secondary" onClick={() => void sendControl("cancel_all")}>
                Cancel all
              </button>
            </div>

            {!mdOutput ? (
              <p className="muted" style={{ marginTop: 8 }}>
                Waiting for Media Downloader output…
              </p>
            ) : (
              <div style={{ marginTop: 12 }}>
                <p className="muted small">
                  Items: <code>{mdOutput.plan.stable_item_total}</code>
                </p>
                <div style={{ overflowX: "auto" }}>
                  <table className="table" style={{ width: "100%", borderCollapse: "collapse" }}>
                    <thead>
                      <tr>
                        <th style={{ textAlign: "left", padding: 8, borderBottom: "1px solid #e5e7eb" }}>Item</th>
                        <th style={{ textAlign: "left", padding: 8, borderBottom: "1px solid #e5e7eb" }}>URL</th>
                        <th style={{ textAlign: "left", padding: 8, borderBottom: "1px solid #e5e7eb" }}>Status</th>
                        <th style={{ textAlign: "left", padding: 8, borderBottom: "1px solid #e5e7eb" }}>Output</th>
                        <th style={{ padding: 8, borderBottom: "1px solid #e5e7eb" }}></th>
                      </tr>
                    </thead>
                    <tbody>
                      {itemsMerged.map(({ plan, result }) => {
                        const status = result?.status ?? "pending";
                        const errorCode = result?.error_code ?? null;
                        const paths = result?.materialized_paths ?? [];
                        return (
                          <tr key={plan.item_id}>
                            <td style={{ padding: 8, borderBottom: "1px solid #f3f4f6" }}>
                              <code>{plan.item_id}</code>
                            </td>
                            <td style={{ padding: 8, borderBottom: "1px solid #f3f4f6", maxWidth: 420 }}>
                              <span style={{ wordBreak: "break-all" }}>{plan.url_canonical}</span>
                            </td>
                            <td style={{ padding: 8, borderBottom: "1px solid #f3f4f6" }}>
                              <span>
                                {status}
                                {errorCode ? ` (${errorCode})` : ""}
                              </span>
                            </td>
                            <td style={{ padding: 8, borderBottom: "1px solid #f3f4f6", maxWidth: 520 }}>
                              {paths.length === 0 ? (
                                <span className="muted small">—</span>
                              ) : (
                                <div style={{ display: "grid", gap: 4 }}>
                                  {paths.slice(0, 3).map((p) => (
                                    <code key={p} style={{ wordBreak: "break-all" }}>
                                      {p}
                                    </code>
                                  ))}
                                  {paths.length > 3 && <span className="muted small">+{paths.length - 3} more</span>}
                                </div>
                              )}
                            </td>
                            <td style={{ padding: 8, borderBottom: "1px solid #f3f4f6", textAlign: "right" }}>
                              <button
                                type="button"
                                className="secondary"
                                disabled={status !== "running" && status !== "pending"}
                                onClick={() => void sendControl("cancel_one", plan.item_id)}
                              >
                                Cancel
                              </button>
                            </td>
                          </tr>
                        );
                      })}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
