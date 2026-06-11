import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const DEFAULT_WP_ID = "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";
const EXPECTED_SWARM_BOARD_PROJECTS = new Set([
  "visual-normal-1280-light-en-US",
  "visual-constrained-390-dark-en-US",
  "visual-edge-empty-1024-light-nl-NL",
]);
const CONSTRAINED_SWARM_BOARD_PROJECT = "visual-constrained-390-dark-en-US";

function isNonEmptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function parseArgs(argv) {
  const args = { repoRoot: "", report: "", wpId: DEFAULT_WP_ID };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--repo-root") {
      args.repoRoot = argv[++index] || "";
      continue;
    }
    if (arg === "--report") {
      args.report = argv[++index] || "";
      continue;
    }
    if (arg === "--wp") {
      args.wpId = argv[++index] || "";
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  return args;
}

function scriptRepoRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function artifactRoot(repoRoot) {
  return process.env.HANDSHAKE_ARTIFACT_ROOT
    ? path.resolve(process.env.HANDSHAKE_ARTIFACT_ROOT)
    : path.resolve(repoRoot, "..", "Handshake_Artifacts");
}

function timestampSlug(date = new Date()) {
  return date.toISOString().replace(/[:.]/g, "").replace(/Z$/, "Z");
}

function reportPathFor(repoRoot, injectedReport) {
  if (isNonEmptyString(injectedReport)) return path.resolve(injectedReport);
  return path.join(artifactRoot(repoRoot), "visual-smoke", `hbr-visual-smoke-${timestampSlug()}.jsonl`);
}

function packetPathFor(repoRoot, wpId) {
  return path.join(repoRoot, ".GOV", "task_packets", wpId, "packet.json");
}

function readReportRows(reportPath) {
  if (!fs.existsSync(reportPath)) return [];
  return fs.readFileSync(reportPath, "utf8")
    .trim()
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function summarizeA2Rows(rows) {
  return rows
    .filter((row) => row?.schema_id === "hsk.visual.a2_smoke_report@1")
    .reduce((summary, row) => ({
    scenarios_run: summary.scenarios_run + Number(row.scenarios_run || 0),
    drifts_detected: summary.drifts_detected + Number(row.drifts_detected || 0),
    gaps_detected: summary.gaps_detected + Number(row.gaps_detected || 0),
    console_errors_seen: summary.console_errors_seen + Number(row.console_errors_seen || 0),
    handshake_owned_events: summary.handshake_owned_events
      + Number(row.focus_audit_summary?.handshake_owned_events?.length || 0),
  }), {
    scenarios_run: 0,
    drifts_detected: 0,
    gaps_detected: 0,
    console_errors_seen: 0,
    handshake_owned_events: 0,
  });
}

function swarmBoardRows(rows) {
  return rows.filter((row) => row?.schema_id === "hsk.visual.swarm_board_smoke_report@1");
}

function validateSwarmBoardRows(rows) {
  const invalid = [];
  if (rows.length !== EXPECTED_SWARM_BOARD_PROJECTS.size) {
    invalid.push({ reason: "unexpected row count", expected: EXPECTED_SWARM_BOARD_PROJECTS.size, actual: rows.length });
  }
  const seen = new Set();
  for (const row of rows) {
    const project = String(row?.project_name || "");
    if (!EXPECTED_SWARM_BOARD_PROJECTS.has(project)) {
      invalid.push({ reason: "unexpected project", project });
    } else if (seen.has(project)) {
      invalid.push({ reason: "duplicate project", project });
    }
    seen.add(project);
    if (
      row.geometry_status !== "passed"
      || row.event_delta_status !== "passed"
      || !isNonEmptyString(row.screenshot_path)
      || Number(row.columns_count || 0) < 14
      || Number(row.board_scroll_width || 0) < Number(row.board_client_width || 0)
    ) {
      invalid.push({ reason: "invalid geometry/status fields", row });
    }
  }
  for (const project of EXPECTED_SWARM_BOARD_PROJECTS) {
    if (!seen.has(project)) invalid.push({ reason: "missing expected project", project });
  }
  const constrainedRows = rows.filter((row) => row?.project_name === CONSTRAINED_SWARM_BOARD_PROJECT);
  if (constrainedRows.length !== 1) {
    invalid.push({
      reason: "expected exactly one constrained project row",
      project: CONSTRAINED_SWARM_BOARD_PROJECT,
      actual: constrainedRows.length,
    });
  } else {
    const row = constrainedRows[0];
    if (row.constrained_scroll_verified !== true) {
      invalid.push({ reason: "constrained project did not mark constrained_scroll_verified", row });
    }
    if (Number(row?.viewport?.width || 0) > 390 || Number(row.capture_root_width || 0) > 390) {
      invalid.push({ reason: "constrained project did not prove constrained width", row });
    }
    if (Number(row.board_scroll_width || 0) <= Number(row.board_client_width || 0)) {
      invalid.push({ reason: "constrained project did not prove horizontal overflow", row });
    }
  }
  const nonConstrainedRows = rows.filter((row) => row?.project_name !== CONSTRAINED_SWARM_BOARD_PROJECT);
  for (const row of nonConstrainedRows) {
    if (row.constrained_scroll_verified === true) {
      invalid.push({ reason: "non-constrained project spoofed constrained proof", row });
    }
  }
  return invalid;
}

function failureRecord(reason, details = {}) {
  return {
    check: "hbr-visual-smoke",
    verdict: "FAIL",
    reason,
    ...details,
  };
}

function runPlaywrightSpec(repoRoot, reportPath, wpId, specPath, runIdPrefix) {
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  const artifacts = artifactRoot(repoRoot);
  const env = {
    ...process.env,
    HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
    HANDSHAKE_ARTIFACT_ROOT: artifacts,
    HANDSHAKE_VISUAL_SMOKE_REPORT: reportPath,
    HANDSHAKE_VISUAL_SMOKE_RUN_ID: `${runIdPrefix}-${timestampSlug()}`,
    HANDSHAKE_VISUAL_SMOKE_WP: wpId,
    HANDSHAKE_VISUAL_SMOKE_PACKET: packetPathFor(repoRoot, wpId),
    PLAYWRIGHT_BROWSERS_PATH: process.env.PLAYWRIGHT_BROWSERS_PATH
      || path.join(artifacts, "playwright-browsers"),
  };
  const command = [
    "pnpm", "-C", "app",
    "exec", "playwright", "test",
    "--config", "playwright.config.ts",
    specPath,
  ];
  const executable = process.platform === "win32" ? (process.env.ComSpec || "cmd.exe") : command[0];
  const args = process.platform === "win32" ? ["/d", "/s", "/c", command.join(" ")] : command.slice(1);
  return spawnSync(executable, args, {
    cwd: repoRoot,
    env,
    encoding: "utf8",
  });
}

export function runCli(argv = process.argv.slice(2)) {
  try {
    const args = parseArgs(argv);
    const repoRoot = path.resolve(args.repoRoot || process.env.HANDSHAKE_ACTIVE_REPO_ROOT || scriptRepoRoot());
    const wpId = args.wpId || DEFAULT_WP_ID;
    const reportPath = reportPathFor(repoRoot, args.report);
    if (fs.existsSync(reportPath)) fs.rmSync(reportPath, { force: true });

    const result = runPlaywrightSpec(repoRoot, reportPath, wpId, "tests/visual/a2_smoke.spec.ts", "a2-smoke");
    const rows = readReportRows(reportPath);
    const summary = summarizeA2Rows(rows);

    if (result.status !== 0) {
      console.error(JSON.stringify(failureRecord("playwright smoke command failed", {
        status: result.status,
        error: result.error ? String(result.error.message || result.error) : undefined,
        report_path: reportPath,
        stdout: result.stdout,
        stderr: result.stderr,
      })));
      return 2;
    }
    if (summary.scenarios_run < 3) {
      console.error(JSON.stringify(failureRecord("visual smoke did not run all three matrix scenarios", {
        report_path: reportPath,
        summary,
      })));
      return 2;
    }
    for (const [field, value] of Object.entries({
      drifts_detected: summary.drifts_detected,
      gaps_detected: summary.gaps_detected,
      console_errors_seen: summary.console_errors_seen,
      handshake_owned_events: summary.handshake_owned_events,
    })) {
      if (value !== 0) {
        console.error(JSON.stringify(failureRecord(`visual smoke reported non-zero ${field}`, {
          report_path: reportPath,
          summary,
        })));
        return 2;
      }
    }

    const boardResult = runPlaywrightSpec(
      repoRoot,
      reportPath,
      wpId,
      "tests/visual/swarm-board.spec.ts",
      "swarm-board-smoke",
    );
    if (boardResult.status !== 0) {
      console.error(JSON.stringify(failureRecord("SwarmBoard visual smoke command failed", {
        status: boardResult.status,
        error: boardResult.error ? String(boardResult.error.message || boardResult.error) : undefined,
        report_path: reportPath,
        stdout: boardResult.stdout,
        stderr: boardResult.stderr,
      })));
      return 2;
    }
    const boardRows = swarmBoardRows(readReportRows(reportPath));
    if (boardRows.length < EXPECTED_SWARM_BOARD_PROJECTS.size) {
      console.error(JSON.stringify(failureRecord("SwarmBoard visual smoke did not run all three matrix scenarios", {
        report_path: reportPath,
        board_rows: boardRows.length,
        stdout: boardResult.stdout,
        stderr: boardResult.stderr,
      })));
      return 2;
    }
    const invalidBoardRows = validateSwarmBoardRows(boardRows);
    if (invalidBoardRows.length > 0) {
      console.error(JSON.stringify(failureRecord("SwarmBoard visual smoke reported invalid structured row(s)", {
        report_path: reportPath,
        invalid_rows: invalidBoardRows,
      })));
      return 2;
    }

    console.log(`hbr-visual-smoke ok (${summary.scenarios_run} A.2 scenario(s), ${boardRows.length} SwarmBoard structured scenario(s), report=${reportPath})`);
    return 0;
  } catch (error) {
    console.error(JSON.stringify(failureRecord(error instanceof Error ? error.message : String(error))));
    return 3;
  }
}

function isInvokedAsMain() {
  if (!process.argv[1]) return false;
  return fs.realpathSync.native(path.resolve(process.argv[1]))
    === fs.realpathSync.native(fileURLToPath(import.meta.url));
}

if (isInvokedAsMain()) {
  process.exitCode = runCli();
}
