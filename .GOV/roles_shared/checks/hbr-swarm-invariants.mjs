import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPORT_SCHEMA_ID = "hsk.swarm_invariants_evidence@1";
const FAILURE_RECEIPT_KIND = "HBR_SWARM_INVARIANT_FAIL";
const DEFAULT_WP_ID = "WP-KERNEL-004-Local-Model-Boxing-Inference-Lab-Sandbox-Memory-V1-HBR-Enforcement-v1";

function parseArgs(argv) {
  const args = { repoRoot: "", report: "", wpId: DEFAULT_WP_ID, help: false };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      continue;
    }
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

function usage() {
  return [
    "Usage: node .GOV/roles_shared/checks/hbr-swarm-invariants.mjs [--repo-root <path>] [--report <path>] [--wp <wp-id>]",
    "",
    "Runs the MT-037 swarm invariant evidence test and validates the JSONL report.",
  ].join("\n");
}

function scriptRepoRoot() {
  return path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../../..");
}

function artifactRoot(repoRoot) {
  return process.env.HANDSHAKE_ARTIFACT_ROOT
    ? path.resolve(process.env.HANDSHAKE_ARTIFACT_ROOT)
    : path.resolve(repoRoot, "..", "..", "Handshake_Artifacts");
}

function timestampSlug(date = new Date()) {
  return date.toISOString().replace(/[:.]/g, "").replace(/Z$/, "Z");
}

function reportPathFor(repoRoot, injectedReport) {
  if (injectedReport && injectedReport.trim()) return path.resolve(injectedReport);
  return path.join(artifactRoot(repoRoot), "hbr-swarm-invariants", `swarm-invariants-${timestampSlug()}.jsonl`);
}

function failureRecord(reason, details = {}) {
  return {
    check: "hbr-swarm-invariants",
    verdict: "FAIL",
    reason,
    ...details,
  };
}

function truncate(value, limit = 8000) {
  const text = String(value || "");
  if (text.length <= limit) return text;
  return `${text.slice(0, limit)}...<truncated>`;
}

function readReportRows(reportPath) {
  if (!fs.existsSync(reportPath)) return [];
  return fs.readFileSync(reportPath, "utf8")
    .trim()
    .split(/\r?\n/)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function runCargoSmoke(repoRoot, reportPath) {
  const artifacts = artifactRoot(repoRoot);
  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  fs.mkdirSync(artifacts, { recursive: true });
  if (fs.existsSync(reportPath)) fs.rmSync(reportPath, { force: true });

  return spawnSync("cargo", [
    "test",
    "--manifest-path",
    path.join(repoRoot, "src/backend/handshake_core/Cargo.toml"),
    "-p",
    "handshake_core",
    "--target-dir",
    path.join(artifacts, "handshake-cargo-target"),
    "--test",
    "swarm_invariants_tests",
    "--",
    "--test-threads=1",
  ], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
      HANDSHAKE_ARTIFACT_ROOT: artifacts,
      HANDSHAKE_SWARM_INVARIANTS_REPORT: reportPath,
      CARGO_TARGET_DIR: path.join(artifacts, "handshake-cargo-target"),
    },
    encoding: "utf8",
  });
}

function validateNumber(row, field, actual, expected, reportPath) {
  if (Number(actual) !== expected) {
    return failureRecord(`unexpected ${field}`, { report_path: reportPath, field, expected, actual, row });
  }
  return null;
}

function validateRows(rows, reportPath, wpId) {
  const failureReceipt = rows.find((row) => row.receipt_kind === FAILURE_RECEIPT_KIND);
  if (failureReceipt) {
    return failureRecord("swarm invariant failure receipt emitted", { report_path: reportPath, failure_receipt: failureReceipt });
  }

  const row = rows.find((candidate) => candidate.schema_id === REPORT_SCHEMA_ID);
  if (!row) {
    return failureRecord("swarm invariant report row missing", { report_path: reportPath });
  }
  if (row.wp_id !== wpId) {
    return failureRecord("unexpected wp_id", { report_path: reportPath, expected: wpId, actual: row.wp_id, row });
  }

  const required = [
    ["lock_lease.sessions", row.lock_lease?.sessions, 16],
    ["lock_lease.grants_completed", row.lock_lease?.grants_completed, 16],
    ["lock_lease.unique_grants", row.lock_lease?.unique_grants, 16],
    ["lock_lease.max_simultaneous_holders", row.lock_lease?.max_simultaneous_holders, 1],
    ["cancellation.sessions", row.cancellation?.sessions, 8],
    ["cancellation.cancelled_sessions", row.cancellation?.cancelled_sessions, 8],
    ["loop_counter.cap", row.loop_counter?.cap, 1000],
    ["loop_counter.iterations", row.loop_counter?.iterations, 1000],
    ["process_ledger.sessions", row.process_ledger?.sessions, 8],
    ["process_ledger.processes_per_session", row.process_ledger?.processes_per_session, 10],
    ["process_ledger.start_rows", row.process_ledger?.start_rows, 80],
    ["process_ledger.stop_rows", row.process_ledger?.stop_rows, 80],
    ["process_ledger.duplicate_process_uuid_count", row.process_ledger?.duplicate_process_uuid_count, 0],
    ["process_ledger.missing_stop_count", row.process_ledger?.missing_stop_count, 0],
    ["process_ledger.wrong_session_correlation_count", row.process_ledger?.wrong_session_correlation_count, 0],
    ["process_ledger.ledger_overflow_count", row.process_ledger?.ledger_overflow_count, 0],
  ];
  for (const [field, actual, expected] of required) {
    const failure = validateNumber(row, field, actual, expected, reportPath);
    if (failure) return failure;
  }
  if (Number(row.cancellation?.max_propagation_ms || 0) > 500) {
    return failureRecord("cancellation propagation exceeded 500ms", { report_path: reportPath, row });
  }
  if (row.loop_counter?.receipt_emitted !== true || row.loop_counter?.terminated !== true) {
    return failureRecord("loop counter did not emit and terminate", { report_path: reportPath, row });
  }
  if (row.loop_counter?.event_type !== "FR-EVT-LOOP-CAP") {
    return failureRecord("loop counter emitted wrong event type", { report_path: reportPath, row });
  }
  if (row.failure_receipt_kind !== FAILURE_RECEIPT_KIND) {
    return failureRecord("failure receipt kind not documented in report", { report_path: reportPath, row });
  }
  const hbrIds = new Set(Array.isArray(row.hbr_ids) ? row.hbr_ids : []);
  for (const hbrId of ["HBR-SWARM-001", "HBR-SWARM-002", "HBR-SWARM-003", "HBR-SWARM-004", "HBR-QUIET-003"]) {
    if (!hbrIds.has(hbrId)) {
      return failureRecord(`swarm invariant report missing ${hbrId}`, { report_path: reportPath, row });
    }
  }
  return null;
}

export function runCli(argv = process.argv.slice(2)) {
  try {
    const args = parseArgs(argv);
    if (args.help) {
      console.log(usage());
      return 0;
    }
    const repoRoot = path.resolve(args.repoRoot || process.env.HANDSHAKE_ACTIVE_REPO_ROOT || scriptRepoRoot());
    const reportPath = reportPathFor(repoRoot, args.report);
    const wpId = args.wpId || DEFAULT_WP_ID;
    const result = runCargoSmoke(repoRoot, reportPath);
    if (result.status !== 0) {
      console.error(JSON.stringify(failureRecord("cargo swarm invariant test failed", {
        status: result.status,
        error: result.error ? String(result.error.message || result.error) : undefined,
        report_path: reportPath,
        stdout: truncate(result.stdout),
        stderr: truncate(result.stderr),
      })));
      return 2;
    }
    const failure = validateRows(readReportRows(reportPath), reportPath, wpId);
    if (failure) {
      console.error(JSON.stringify(failure));
      return 2;
    }
    console.log(`hbr-swarm-invariants ok (report=${reportPath})`);
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
