import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REPORT_SCHEMA_ID = "hsk.swarm_n8_perf_evidence@1";
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
    "Usage: node .GOV/roles_shared/checks/hbr-swarm-n8.mjs [--repo-root <path>] [--report <path>] [--wp <wp-id>]",
    "",
    "Runs the MT-035 N=8 swarm perf evidence test and validates the JSONL report.",
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
  return path.join(artifactRoot(repoRoot), "hbr-swarm-n8", `swarm-n8-${timestampSlug()}.jsonl`);
}

function failureRecord(reason, details = {}) {
  return {
    check: "hbr-swarm-n8",
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

function runCargoSmoke(repoRoot, reportPath, wpId) {
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
    "swarm_n8_perf_tests",
    "--",
    "--test-threads=1",
  ], {
    cwd: repoRoot,
    env: {
      ...process.env,
      HANDSHAKE_ACTIVE_REPO_ROOT: repoRoot,
      HANDSHAKE_ARTIFACT_ROOT: artifacts,
      HANDSHAKE_SWARM_N8_REPORT: reportPath,
      HANDSHAKE_SWARM_N8_WP: wpId,
      CARGO_TARGET_DIR: path.join(artifacts, "handshake-cargo-target"),
    },
    encoding: "utf8",
  });
}

function validateRows(rows, reportPath) {
  const row = rows.find((candidate) => candidate.schema_id === REPORT_SCHEMA_ID);
  if (!row) {
    return failureRecord("swarm N=8 report row missing", { report_path: reportPath });
  }
  const required = {
    n: 8,
    mutations_per_session: 100,
    total_mutations: 800,
    sessions_completed: 8,
    silent_overwrites: 0,
    ledger_overflow_count: 0,
  };
  for (const [field, expected] of Object.entries(required)) {
    if (Number(row[field]) !== expected) {
      return failureRecord(`unexpected ${field}`, { report_path: reportPath, expected, actual: row[field], row });
    }
  }
  if (row.deadlock_detected !== false) {
    return failureRecord("swarm N=8 deadlock detected", { report_path: reportPath, row });
  }
  if (Number(row.max_lease_wait_ms || 0) >= 5000) {
    return failureRecord("swarm N=8 lease wait exceeded 5s", { report_path: reportPath, row });
  }
  if (Number(row.conflict_report_count || 0) <= 0 || Number(row.revision_rejection_count || 0) <= 0) {
    return failureRecord("swarm N=8 missing deterministic conflict/revision evidence", { report_path: reportPath, row });
  }
  const eventTypes = new Set(Array.isArray(row.event_ledger_event_types) ? row.event_ledger_event_types : []);
  for (const eventType of ["CRDT_CONFLICT_REPORT", "REVISION_REJECTION"]) {
    if (!eventTypes.has(eventType)) {
      return failureRecord(`swarm N=8 missing ${eventType} row`, { report_path: reportPath, row });
    }
  }
  const hbrIds = new Set(Array.isArray(row.hbr_ids) ? row.hbr_ids : []);
  for (const hbrId of ["HBR-SWARM-001", "HBR-SWARM-002", "HBR-SWARM-003", "HBR-SWARM-004"]) {
    if (!hbrIds.has(hbrId)) {
      return failureRecord(`swarm N=8 missing ${hbrId} evidence tag`, { report_path: reportPath, row });
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
    const result = runCargoSmoke(repoRoot, reportPath, args.wpId || DEFAULT_WP_ID);
    if (result.status !== 0) {
      console.error(JSON.stringify(failureRecord("cargo swarm N=8 test failed", {
        status: result.status,
        error: result.error ? String(result.error.message || result.error) : undefined,
        report_path: reportPath,
        stdout: truncate(result.stdout),
        stderr: truncate(result.stderr),
      })));
      return 2;
    }
    const failure = validateRows(readReportRows(reportPath), reportPath);
    if (failure) {
      console.error(JSON.stringify(failure));
      return 2;
    }
    console.log(`hbr-swarm-n8 ok (report=${reportPath})`);
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
