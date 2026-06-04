import { spawnSync } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";

import { expect, test } from "./console_error_scan";

import {
  captureAndCompareEntry,
  captureMatrixEntryFromProject,
} from "./capture_matrix";

import {
  withFocusAudit,
  type FocusAuditEvent,
  type FocusAuditOutcome,
} from "./focus_audit_driver";

type SmokeReportRow = {
  schema_id: "hsk.visual.a2_smoke_report@1";
  report_version: 1;
  run_id: string;
  project_name: string;
  scenario_id: string;
  scenarios_run: 1;
  capture_status: "passed" | "bootstrapped" | "drift";
  drifts_detected: number;
  gaps_detected: number;
  console_errors_seen: number;
  dom_snapshot_root: string | null;
  focus_audit_summary: {
    run_id: string;
    // MT-027: this is now the REAL FocusAuditReport.handshake_owned_events,
    // driven by the live focus-audit hook (Tauri IPC or the focus-audit-probe
    // binary), not a hardcoded empty array.
    handshake_owned_events: FocusAuditEvent[];
    audit_source: "tauri_ipc" | "probe_binary";
    audit_status: "audited" | "unsupported_platform";
    total_foreground_events: number | null;
  };
  gap_receipts: string[];
  total_duration_ms: number;
};

const DEFAULT_WP_ID = "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";

function repoRoot(): string {
  return path.resolve(__dirname, "..", "..");
}

function focusAuditRuntimeRoot(testInfo: import("playwright").TestInfo): string {
  // Land the real focus-audit JSONL evidence inside the test artifact tree so
  // the audit is disk-agnostic and does not pollute the repo. The ledger is
  // written under <root>/gov_runtime/focus_audit/<run_id>.jsonl by the core.
  return process.env.HANDSHAKE_FOCUS_AUDIT_RUNTIME_ROOT
    ?? testInfo.outputPath("focus-audit-runtime");
}

function smokeRunId(): string {
  return process.env.HANDSHAKE_VISUAL_SMOKE_RUN_ID || `a2-smoke-${Date.now()}`;
}

function smokeReportPath(): string | null {
  return process.env.HANDSHAKE_VISUAL_SMOKE_REPORT || null;
}

async function appendSmokeReport(row: SmokeReportRow): Promise<void> {
  const reportPath = smokeReportPath();
  if (!reportPath) return;
  await fs.mkdir(path.dirname(reportPath), { recursive: true });
  await fs.appendFile(reportPath, `${JSON.stringify(row)}\n`, "utf8");
}

async function cdpDomSnapshotRoot(page: import("playwright").Page): Promise<string | null> {
  const session = await page.context().newCDPSession(page);
  try {
    await session.send("DOM.enable");
    const snapshot = await session.send("DOM.getDocument", { depth: 2, pierce: true });
    return typeof snapshot?.root?.nodeName === "string" ? snapshot.root.nodeName : null;
  } finally {
    await session.detach().catch(() => undefined);
  }
}

function emitVisGapReceipt(gap: {
  surfaceName: string;
  surfacePath: string;
  gapClass: string;
  evidencePointer?: string;
}): string | null {
  const packetPath = process.env.HANDSHAKE_VISUAL_SMOKE_PACKET;
  if (!packetPath) return null;
  const wpId = process.env.HANDSHAKE_VISUAL_SMOKE_WP || DEFAULT_WP_ID;
  const scriptPath = path.join(repoRoot(), ".GOV", "roles_shared", "scripts", "hbr-vis-gap-emit.mjs");
  const result = spawnSync(process.execPath, [
    scriptPath,
    "--wp", wpId,
    "--surface", gap.surfaceName,
    "--surface-path", gap.surfacePath,
    "--gap-class", gap.gapClass,
    "--packet", packetPath,
    ...(gap.evidencePointer ? ["--evidence-pointer", gap.evidencePointer] : []),
  ], {
    cwd: repoRoot(),
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`HBR_VIS_GAP emission failed:\n${result.stdout}\n${result.stderr}`);
  }
  const summary = JSON.parse(result.stdout.trim());
  return `receipt://${summary.receipt_uuid}`;
}

test("A.2 visual debugger stack smoke", async ({ page }, testInfo) => {
  const startedAt = Date.now();
  const runId = smokeRunId();
  const entry = captureMatrixEntryFromProject(testInfo);

  // MT-027: drive the REAL foreground focus audit around the visual capture.
  // The hook is open for the entire capture/gap/DOM-snapshot scenario; HBR-QUIET-001
  // requires that no Handshake-owned window stole the foreground during it.
  let captureResult: Awaited<ReturnType<typeof captureAndCompareEntry>> | null = null;
  let gaps: Array<{
    surfaceName: string;
    surfacePath: string;
    gapClass: string;
    evidencePointer?: string;
  }> = [];
  let gapReceipts: string[] = [];
  let domSnapshotRoot: string | null = null;

  const auditOutcome: FocusAuditOutcome = await withFocusAudit(
    page,
    runId,
    focusAuditRuntimeRoot(testInfo),
    async () => {
      captureResult = await captureAndCompareEntry(page, entry, {
        baselineRoot: testInfo.outputPath("baselines"),
        receiptRoot: testInfo.outputPath("receipts"),
        allowBootstrap: true,
        projectName: testInfo.project.name,
      });

      const surfaceCount = await page.locator(entry.wait_for).count();
      gaps = surfaceCount === 0
        ? [{
          surfaceName: `visual smoke ${entry.scenario_id}`,
          surfacePath: entry.wait_for,
          gapClass: "no_cdp_handle",
          evidencePointer: captureResult.receipt_path ?? undefined,
        }]
        : [];
      gapReceipts = gaps
        .map((gap) => emitVisGapReceipt(gap))
        .filter((receipt): receipt is string => Boolean(receipt));
      domSnapshotRoot = await cdpDomSnapshotRoot(page);
    },
  );

  if (!captureResult) {
    throw new Error("visual capture scenario did not run inside the focus audit");
  }
  const result = captureResult as Awaited<ReturnType<typeof captureAndCompareEntry>>;

  const ownedEvents = auditOutcome.kind === "report"
    ? auditOutcome.report.handshake_owned_events
    : [];
  const row: SmokeReportRow = {
    schema_id: "hsk.visual.a2_smoke_report@1",
    report_version: 1,
    run_id: runId,
    project_name: testInfo.project.name,
    scenario_id: entry.scenario_id,
    scenarios_run: 1,
    capture_status: result.status,
    drifts_detected: result.status === "drift" ? 1 : 0,
    gaps_detected: gaps.length,
    console_errors_seen: 0,
    dom_snapshot_root: domSnapshotRoot,
    focus_audit_summary: {
      run_id: auditOutcome.kind === "report" ? auditOutcome.report.run_id : runId,
      handshake_owned_events: ownedEvents,
      audit_source: auditOutcome.source,
      audit_status: auditOutcome.kind === "report" ? "audited" : "unsupported_platform",
      total_foreground_events: auditOutcome.kind === "report"
        ? auditOutcome.report.total_events
        : null,
    },
    gap_receipts: gapReceipts,
    total_duration_ms: Date.now() - startedAt,
  };
  await appendSmokeReport(row);

  expect(row.drifts_detected).toBe(0);
  expect(row.gaps_detected).toBe(0);
  expect(row.console_errors_seen).toBe(0);
  expect(row.dom_snapshot_root).toBeTruthy();

  if (auditOutcome.kind === "report") {
    // HBR-QUIET-001: the REAL audit must show no Handshake-owned foreground
    // event during the captured run. This is no longer a tautology — the
    // vector is the genuine FocusAuditReport.handshake_owned_events.
    expect(row.focus_audit_summary.handshake_owned_events).toEqual([]);
  } else {
    // The Win32 foreground hook is unavailable on this host (e.g. the Linux dev
    // lane). Fail honestly rather than asserting an empty array we never
    // measured: require the audit to report the real unsupported-platform
    // status, and skip the QUIET assertion on a host that cannot enforce it.
    expect(row.focus_audit_summary.audit_status).toBe("unsupported_platform");
    test.skip(
      true,
      `focus audit unavailable on ${process.platform}: ${
        auditOutcome.kind === "unsupported_platform" ? auditOutcome.detail : ""
      }`,
    );
  }
});
